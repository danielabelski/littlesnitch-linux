// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use aya_ebpf::{cty::c_void, helpers::generated::bpf_probe_read_kernel, programs::SkBuffContext};
use aya_log_ebpf::error;
use core::{cmp::min, mem::MaybeUninit};

/// Copy kernel memory to result. Can be used to access struct fields when the field offset
/// is known.
#[inline(always)]
pub fn read_at_offset<P, T>(ptr: *const P, offset: usize) -> Option<T> {
    unsafe {
        let mut value = MaybeUninit::<T>::uninit();
        if bpf_probe_read_kernel(
            value.as_mut_ptr() as *mut c_void,
            size_of_val(&value) as u32,
            (ptr as *const u8).add(offset) as _,
        ) == 0
        {
            Some(value.assume_init())
        } else {
            None
        }
    }
}

fn try_hexdump(ctx: &SkBuffContext) -> Result<(), i64> {
    let len = min(128, ctx.skb.len() as usize);
    let mut offset = 0;
    error!(ctx, "packet of length {}:", len);
    for _ in 0..(len / 8) {
        let d0: u8 = ctx.skb.load(offset + 0)?;
        let d1: u8 = ctx.skb.load(offset + 1)?;
        let d2: u8 = ctx.skb.load(offset + 2)?;
        let d3: u8 = ctx.skb.load(offset + 3)?;
        let d4: u8 = ctx.skb.load(offset + 4)?;
        let d5: u8 = ctx.skb.load(offset + 5)?;
        let d6: u8 = ctx.skb.load(offset + 6)?;
        let d7: u8 = ctx.skb.load(offset + 7)?;

        error!(ctx, "  {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}", d0, d1, d2, d3, d4, d5, d6, d7);
        offset += 8;
    }
    while offset < len {
        let d: u8 = ctx.skb.load(offset)?;
        error!(ctx, "  {:x}", d);
        offset += 1;
    }
    Ok(())
}

#[allow(dead_code)]
fn hexdump(ctx: &SkBuffContext) {
    _ = try_hexdump(ctx);
}
