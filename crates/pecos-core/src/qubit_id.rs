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

use crate::IndexableElement;
use std::fmt;
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct QubitId(pub usize);

impl IndexableElement for QubitId {
    #[inline]
    fn to_index(&self) -> usize {
        self.0
    }

    #[inline]
    fn from_index(value: usize) -> Self {
        Self(value)
    }
}

// Automatic conversion from usize to QubitId
impl From<usize> for QubitId {
    fn from(value: usize) -> Self {
        QubitId(value)
    }
}

// Automatic conversion from QubitId to usize
impl From<QubitId> for usize {
    fn from(qubit: QubitId) -> usize {
        qubit.0
    }
}

// Add Deref implementation to match QubitIndex
impl Deref for QubitId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Add Display implementation to match QubitIndex
impl fmt::Display for QubitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Add convenience methods to match QubitIndex
impl QubitId {
    /// Create a new `QubitId`
    #[must_use]
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// Get the underlying index value
    #[must_use]
    pub fn index(&self) -> usize {
        self.0
    }
}
