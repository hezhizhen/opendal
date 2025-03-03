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

//! Errors that returned by OpenDAL
//!
//! # Examples
//!
//! ```
//! # use anyhow::Result;
//! # use opendal::ObjectMode;
//! # use opendal::Operator;
//! use opendal::ErrorKind;
//! # #[tokio::main]
//! # async fn test(op: Operator) -> Result<()> {
//! if let Err(e) = op.object("test_file").metadata().await {
//!     if e.kind() == ErrorKind::ObjectNotFound {
//!         println!("object not exist")
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io;

/// Result that is a wrapper of `Reustl<T, opendal::Error>`
pub type Result<T> = std::result::Result<T, Error>;

/// ErrorKind is all kinds of opendal's Error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// OpenDAL don't know what happened here, and no actions other than just
    /// returning it back. For example, s3 returns an internal servie error.
    Unexpected,
    /// Underlying service doesn't support this operation.
    Unsupported,

    /// The config for backend is invalid.
    BackendConfigInvalid,

    /// Object is not found.
    ObjectNotFound,
    /// Object doesn't have enough permission for this operation
    ObjectPermissionDenied,
    /// Object is a directory.
    ObjectIsADirectory,
    /// Object is not a directory.
    ObjectNotADirectory,
    /// Object already exists thus we failed to the specified operation on it.
    ObjectAlreadyExists,
    /// Requests that sent to this object is over the limit, please slow down.
    ObjectRateLimited,
}

impl ErrorKind {
    /// Convert self into static str.
    pub fn into_static(self) -> &'static str {
        self.into()
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_static())
    }
}

impl From<ErrorKind> for &'static str {
    fn from(v: ErrorKind) -> &'static str {
        match v {
            ErrorKind::Unexpected => "Unexpected",
            ErrorKind::Unsupported => "Unsupported",
            ErrorKind::BackendConfigInvalid => "BackendConfigInvalid",
            ErrorKind::ObjectNotFound => "ObjectNotFound",
            ErrorKind::ObjectPermissionDenied => "ObjectPermissionDenied",
            ErrorKind::ObjectIsADirectory => "ObjectIsADirectory",
            ErrorKind::ObjectNotADirectory => "ObjectNotADirectory",
            ErrorKind::ObjectAlreadyExists => "ObjectAlreadyExists",
            ErrorKind::ObjectRateLimited => "ObjectRateLimited",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ErrorStatus {
    /// Permenent means without external changes, the error never changes.
    ///
    /// For example, underlying services returns a not found error.
    ///
    /// Users SHOULD never retry this operation.
    Permanent,
    /// Temporary means this error is returned for temporary.
    ///
    /// For example, underlying services is rate limited or unailable for temporary.
    ///
    /// Users CAN retry the operation to resolve it.
    Temporary,
    /// Persistent means this error used to be temporary but still failed after retry.
    ///
    /// For example, underlying services kept returning network errors.
    ///
    /// Users MAY retry this opration but it's highly possible to error again.
    Persistent,
}

impl Display for ErrorStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ErrorStatus::Permanent => write!(f, "permanent"),
            ErrorStatus::Temporary => write!(f, "temporary"),
            ErrorStatus::Persistent => write!(f, "persistent"),
        }
    }
}

/// Error is the error struct returned by all opendal functions.
pub struct Error {
    kind: ErrorKind,
    message: String,

    status: ErrorStatus,
    operation: &'static str,
    context: Vec<(&'static str, String)>,
    source: Option<anyhow::Error>,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}) at {}", self.kind, self.status, self.operation)?;

        if !self.context.is_empty() {
            write!(f, ", context: {{ ")?;
            write!(
                f,
                "{}",
                self.context
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
            write!(f, " }}")?;
        }

        if !self.message.is_empty() {
            write!(f, " => {}", self.message)?;
        }

        if let Some(source) = &self.source {
            write!(f, ", source: {source}")?;
        }

        Ok(())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // If alternate has been specified, we will print like Debug.
        if f.alternate() {
            let mut de = f.debug_struct("Error");
            de.field("kind", &self.kind);
            de.field("message", &self.message);
            de.field("status", &self.status);
            de.field("operation", &self.operation);
            de.field("context", &self.context);
            de.field("source", &self.source);
            return de.finish();
        }

        write!(f, "{} ({}) at {}", self.kind, self.status, self.operation)?;
        if !self.message.is_empty() {
            write!(f, " => {}", self.message)?;
        }
        writeln!(f)?;

        if !self.context.is_empty() {
            writeln!(f)?;
            writeln!(f, "Context:")?;
            for (k, v) in self.context.iter() {
                writeln!(f, "    {k}: {v}")?;
            }
        }
        if let Some(source) = &self.source {
            writeln!(f)?;
            writeln!(f, "Source: {source:?}")?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|v| v.as_ref())
    }
}

impl Error {
    /// Create a new Error with error kind and message.
    pub fn new(kind: ErrorKind, message: &str) -> Self {
        Self {
            kind,
            message: message.to_string(),

            status: ErrorStatus::Permanent,
            operation: "",
            context: Vec::default(),
            source: None,
        }
    }

    /// Update error's operation.
    ///
    /// # Notes
    ///
    /// If the error already carries an operation, we will push a new context
    /// `(called, operation)`.
    pub fn with_operation(mut self, operation: &'static str) -> Self {
        if !self.operation.is_empty() {
            self.context.push(("called", self.operation.to_string()));
        }

        self.operation = operation;
        self
    }

    /// Add more context in error.
    pub fn with_context(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.context.push((key, value.into()));
        self
    }

    /// Set source for error.
    ///
    /// # Notes
    ///
    /// If the source has been set, we will raise a panic here.
    pub fn set_source(mut self, src: impl Into<anyhow::Error>) -> Self {
        debug_assert!(self.source.is_none(), "the source error has been set");

        self.source = Some(src.into());
        self
    }

    /// Operate on error with map.
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    /// Set permenent status for error.
    pub fn set_permanent(mut self) -> Self {
        self.status = ErrorStatus::Permanent;
        self
    }

    /// Set temporary status for error.
    ///
    /// By set temporary, we indicate this error is retryable.
    pub fn set_temporary(mut self) -> Self {
        self.status = ErrorStatus::Temporary;
        self
    }

    /// Set perisistent status for error.
    ///
    /// By setting persistent, we indicate the retry should be stopped.
    pub fn set_persistent(mut self) -> Self {
        self.status = ErrorStatus::Persistent;
        self
    }

    /// Return error's kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Check if this error is temporary.
    pub fn is_temporary(&self) -> bool {
        self.status == ErrorStatus::Temporary
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        let kind = match err.kind() {
            ErrorKind::ObjectNotFound => io::ErrorKind::NotFound,
            ErrorKind::ObjectPermissionDenied => io::ErrorKind::PermissionDenied,
            _ => io::ErrorKind::Other,
        };

        io::Error::new(kind, err)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use once_cell::sync::Lazy;

    use super::*;

    static TEST_ERROR: Lazy<Error> = Lazy::new(|| Error {
        kind: ErrorKind::Unexpected,
        message: "something wrong happened".to_string(),
        status: ErrorStatus::Permanent,
        operation: "Read",
        context: vec![
            ("path", "/path/to/file".to_string()),
            ("called", "send_async".to_string()),
        ],
        source: Some(anyhow!("networking error")),
    });

    #[test]
    fn test_error_display() {
        let s = format!("{}", Lazy::force(&TEST_ERROR));
        assert_eq!(
            s,
            r#"Unexpected (permanent) at Read, context: { path: /path/to/file, called: send_async } => something wrong happened, source: networking error"#
        )
    }

    #[test]
    fn test_error_debug() {
        let s = format!("{:?}", Lazy::force(&TEST_ERROR));
        assert_eq!(
            s,
            r#"Unexpected (permanent) at Read => something wrong happened

Context:
    path: /path/to/file
    called: send_async

Source: networking error
"#
        )
    }
}
