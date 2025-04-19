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

use std::fmt;
use std::ops::Deref;

/// Represents a result identifier for quantum measurements
///
/// This type provides a simple wrapper around `usize` to represent
/// result identifiers for quantum measurements.
///
/// # Examples
///
/// ```
/// use pecos_engines::core::result_id::ResultId;
///
/// // Create a ResultId with ID 0
/// let id = ResultId(0);
///
/// // Access the inner value
/// assert_eq!(id.0, 0);
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ResultId(pub usize);

impl ResultId {
    /// Create a new `ResultId`
    #[must_use]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Get the underlying ID value
    #[must_use]
    pub fn id(&self) -> usize {
        self.0
    }
}

// Automatic conversion from usize to ResultId
impl From<usize> for ResultId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

// Automatic conversion from ResultId to usize
impl From<ResultId> for usize {
    fn from(result_id: ResultId) -> Self {
        result_id.0
    }
}

impl Deref for ResultId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ResultId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
