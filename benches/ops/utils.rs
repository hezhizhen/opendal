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

use std::env;

use bytes::Bytes;
use once_cell::sync::Lazy;
use opendal::*;
use rand::prelude::*;

pub static TOKIO: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("build tokio runtime"));

fn service<AB: Builder>() -> Option<Operator> {
    let test_key = format!("opendal_{}_test", AB::SCHEME).to_uppercase();
    if env::var(test_key).unwrap_or_default() != "on" {
        return None;
    }

    Some(
        Operator::from_env::<AB>()
            .unwrap_or_else(|_| panic!("init {} must succeed", AB::SCHEME))
            .finish(),
    )
}

pub fn services() -> Vec<(&'static str, Option<Operator>)> {
    let _ = dotenvy::dotenv();

    vec![
        ("fs", service::<services::Fs>()),
        ("s3", service::<services::S3>()),
        ("memory", service::<services::Memory>()),
    ]
}

pub fn gen_bytes(rng: &mut ThreadRng, size: usize) -> Vec<u8> {
    let mut content = vec![0; size];
    rng.fill_bytes(&mut content);

    content
}

pub struct TempData {
    op: Operator,
    path: String,
}

impl TempData {
    pub fn existing(op: Operator, path: &str) -> Self {
        Self {
            op,
            path: path.to_string(),
        }
    }

    pub fn generate(op: Operator, path: &str, content: Vec<u8>) -> Self {
        TOKIO.block_on(async {
            op.object(path)
                .write(Bytes::from(content))
                .await
                .expect("create test data")
        });

        Self {
            op,
            path: path.to_string(),
        }
    }
}

impl Drop for TempData {
    fn drop(&mut self) {
        TOKIO.block_on(async {
            self.op
                .object(&self.path)
                .delete()
                .await
                .expect("cleanup test data");
        })
    }
}
