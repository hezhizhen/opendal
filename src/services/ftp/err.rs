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

use suppaftp::FtpError;
use suppaftp::Status;

use crate::Error;
use crate::ErrorKind;

impl From<FtpError> for Error {
    fn from(e: FtpError) -> Self {
        let (kind, retryable) = match e {
            // Allow retry for error
            //
            // `{ status: NotAvailable, body: "421 There are too many connections from your internet address." }`
            FtpError::UnexpectedResponse(ref resp) if resp.status == Status::NotAvailable => {
                (ErrorKind::Unexpected, true)
            }
            FtpError::UnexpectedResponse(ref resp) if resp.status == Status::FileUnavailable => {
                (ErrorKind::ObjectNotFound, false)
            }
            // Allow retry bad response.
            FtpError::BadResponse => (ErrorKind::Unexpected, true),
            _ => (ErrorKind::Unexpected, false),
        };

        let mut err = Error::new(kind, "ftp error").set_source(e);

        if retryable {
            err = err.set_temporary();
        }

        err
    }
}
