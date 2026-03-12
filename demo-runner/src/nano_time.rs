// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use common::NanoTime;
use libc::{CLOCK_MONOTONIC_COARSE, clock_gettime, timespec};
use std::mem::MaybeUninit;

pub fn now() -> NanoTime {
    let nanos = unsafe {
        let mut timespec = MaybeUninit::<timespec>::uninit();
        clock_gettime(CLOCK_MONOTONIC_COARSE, timespec.as_mut_ptr());
        let timespec = timespec.assume_init();
        timespec.tv_sec * 1000 * 1000 * 1000 + timespec.tv_nsec
    };
    NanoTime(nanos)
}
