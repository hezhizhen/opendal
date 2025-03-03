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

use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::sync::Arc;

use async_trait::async_trait;
use rocksdb::TransactionDB;

use crate::raw::adapters::kv;
use crate::raw::*;
use crate::Result;
use crate::*;

/// Rocksdb support for OpenDAL
///
/// # Note
///
/// The storage format for this service is not **stable** yet.
///
/// PLEASE DON'T USE THIS SERVICE FOR PERSIST DATA.
///
/// # Configuration
///
/// - `root`: Set the working directory of `OpenDAL`
/// - `datadir`: Set the path to the rocksdb data directory
///
/// You can refer to [`RocksdbBuilder`]'s docs for more information
///
/// # Example
///
/// ## Via Builder
///
/// ```no_run
/// use anyhow::Result;
/// use opendal::services::Rocksdb;
/// use opendal::Object;
/// use opendal::Operator;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let mut builder = Rocksdb::default();
///     builder.datadir("/tmp/opendal/rocksdb");
///
///     let op: Operator = Operator::create(builder)?.finish();
///     let _: Object = op.object("test_file");
///     Ok(())
/// }
/// ```
#[derive(Clone, Default, Debug)]
pub struct RocksdbBuilder {
    /// The path to the rocksdb data directory.
    datadir: Option<String>,
    /// the working directory of the service. Can be "/path/to/dir"
    ///
    /// default is "/"
    root: Option<String>,
}

impl RocksdbBuilder {
    /// Set the path to the rocksdb data directory. Will create if not exists.
    pub fn datadir(&mut self, path: &str) -> &mut Self {
        self.datadir = Some(path.into());
        self
    }

    /// set the working directory, all operations will be performed under it.
    ///
    /// default: "/"
    pub fn root(&mut self, root: &str) -> &mut Self {
        if !root.is_empty() {
            self.root = Some(root.to_owned());
        }
        self
    }
}

impl Builder for RocksdbBuilder {
    const SCHEME: Scheme = Scheme::Rocksdb;
    type Accessor = RocksdbBackend;

    fn from_map(map: HashMap<String, String>) -> Self {
        let mut builder = RocksdbBuilder::default();

        map.get("datadir").map(|v| builder.datadir(v));

        builder
    }

    fn build(&mut self) -> Result<Self::Accessor> {
        let path = self.datadir.take().ok_or_else(|| {
            Error::new(
                ErrorKind::BackendConfigInvalid,
                "datadir is required but not set",
            )
            .with_context("service", Scheme::Rocksdb)
        })?;
        let db = TransactionDB::open_default(&path).map_err(|e| {
            Error::new(
                ErrorKind::BackendConfigInvalid,
                "open default transaction db",
            )
            .with_context("service", Scheme::Rocksdb)
            .with_context("datadir", path)
            .set_source(e)
        })?;

        Ok(RocksdbBackend::new(Adapter { db: Arc::new(db) }))
    }
}

/// Backend for rocksdb services.
pub type RocksdbBackend = kv::Backend<Adapter>;

#[derive(Clone)]
pub struct Adapter {
    db: Arc<TransactionDB>,
}

impl Debug for Adapter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("Adapter");
        ds.field("path", &self.db.path());
        ds.finish()
    }
}

#[async_trait]
impl kv::Adapter for Adapter {
    fn metadata(&self) -> kv::Metadata {
        kv::Metadata::new(
            Scheme::Rocksdb,
            &self.db.path().to_string_lossy(),
            AccessorCapability::Read | AccessorCapability::Write,
        )
    }

    async fn get(&self, path: &str) -> Result<Option<Vec<u8>>> {
        self.blocking_get(path)
    }

    fn blocking_get(&self, path: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.db.get(path)?)
    }

    async fn set(&self, path: &str, value: &[u8]) -> Result<()> {
        self.blocking_set(path, value)
    }

    fn blocking_set(&self, path: &str, value: &[u8]) -> Result<()> {
        Ok(self.db.put(path, value)?)
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.blocking_delete(path)
    }

    fn blocking_delete(&self, path: &str) -> Result<()> {
        Ok(self.db.delete(path)?)
    }
}

impl From<rocksdb::Error> for Error {
    fn from(e: rocksdb::Error) -> Self {
        Error::new(ErrorKind::Unexpected, "got rocksdb error").set_source(e)
    }
}
