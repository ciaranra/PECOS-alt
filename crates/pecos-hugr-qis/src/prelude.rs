// Copyright 2025 The PECOS Developers
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except
// in compliance with the License.You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License
// is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express
// or implied. See the License for the specific language governing permissions and limitations under
// the License.

//! A prelude for users of the `pecos-hugr-qis` crate.
//!
//! This prelude re-exports the HUGR compilation functionality.

// Re-export compiler types and functions
pub use crate::compiler::compile_hugr_bytes_to_string;
pub use crate::{HugrCompiler, HugrCompilerConfig};
