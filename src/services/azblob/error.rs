// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::Debug;

use bytes::Buf;
use http::Response;
use http::StatusCode;
use quick_xml::de;
use serde::Deserialize;

use crate::raw::*;
use crate::Error;
use crate::ErrorKind;
use crate::Result;

/// AzblobError is the error returned by azure blob service.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct AzblobError {
    code: String,
    message: String,
    query_parameter_name: String,
    query_parameter_value: String,
    reason: String,
}

impl Debug for AzblobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut de = f.debug_struct("AzblobError");
        de.field("code", &self.code);
        // replace `\n` to ` ` for better reading.
        de.field("message", &self.message.replace('\n', " "));

        if !self.query_parameter_name.is_empty() {
            de.field("query_parameter_name", &self.query_parameter_name);
        }
        if !self.query_parameter_value.is_empty() {
            de.field("query_parameter_value", &self.query_parameter_value);
        }
        if !self.reason.is_empty() {
            de.field("reason", &self.reason);
        }

        de.finish()
    }
}

/// Parse error respons into Error.
pub async fn parse_error(resp: Response<IncomingAsyncBody>) -> Result<Error> {
    let (parts, body) = resp.into_parts();
    let bs = body.bytes().await?;

    let (kind, retryable) = match parts.status {
        StatusCode::NOT_FOUND => (ErrorKind::ObjectNotFound, false),
        StatusCode::FORBIDDEN => (ErrorKind::ObjectPermissionDenied, false),
        StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => (ErrorKind::Unexpected, true),
        _ => (ErrorKind::Unexpected, false),
    };

    let mut message = match de::from_reader::<_, AzblobError>(bs.clone().reader()) {
        Ok(azblob_err) => format!("{azblob_err:?}"),
        Err(_) => String::from_utf8_lossy(&bs).into_owned(),
    };
    // If there is no body here, fill with error code.
    if message.is_empty() {
        if let Some(v) = parts.headers.get("x-ms-error-code") {
            if let Ok(code) = v.to_str() {
                message = format!(
                    "{:?}",
                    AzblobError {
                        code: code.to_string(),
                        ..Default::default()
                    }
                )
            }
        }
    }

    let mut err = Error::new(kind, &message).with_context("response", format!("{parts:?}"));

    if retryable {
        err = err.set_temporary();
    }

    Ok(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error() {
        let bs = bytes::Bytes::from(
            r#"
<?xml version="1.0" encoding="utf-8"?>
<Error>
  <Code>string-value</Code>
  <Message>string-value</Message>
</Error>
"#,
        );

        let out: AzblobError = de::from_reader(bs.reader()).expect("must success");
        println!("{out:?}");

        assert_eq!(out.code, "string-value");
        assert_eq!(out.message, "string-value");
    }

    #[test]
    fn test_parse_error_with_reason() {
        let bs = bytes::Bytes::from(
            r#"
<?xml version="1.0" encoding="utf-8"?>
<Error>
  <Code>InvalidQueryParameterValue</Code>
  <Message>Value for one of the query parameters specified in the request URI is invalid.</Message>
  <QueryParameterName>popreceipt</QueryParameterName>
  <QueryParameterValue>33537277-6a52-4a2b-b4eb-0f905051827b</QueryParameterValue>
  <Reason>invalid receipt format</Reason>
</Error>
"#,
        );

        let out: AzblobError = de::from_reader(bs.reader()).expect("must success");
        println!("{out:?}");

        assert_eq!(out.code, "InvalidQueryParameterValue");
        assert_eq!(
            out.message,
            "Value for one of the query parameters specified in the request URI is invalid."
        );
        assert_eq!(out.query_parameter_name, "popreceipt");
        assert_eq!(
            out.query_parameter_value,
            "33537277-6a52-4a2b-b4eb-0f905051827b"
        );
        assert_eq!(out.reason, "invalid receipt format");
    }
}
