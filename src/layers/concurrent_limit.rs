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
use std::io::SeekFrom;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::OwnedSemaphorePermit;
use tokio::sync::Semaphore;

use crate::raw::*;
use crate::*;

/// ConcurrentLimitLayer will add concurrent limit for OpenDAL.
///
/// # Examples
///
/// ```
/// use anyhow::Result;
/// use opendal::layers::ConcurrentLimitLayer;
/// use opendal::services;
/// use opendal::Operator;
/// use opendal::Scheme;
///
/// let _ = Operator::from_env::<services::Fs>()
///     .expect("must init")
///     .layer(ConcurrentLimitLayer::new(1024))
///     .finish();
/// ```
pub struct ConcurrentLimitLayer {
    permits: usize,
}

impl ConcurrentLimitLayer {
    /// Create a new ConcurrentLimitLayer will specify permits
    pub fn new(permits: usize) -> Self {
        Self { permits }
    }
}

impl<A: Accessor> Layer<A> for ConcurrentLimitLayer {
    type LayeredAccessor = ConcurrentLimitAccessor<A>;

    fn layer(&self, inner: A) -> Self::LayeredAccessor {
        ConcurrentLimitAccessor {
            inner,
            semaphore: Arc::new(Semaphore::new(self.permits)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConcurrentLimitAccessor<A: Accessor> {
    inner: A,
    semaphore: Arc<Semaphore>,
}

#[async_trait]
impl<A: Accessor> LayeredAccessor for ConcurrentLimitAccessor<A> {
    type Inner = A;
    type Reader = ConcurrentLimitReader<A::Reader>;
    type BlockingReader = ConcurrentLimitReader<A::BlockingReader>;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    async fn create(&self, path: &str, args: OpCreate) -> Result<RpCreate> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.create(path, args).await
    }

    async fn read(&self, path: &str, args: OpRead) -> Result<(RpRead, Self::Reader)> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore must be valid");

        self.inner
            .read(path, args)
            .await
            .map(|(rp, r)| (rp, ConcurrentLimitReader::new(r, permit)))
    }

    async fn write(&self, path: &str, args: OpWrite, r: input::Reader) -> Result<RpWrite> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.write(path, args, r).await
    }

    async fn stat(&self, path: &str, args: OpStat) -> Result<RpStat> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.stat(path, args).await
    }

    async fn delete(&self, path: &str, args: OpDelete) -> Result<RpDelete> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.delete(path, args).await
    }

    async fn list(&self, path: &str, args: OpList) -> Result<(RpList, ObjectPager)> {
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore must be valid");

        self.inner.list(path, args).await.map(|(rp, s)| {
            (
                rp,
                Box::new(ConcurrentLimitPager::new(s, permit)) as ObjectPager,
            )
        })
    }

    async fn create_multipart(
        &self,
        path: &str,
        args: OpCreateMultipart,
    ) -> Result<RpCreateMultipart> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.create_multipart(path, args).await
    }

    async fn write_multipart(
        &self,
        path: &str,
        args: OpWriteMultipart,
        r: input::Reader,
    ) -> Result<RpWriteMultipart> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.write_multipart(path, args, r).await
    }

    async fn complete_multipart(
        &self,
        path: &str,
        args: OpCompleteMultipart,
    ) -> Result<RpCompleteMultipart> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.complete_multipart(path, args).await
    }

    async fn abort_multipart(
        &self,
        path: &str,
        args: OpAbortMultipart,
    ) -> Result<RpAbortMultipart> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore must be valid");

        self.inner.abort_multipart(path, args).await
    }

    fn blocking_create(&self, path: &str, args: OpCreate) -> Result<RpCreate> {
        let _permit = self
            .semaphore
            .try_acquire()
            .expect("semaphore must be valid");

        self.inner.blocking_create(path, args)
    }

    fn blocking_read(&self, path: &str, args: OpRead) -> Result<(RpRead, Self::BlockingReader)> {
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .expect("semaphore must be valid");

        self.inner
            .blocking_read(path, args)
            .map(|(rp, r)| (rp, ConcurrentLimitReader::new(r, permit)))
    }

    fn blocking_write(
        &self,
        path: &str,
        args: OpWrite,
        r: input::BlockingReader,
    ) -> Result<RpWrite> {
        let _permit = self
            .semaphore
            .try_acquire()
            .expect("semaphore must be valid");

        self.inner.blocking_write(path, args, r)
    }

    fn blocking_stat(&self, path: &str, args: OpStat) -> Result<RpStat> {
        let _permit = self
            .semaphore
            .try_acquire()
            .expect("semaphore must be valid");

        self.inner.blocking_stat(path, args)
    }

    fn blocking_delete(&self, path: &str, args: OpDelete) -> Result<RpDelete> {
        let _permit = self
            .semaphore
            .try_acquire()
            .expect("semaphore must be valid");

        self.inner.blocking_delete(path, args)
    }

    fn blocking_list(&self, path: &str, args: OpList) -> Result<(RpList, BlockingObjectPager)> {
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .expect("semaphore must be valid");

        self.inner.blocking_list(path, args).map(|(rp, it)| {
            (
                rp,
                Box::new(BlockingConcurrentLimitPager::new(it, permit)) as BlockingObjectPager,
            )
        })
    }
}

pub struct ConcurrentLimitReader<R> {
    inner: R,

    // Hold on this permit until this reader has been dropped.
    _permit: OwnedSemaphorePermit,
}

impl<R> ConcurrentLimitReader<R> {
    fn new(inner: R, permit: OwnedSemaphorePermit) -> Self {
        Self {
            inner,
            _permit: permit,
        }
    }
}

impl<R: output::Read> output::Read for ConcurrentLimitReader<R> {
    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<std::io::Result<usize>> {
        self.inner.poll_read(cx, buf)
    }

    fn poll_seek(&mut self, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<std::io::Result<u64>> {
        self.inner.poll_seek(cx, pos)
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<std::io::Result<Bytes>>> {
        self.inner.poll_next(cx)
    }
}

impl<R: output::BlockingRead> output::BlockingRead for ConcurrentLimitReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }

    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }

    fn next(&mut self) -> Option<std::io::Result<Bytes>> {
        self.inner.next()
    }
}

struct ConcurrentLimitPager {
    inner: ObjectPager,

    // Hold on this permit until this streamer has been dropped.
    _permit: OwnedSemaphorePermit,
}

impl ConcurrentLimitPager {
    fn new(inner: ObjectPager, permit: OwnedSemaphorePermit) -> Self {
        Self {
            inner,
            _permit: permit,
        }
    }
}

#[async_trait]
impl ObjectPage for ConcurrentLimitPager {
    async fn next_page(&mut self) -> Result<Option<Vec<ObjectEntry>>> {
        self.inner.next_page().await
    }
}

struct BlockingConcurrentLimitPager {
    inner: BlockingObjectPager,

    // Hold on this permit until this iterator has been dropped.
    _permit: OwnedSemaphorePermit,
}

impl BlockingConcurrentLimitPager {
    fn new(inner: BlockingObjectPager, permit: OwnedSemaphorePermit) -> Self {
        Self {
            inner,
            _permit: permit,
        }
    }
}

impl BlockingObjectPage for BlockingConcurrentLimitPager {
    fn next_page(&mut self) -> Result<Option<Vec<ObjectEntry>>> {
        self.inner.next_page()
    }
}
