// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::demo_filter_maps::DemoFilterMaps;
use common::network_filter::blocklist_page::*;
use std::{ptr, slice::from_raw_parts};

pub struct DemoBlocklistEntry<'a> {
    pub name: &'a [u8],
    pub is_domain: bool,
}

impl DemoFilterMaps {
    /// Returns the number of pages used for blocklists
    pub fn set_name_blocklist_entries(&mut self, mut entries: Vec<DemoBlocklistEntry>) -> usize {
        // Sort must use the same encoding as the page (0-byte separator, not dot), because the
        // binary search comparison reads the stored bytes where dots have been replaced by 0.
        // '-' (45) < '.' (46) but '-' (45) > 0, so using dots vs 0 gives different orderings
        // for names with hyphens, which breaks binary search.
        let encode = |&b: &u8| if b == b'.' { 0u8 } else { b };
        entries.sort_by(|a, b| a.name.iter().map(encode).rev().cmp(b.name.iter().map(encode).rev()));
        let entries_len = entries.len();
        let mut raw_page = DemoRawPage::new();
        let mut pages = Vec::<NameBlocklistPage>::new();
        for DemoBlocklistEntry { name, is_domain } in entries {
            if raw_page.prospective_len(name) >= BYTES_PER_BLOCKLIST_PAGE {
                pages.push(raw_page.emit_page());
            }
            raw_page.push_name(name, is_domain);
        }
        if !raw_page.is_empty() {
            pages.push(raw_page.emit_page());
        }
        let count = pages.len();
        for (index, page) in pages.into_iter().enumerate() {
            _ = self.name_blocklist.set(index as _, page, 0);
        }
        println!(
            "{} blocklist entries in {} pages of {} bytes each, total {} bytes.",
            entries_len,
            count,
            BYTES_PER_BLOCKLIST_PAGE,
            count * BYTES_PER_BLOCKLIST_PAGE
        );
        count
    }
}

struct DemoRawPage {
    pub strings_buffer: Vec<u8>,
    pub offset_buffer: Vec<u16>,
}

impl DemoRawPage {
    fn new() -> Self {
        Self {
            strings_buffer: Vec::new(),
            offset_buffer: Vec::new(),
        }
    }

    fn push_name(&mut self, name: &[u8], is_domain: bool) {
        assert!(name.len() > 0);
        let domain_tag = if is_domain { 0x8000u16 } else { 0 };
        self.offset_buffer.push(domain_tag | self.strings_buffer.len() as u16);
        for b in name {
            if b == &b'.' {
                // we represent the domain separator by a 0-byte
                self.strings_buffer.push(0);
            } else {
                self.strings_buffer.push(*b);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.offset_buffer.is_empty()
    }

    fn len(&self) -> usize {
        size_of::<u16>() + self.offset_buffer.len() * size_of::<u16>() + self.strings_buffer.len()
    }

    // length when written to a NameBlocklistPage
    fn prospective_len(&self, next_name: &[u8]) -> usize {
        // +size_of::<u16>() for new entry's offset, +size_of::<u16>() for the terminal offset
        // that emit_page always appends — len() does not include the terminal offset.
        self.len() + size_of::<u16>() * 2 + next_name.len()
    }

    fn emit_page(&mut self) -> NameBlocklistPage {
        self.offset_buffer.push(self.strings_buffer.len() as _);
        let mut page = NameBlocklistPage {
            entry_count: (self.offset_buffer.len() - 1) as _,
            string_offset: [0u16; _],
        };
        let strings_offset = self.offset_buffer.len() * size_of::<u16>() + size_of::<u16>();
        for (index, offset) in self.offset_buffer.iter().enumerate() {
            page.string_offset[index] = *offset + strings_offset as u16;
        }
        unsafe {
            ptr::copy(
                self.strings_buffer.as_ptr(),
                (&mut page as *mut _ as *mut u8).add(strings_offset),
                self.strings_buffer.len(),
            );
        }
        self.offset_buffer.clear();
        self.strings_buffer.clear();
        page
    }
}

// for debugging
fn _demo_print_page(page: &NameBlocklistPage) {
    println!("Blocklist Page with {} entries:", page.entry_count);
    for i in 0..(page.entry_count as usize) {
        let offset = page.string_offset[i] & 0x7fff;
        let length = (page.string_offset[i + 1] & 0x7fff) - offset;
        let start_ptr = unsafe { (page as *const _ as *const u8).add(offset as _) };
        let bytes = unsafe { from_raw_parts(start_ptr, length as _) };
        println!("entry at offset {}: {}", offset, unsafe { str::from_utf8_unchecked(bytes) });
    }
}
