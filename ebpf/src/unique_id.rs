// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use core::num::NonZeroU64;
use core::mem::transmute;
use aya_ebpf::{helpers::generated::bpf_get_smp_processor_id, macros::map, maps::PerCpuArray};

#[map]
static UNIQUE_ID_COUNTER: PerCpuArray<u64> = PerCpuArray::with_max_entries(10, 0);

pub struct UniqueId {
    counter: *mut u64,
}

#[repr(u32)]
pub enum Purpose {
    StringId,
    NodeId,
}

impl UniqueId {

    pub fn new(purpose: Purpose) -> Self {
        let counter = UNIQUE_ID_COUNTER.get_ptr_mut(purpose as _).unwrap_or_default();
        UniqueId { counter }
    }

    /// Obtain the current unique ID. The caller may decide not to use it.
    /// If the ID is used, call `consume()`. If not, just drop the `UniqueId`` struct.
    pub fn get(&self) -> NonZeroU64 {
        // Build the ID from the CPU index (16 bit) and a per-cpu counter (48 bit). The size of
        // 48 bits ensures that a wrap-around occurs only every 30 years, even if we request a
        // new ID every 10 nanoseconds.
        // Add 1 to the CPU index to leave room for "user space" at index 0 and to ensure that
        // the value will never be 0, even though the maps are initialized to 0.
        let cpu_index = unsafe { bpf_get_smp_processor_id() } + 1;
        let value = unsafe { (*self.counter << 16) | cpu_index as u64 };
        // use transmute to avoid any possible panics
        unsafe { transmute(value) }
    }

    /// Increment the per-CPU counter so the next call to `get()` returns a fresh ID.
    /// Must be called exactly once after the ID returned by `get()` has been committed to a map.
    pub fn consume(&mut self) {
        unsafe { *self.counter += 1 };
    }
}
