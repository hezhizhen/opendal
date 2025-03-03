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

use crate::raw::*;
use crate::Error;
use crate::ErrorKind;
use crate::Result;

/// Parse error respons into Error.
pub async fn parse_error(resp: Response<IncomingAsyncBody>) -> Result<Error> {
    let (parts, body) = resp.into_parts();

    let (kind, retryable) = match parts.status {
        StatusCode::NOT_FOUND | StatusCode::NO_CONTENT => (ErrorKind::ObjectNotFound, false),
        StatusCode::CONFLICT => (ErrorKind::ObjectAlreadyExists, false),
        StatusCode::FORBIDDEN => (ErrorKind::ObjectPermissionDenied, false),
        StatusCode::TOO_MANY_REQUESTS => (ErrorKind::ObjectRateLimited, true),
        StatusCode::INTERNAL_SERVER_ERROR
        | StatusCode::BAD_GATEWAY
        | StatusCode::SERVICE_UNAVAILABLE
        | StatusCode::GATEWAY_TIMEOUT => (ErrorKind::Unexpected, true),
        _ => (ErrorKind::Unexpected, false),
    };

    let bs = body.bytes().await?;
    let mut err = Error::new(kind, &String::from_utf8_lossy(&bs))
        .with_context("response", format!("{parts:?}"));

    if retryable {
        err = err.set_temporary();
    }

    Ok(err)
}
