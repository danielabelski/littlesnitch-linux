// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use core::cmp::Ordering;

/// This type is a generalized `core::cmp::Ordering`:
/// 0 if two items compare equal
/// negative (instead of -1) represents Less
/// positive (instead of +1) represents Greater.
/// The magnitude can be used to express the quality of a match, e.g. how many bytes of a string
/// were equal (plus 1, to avoid using 0).
pub struct ExtOrder {
    pub value: isize,
}

impl ExtOrder {
    pub fn from(order: Ordering, magnitude: usize) -> Self {
        Self { value: (order as isize) * (magnitude as isize + 1) }
    }

    pub fn equal() -> Self {
        Self { value: 0 }
    }

    pub fn greater(magnitude: usize) -> Self {
        Self { value: magnitude as isize + 1 }
    }

    pub fn less(magnitude: usize) -> Self {
        Self { value: -1 - magnitude as isize }
    }

    pub fn is_less(&self) -> bool {
        self.value < 0
    }

    pub fn is_greater(&self) -> bool {
        self.value > 0
    }

    pub fn is_equal(&self) -> bool {
        self.value == 0
    }

    pub fn magnitude(&self) -> usize {
        self.value.abs() as usize - 1 // this definition automatically assigns usize::MAX for equal
    }

    pub fn reverse(&self) -> Self {
        Self { value: -self.value }
    }
}
