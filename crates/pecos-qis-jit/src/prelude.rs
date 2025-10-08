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

//! A prelude for users of the `pecos-qis-jit` crate.
//!
//! This prelude re-exports the most commonly used types, traits, and functions
//! needed for working with JIT-based QIS interfaces in PECOS.

// Re-export builder types
pub use crate::builder::{JitInterfaceBuilder, jit_interface_builder};

// Re-export main interface type
pub use crate::jit_interface::QisJitInterface;

// Re-export executor
pub use crate::jit_executor::JitExecutor;

// Re-export measurement manager utilities
pub use crate::measurement_manager::{
    JitMeasurementManager, reset_measurement_manager, with_measurement_manager,
    with_measurement_manager_mut,
};
