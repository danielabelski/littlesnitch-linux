// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    ByteAtOffset, DOMAIN_SEP,
    repeat::{LoopReturn, repeat_closure},
    touch_usize,
};
use core::{
    fmt::{self, Debug},
    mem::MaybeUninit,
    ptr,
    slice::from_raw_parts,
};

#[derive(PartialEq, Eq)]
#[repr(align(8))]
#[cfg_attr(feature = "user", derive(Clone, Copy))]
pub struct BpfString {
    pub len: u8,
    pub data: [u8; 255],
}

// used by unit tests
impl Debug for BpfString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl BpfString {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut string = MaybeUninit::<Self>::uninit();
        let this = string.as_mut_ptr();
        let buffer = unsafe { &mut (*this).data as *mut u8 };
        let string_len = bytes.len();
        let max_len = unsafe { size_of_val(&(*this).data) };
        let len = string_len.min(max_len);
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, len);
            ptr::write_bytes(buffer.add(len), 0, max_len - len);
            (*this).len = len as u8;
            string.assume_init()
        }
    }

    pub fn from_str_bytes(bytes: &[u8]) -> Self {
        let mut string = Self::from_bytes(bytes);
        for i in 0..string.len() {
            if string.data[i] == b'.' {
                string.data[i] = DOMAIN_SEP;
            }
        }
        string
    }

    pub fn update(&mut self, block: impl FnOnce(&mut [u8; 255]) -> u8) {
        self.len = block(&mut self.data);
    }

    // We must trick the compiler so that it does not know that we are zeroing out memory.
    // If it finds out, it replaces our code with a memset function which works byte-wise,
    // which is much less efficient and blasts our eBPF verifier budget of instructions.
    pub fn clear(&mut self) {
        let ptr = self as *mut _ as *mut u64;
        repeat_closure(32, |i| unsafe {
            if i < 32 {
                *ptr.add(i) = 0;
            }
            LoopReturn::LoopContinue
        });
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(&self.data as *const u8, self.len as usize) }
    }

    // for logging
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_slice()) }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }
}

impl ByteAtOffset for BpfString {
    fn byte_at_offset(&self, mut index: usize) -> u8 {
        touch_usize(&mut index); // prevent optimization of range check
        if index < self.len() {
            self.data[index]
        } else {
            0
        }
    }
}

// used for unit tests and in daemon
impl Default for BpfString {
    fn default() -> Self {
        Self { len: 0, data: [0u8; _] }
    }
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for BpfString {}
