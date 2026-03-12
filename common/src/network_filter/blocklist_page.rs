// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use core::ops::Range;
use core::slice::from_raw_parts;

#[cfg(feature = "user")]
use aya::Pod;

/*
The blocklist is encoded as ordered list of strings, ready for a binary search. Due to the
limitations of eBPF maps, we arrange the list in pages of BYTES_PER_BLOCKLIST_PAGE each. We
first search for the page containing a given string, then for the entry within the page.

The ordered list allows us to do a prefix search, which we can use to find domain entries covering
a given name. Since we need to search for a common suffix (e.g. ...google.com), comparing and
sorting must be done on reversed strings (starting at the end of the string).

Each page has a variable sized header of the following form:

struct BlocklistPageHeader {
    entry_count: u16,
    string_offset: [u16; entry_count + 1];
}

`string_offset` is counted from the beginning of the page. The most significant bit (bit 15)
is used to identify domain entries (set to 1 for domain entries, 0 for host entries). This allows
page sizes of up to 32k. The last offset (at position `[entry_count]`) indicates the end of the
last string (but not the start of the next one). Each string ends where the next begins.
 */

pub const BYTES_PER_BLOCKLIST_PAGE: usize = 2048;

#[cfg_attr(feature = "user", derive(Clone, Copy))]
#[repr(C)]
pub struct NameBlocklistPage {
    pub entry_count: u16,
    // actually variable sized [u16; entry_count + 1]:
    pub string_offset: [u16; BYTES_PER_BLOCKLIST_PAGE / size_of::<u16>() - 1],
    // data follows at variable position
}

impl NameBlocklistPage {
    pub fn entry_at_index(&self, index: u16) -> (Range<usize>, bool) {
        // We need access to `index` and `index + 1` and both must be within map value
        let index = index as usize;
        let index = index.min(self.string_offset.len() - 2);
        let offset = self.string_offset[index];
        let is_domain = offset & 0x8000 != 0;
        let offset = offset & 0x7fff;
        let next_offset = self.string_offset[index + 1] & 0x7fff;
        (Range { start: offset as usize, end: next_offset as usize }, is_domain)
    }

    /// This function cannot be used in the eBPF program because it fails to verify. You can use
    /// it in user space code, though.
    pub fn bytes_in_range(&self, byte_range: Range<usize>) -> &[u8] {
        if byte_range.end > BYTES_PER_BLOCKLIST_PAGE || byte_range.start > byte_range.end {
            panic!();
        }
        let base = self as *const _ as usize;
        let ptr = (base + byte_range.start) as *const u8;
        unsafe { from_raw_parts(ptr, byte_range.end - byte_range.start) }
    }
}

#[cfg_attr(feature = "user", derive(Clone, Copy))]
#[repr(C)]
pub struct IpBlocklistPage<T, const N: usize> {
    pub entries: [T; N],
}

pub const IPV4_BLOCKLIST_PAGE_ENTRY_COUNT: usize =
    BYTES_PER_BLOCKLIST_PAGE / size_of::<u32>() / 2 * 2;
pub const IPV6_BLOCKLIST_PAGE_ENTRY_COUNT: usize =
    BYTES_PER_BLOCKLIST_PAGE / size_of::<u128>() / 2 * 2;

pub type Ipv4BlocklistPage = IpBlocklistPage<u32, { IPV4_BLOCKLIST_PAGE_ENTRY_COUNT }>;
pub type Ipv6BlocklistPage = IpBlocklistPage<u128, { IPV6_BLOCKLIST_PAGE_ENTRY_COUNT }>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct BlocklistMatch {
    pub page_index: u32,
    pub entry_index: u16,
    pub generation: u16,
}

#[cfg(feature = "user")]
unsafe impl Pod for NameBlocklistPage {}
#[cfg(feature = "user")]
unsafe impl Pod for Ipv4BlocklistPage {}
#[cfg(feature = "user")]
unsafe impl Pod for Ipv6BlocklistPage {}
