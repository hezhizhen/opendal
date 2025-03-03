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

use http::Response;
use http::StatusCode;
use serde::Deserialize;
use serde_json::de;

use crate::raw::*;
use crate::Error;
use crate::ErrorKind;
use crate::Result;

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
struct IpfsError {
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "Code")]
    code: usize,
    #[serde(rename = "Type")]
    ty: String,
}

/// Parse error response into io::Error.
///
/// > Status code 500 means that the function does exist, but IPFS was not
/// > able to fulfil the request because of an error.
/// > To know that reason, you have to look at the error message that is
/// > usually returned with the body of the response
/// > (if no error, check the daemon logs).
///
/// ref: https://docs.ipfs.tech/reference/kubo/rpc/#http-status-codes
pub async fn parse_error(resp: Response<IncomingAsyncBody>) -> Result<Error> {
    let (parts, body) = resp.into_parts();
    let bs = body.bytes().await?;

    let ipfs_error = de::from_slice::<IpfsError>(&bs).ok();

    let (kind, retryable) = match parts.status {
        StatusCode::INTERNAL_SERVER_ERROR => {
            if let Some(ie) = &ipfs_error {
                match ie.message.as_str() {
                    "file does not exist" => (ErrorKind::ObjectNotFound, false),
                    _ => (ErrorKind::Unexpected, false),
                }
            } else {
                (ErrorKind::Unexpected, false)
            }
        }
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
            (ErrorKind::Unexpected, true)
        }
        _ => (ErrorKind::Unexpected, false),
    };

    let message = match ipfs_error {
        Some(ipfs_error) => format!("{ipfs_error:?}"),
        None => String::from_utf8_lossy(&bs).into_owned(),
    };

    let mut err = Error::new(kind, &message).with_context("response", format!("{parts:?}"));

    if retryable {
        err = err.set_temporary();
    }

    Ok(err)
}

pub fn parse_json_deserialize_error(e: serde_json::Error) -> Error {
    Error::new(ErrorKind::Unexpected, "deserialize json").set_source(e)
}
