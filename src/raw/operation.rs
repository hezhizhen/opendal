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

use std::fmt::Display;
use std::fmt::Formatter;

/// Operation is the name for APIs in `Accessor`.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    /// Operation for [`crate::raw::Accessor::metadata`]
    Metadata,
    /// Operation for [`crate::raw::Accessor::create`]
    Create,
    /// Operation for [`crate::raw::Accessor::read`]
    Read,
    /// Operation for [`crate::raw::Accessor::write`]
    Write,
    /// Operation for [`crate::raw::Accessor::stat`]
    Stat,
    /// Operation for [`crate::raw::Accessor::delete`]
    Delete,
    /// Operation for [`crate::raw::Accessor::list`]
    List,
    /// Operation for [`crate::raw::Accessor::presign`]
    Presign,
    /// Operation for [`crate::raw::Accessor::create_multipart`]
    CreateMultipart,
    /// Operation for [`crate::raw::Accessor::write_multipart`]
    WriteMultipart,
    /// Operation for [`crate::raw::Accessor::complete_multipart`]
    CompleteMultipart,
    /// Operation for [`crate::raw::Accessor::abort_multipart`]
    AbortMultipart,
    /// Operation for [`crate::raw::Accessor::blocking_create`]
    BlockingCreate,
    /// Operation for [`crate::raw::Accessor::blocking_read`]
    BlockingRead,
    /// Operation for [`crate::raw::Accessor::blocking_write`]
    BlockingWrite,
    /// Operation for [`crate::raw::Accessor::blocking_stat`]
    BlockingStat,
    /// Operation for [`crate::raw::Accessor::blocking_delete`]
    BlockingDelete,
    /// Operation for [`crate::raw::Accessor::blocking_list`]
    BlockingList,
}

impl Operation {
    /// Convert self into static str.
    pub fn into_static(self) -> &'static str {
        self.into()
    }
}

impl Default for Operation {
    fn default() -> Self {
        Operation::Metadata
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.into_static())
    }
}

impl From<Operation> for &'static str {
    fn from(v: Operation) -> &'static str {
        match v {
            Operation::Metadata => "metadata",
            Operation::Create => "create",
            Operation::Read => "read",
            Operation::Write => "write",
            Operation::Stat => "stat",
            Operation::Delete => "delete",
            Operation::List => "list",
            Operation::Presign => "presign",
            Operation::CreateMultipart => "create_multipart",
            Operation::WriteMultipart => "write_multipart",
            Operation::CompleteMultipart => "complete_multipart",
            Operation::AbortMultipart => "abort_multipart",
            Operation::BlockingCreate => "blocking_create",
            Operation::BlockingRead => "blocking_read",
            Operation::BlockingWrite => "blocking_write",
            Operation::BlockingStat => "blocking_stat",
            Operation::BlockingDelete => "blocking_delete",
            Operation::BlockingList => "blocking_list",
        }
    }
}

impl From<Operation> for String {
    fn from(v: Operation) -> Self {
        v.into_static().to_string()
    }
}
