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
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::de;
use time::format_description::well_known::Rfc2822;
use time::OffsetDateTime;

use super::backend::AzdfsBackend;
use super::error::parse_error;
use crate::raw::*;
use crate::*;

pub struct DirStream {
    backend: Arc<AzdfsBackend>,
    root: String,
    path: String,

    continuation: String,
    done: bool,
}

impl DirStream {
    pub fn new(backend: Arc<AzdfsBackend>, root: String, path: String) -> Self {
        Self {
            backend,
            root,
            path,

            continuation: "".to_string(),
            done: false,
        }
    }
}

#[async_trait]
impl ObjectPage for DirStream {
    async fn next_page(&mut self) -> Result<Option<Vec<ObjectEntry>>> {
        if self.done {
            return Ok(None);
        }

        let resp = self
            .backend
            .azdfs_list(&self.path, &self.continuation)
            .await?;

        // Azdfs will return not found for not-exist path.
        if resp.status() == http::StatusCode::NOT_FOUND {
            resp.into_body().consume().await?;
            return Ok(None);
        }
        if resp.status() != http::StatusCode::OK {
            return Err(parse_error(resp).await?);
        }

        // Check whether this list is done.
        if let Some(value) = resp.headers().get("x-ms-continuation") {
            let value = value.to_str().map_err(|err| {
                Error::new(ErrorKind::Unexpected, "header value is not valid string")
                    .set_source(err)
            })?;
            self.continuation = value.to_string();
        } else {
            self.continuation = "".to_string();
            self.done = true;
        }

        let bs = resp.into_body().bytes().await?;

        let output: Output = de::from_slice(&bs).map_err(|e| {
            Error::new(ErrorKind::Unexpected, "deserialize json from response").set_source(e)
        })?;

        let mut entries = Vec::with_capacity(output.paths.len());

        for object in output.paths {
            // Azdfs will return `"true"` and `"false"` for is_directory.
            let mode = if &object.is_directory == "true" {
                ObjectMode::DIR
            } else {
                ObjectMode::FILE
            };

            let meta = ObjectMetadata::new(mode)
                // Keep fit with ETag header.
                .with_etag(&format!("\"{}\"", &object.etag))
                .with_content_length(object.content_length.parse().map_err(|err| {
                    Error::new(ErrorKind::Unexpected, "content length is not valid integer")
                        .set_source(err)
                })?)
                .with_last_modified(
                    OffsetDateTime::parse(&object.last_modified, &Rfc2822).map_err(|e| {
                        Error::new(
                            ErrorKind::Unexpected,
                            "last modified is not valid RFC2822 datetime",
                        )
                        .set_source(e)
                    })?,
                )
                .with_complete();

            let mut path = build_rel_path(&self.root, &object.name);
            if mode == ObjectMode::DIR {
                path += "/"
            };

            let de = ObjectEntry::new(&path, meta);

            entries.push(de);
        }

        Ok(Some(entries))
    }
}

/// # Examples
///
/// ```json
/// {"paths":[{"contentLength":"1977097","etag":"0x8DACF9B0061305F","group":"$superuser","lastModified":"Sat, 26 Nov 2022 10:43:05 GMT","name":"c3b3ef48-7783-4946-81bc-dc07e1728878/d4ea21d7-a533-4011-8b1f-d0e566d63725","owner":"$superuser","permissions":"rw-r-----"}]}
/// ```
#[derive(Default, Debug, Deserialize)]
#[serde(default)]
struct Output {
    paths: Vec<Path>,
}

#[derive(Default, Debug, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct Path {
    #[serde(rename = "contentLength")]
    content_length: String,
    #[serde(rename = "etag")]
    etag: String,
    /// Azdfs will return `"true"` and `"false"` for is_directory.
    #[serde(rename = "isDirectory")]
    is_directory: String,
    #[serde(rename = "lastModified")]
    last_modified: String,
    #[serde(rename = "name")]
    name: String,
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    #[test]
    fn test_parse_path() {
        let bs = Bytes::from(
            r#"{"paths":[{"contentLength":"1977097","etag":"0x8DACF9B0061305F","group":"$superuser","lastModified":"Sat, 26 Nov 2022 10:43:05 GMT","name":"c3b3ef48-7783-4946-81bc-dc07e1728878/d4ea21d7-a533-4011-8b1f-d0e566d63725","owner":"$superuser","permissions":"rw-r-----"}]}"#,
        );
        let out: Output = de::from_slice(&bs).expect("must success");
        println!("{out:?}");

        assert_eq!(
            out.paths[0],
            Path {
                content_length: "1977097".to_string(),
                etag: "0x8DACF9B0061305F".to_string(),
                is_directory: "".to_string(),
                last_modified: "Sat, 26 Nov 2022 10:43:05 GMT".to_string(),
                name: "c3b3ef48-7783-4946-81bc-dc07e1728878/d4ea21d7-a533-4011-8b1f-d0e566d63725"
                    .to_string()
            }
        );
    }
}
