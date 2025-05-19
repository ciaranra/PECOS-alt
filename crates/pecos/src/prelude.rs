// Copyright 2024 The PECOS Developers
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

pub use pecos_core::prelude::*;
pub use pecos_engines::prelude::*;
pub use pecos_phir::prelude::*;
pub use pecos_qasm::prelude::*;
pub use pecos_qir::prelude::*;
pub use pecos_qsim::prelude::*;

pub use crate::{
    engines::{setup_qasm_engine, setup_qir_engine},
    program::{ProgramType, detect_program_type, get_program_path, setup_engine_for_program},
};
