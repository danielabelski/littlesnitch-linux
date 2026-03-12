// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use core::{cmp::Ordering, ops::RangeInclusive};

/// A range type similar to RangeInclusive, but without any invariants and without an additional
/// boolean to represent an empty range. Consequently, it is impossible to represent an empty
/// range with this type.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ClosedRange<T> {
    pub start: T,
    pub end: T,
}

impl<T> From<RangeInclusive<T>> for ClosedRange<T> {
    fn from(value: RangeInclusive<T>) -> Self {
        let (start, end) = value.into_inner();
        Self { start, end }
    }
}

impl<T: Ord> Ord for ClosedRange<T> {
    /// Sort for ascending start first, then for descending end. Long ranges are sorted
    /// before short ones so that covering ranges precede covered.
    fn cmp(&self, other: &Self) -> Ordering {
        match self.start.cmp(&other.start) {
            Ordering::Equal => self.end.cmp(&other.end).reverse(),
            other => other,
        }
    }
}

impl<T: Ord> PartialOrd for ClosedRange<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> ClosedRange<T> {
    pub fn contains(&self, value: &T) -> bool {
        value >= &self.start && value <= &self.end
    }
}

// This trait is not directly related to ClosedRange, but it is required when converting a
// closed range to a half-open range. We therefore include it here.
pub trait NumericBound: Ord + Sized {
    /// Adds one to `self`, needed to convert the end of a closed range to the end of a half-open
    /// range. This function is not supposed to panic, it should just wrap to 0 when the maximum
    /// value is exceeded.
    fn plus_one(&self) -> Self;

    /// Can be used to check whether a wrap-around occurred after adding one.
    fn is_zero(&self) -> bool;

    /// Checks whether `self` is already the maximum possible value and adding one would overflow.
    /// Needed to check whether a representation as half-open range is possible.
    fn is_max(&self) -> bool {
        self.plus_one().is_zero()
    }
}

impl NumericBound for u32 {
    fn plus_one(&self) -> Self {
        self.wrapping_add(1)
    }

    fn is_zero(&self) -> bool {
        self == &0
    }
}

impl NumericBound for u128 {
    fn plus_one(&self) -> Self {
        self.wrapping_add(1)
    }

    fn is_zero(&self) -> bool {
        self == &0
    }
}
