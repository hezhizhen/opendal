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

//! Providing Layer implementations.

mod concurrent_limit;
pub use concurrent_limit::ConcurrentLimitLayer;

mod immutable_index;
pub use immutable_index::ImmutableIndexLayer;

mod logging;
pub use logging::LoggingLayer;

#[cfg(feature = "layers-metrics")]
mod metrics;
#[cfg(feature = "layers-metrics")]
pub use self::metrics::MetricsLayer;

mod retry;
pub use self::retry::RetryLayer;

#[cfg(feature = "layers-tracing")]
mod tracing;
#[cfg(feature = "layers-tracing")]
pub use self::tracing::TracingLayer;

mod type_eraser;
pub(crate) use type_eraser::TypeEraseLayer;

mod error_context;
pub(crate) use error_context::ErrorContextLayer;
