// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

#[cfg(target_arch = "bpf")]
use aya_ebpf::{cty::c_void, helpers::generated::bpf_loop};

#[derive(PartialEq, Eq)]
#[repr(i64)]
pub enum LoopReturn {
    LoopContinue = 0,
    LoopBreak = 1,
}

pub type LoopFunction<C> = extern "C" fn(u64, &mut C) -> LoopReturn;

#[cfg(target_arch = "bpf")]
#[inline(always)]
pub fn repeat<C>(count: u64, function: LoopFunction<C>, context: &mut C) -> u64 {
    unsafe {
        bpf_loop(
            count as _,
            function as *mut c_void,
            context as *mut _ as *mut c_void,
            0,
        ) as _
    }
}

#[cfg(not(target_arch = "bpf"))]
#[inline(always)]
pub fn repeat<C>(count: u64, function: LoopFunction<C>, context: &mut C) -> u64 {
    for i in 0..(count as u64) {
        if function(i, context) == LoopReturn::LoopBreak {
            return i + 1;
        }
    }
    count
}
