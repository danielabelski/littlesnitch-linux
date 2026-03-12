// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    ext_order::ExtOrder,
    network_filter::{
        binary_rule::*, binary_searchable_page::BinarySearchablePage, filter_model::FilterTable,
        rule_types::*,
    },
    node_cache::NodeId,
    touch_usize,
};
use core::{cmp::Ordering, marker::PhantomData};

#[cfg(feature = "user")]
use aya::Pod;

pub const BYTES_PER_RULE_PAGE: usize = 2048;

/// The RulePage is the building block of rule matching. A rule list consists of multiple pages
/// and the entries within a page as well as the pages are sorted. This allows searching via
/// binary search. In order to return predictable results, the sort order must be stable between
/// entries and between entries and the search term. This translates to the requirement that
/// rules within the same table must not have overlapping matches. If a search term compares
/// equal with one entry, it must not compare equal with another.
///
/// The rule page is generic in the endpoint address and we later specialize on name, IPv4, IPv6
/// or any endpoint. Entries in the page are of type `BinaryRule`, which is also generic in
/// the endpoint address type.
#[cfg_attr(feature = "user", derive(Clone, Copy))]
#[repr(C)]
pub struct RulePage<BinaryEndpoint: BinaryEndpointTrait> {
    pub entry_count: u16,
    _padding: u16,
    data: [u8; BYTES_PER_RULE_PAGE - 2 * size_of::<u16>()],
    _marker: PhantomData<BinaryEndpoint>,
}

pub type NameRulePage = RulePage<NameBinaryEndpoint>;
pub type Ipv4RulePage = RulePage<Ipv4BinaryEndpoint>;
pub type Ipv6RulePage = RulePage<Ipv6BinaryEndpoint>;
pub type AnyEndpointRulePage = RulePage<()>;

/// This function receives a pointer to the base of the rule page in `page_base` (cast to a
/// `PortTableEntry` for convenience) and returns a `PortTableEntry` at a given index. The
/// result is `Optional::None` if the entry is outside of the page.
pub fn port_table_entry(
    page_base: *const PortTableEntry,
    index: u16,
) -> Option<&'static PortTableEntry> {
    let mut index = index as usize;
    touch_usize(&mut index);
    if index >= BYTES_PER_RULE_PAGE / size_of::<PortTableEntry>() {
        None
    } else {
        unsafe { Some(&*page_base.add(index)) }
    }
}

impl<BinaryEndpoint: BinaryEndpointTrait> RulePage<BinaryEndpoint> {
    fn base_pointer<V>(&self) -> *const V {
        self as *const _ as *const V
    }

    pub fn entry_base_ptr(&self) -> *const BinaryRule<BinaryEndpoint> {
        &self.data as *const _ as *const _
    }

    pub fn entry_base_ptr_mut(&mut self) -> *mut BinaryRule<BinaryEndpoint> {
        &mut self.data as *mut _ as *mut _
    }

    pub fn entry_at_index(&self, index: u16) -> Option<&BinaryRule<BinaryEndpoint>> {
        let mut index = index as usize;
        touch_usize(&mut index);
        let entry_size = size_of::<BinaryRule<BinaryEndpoint>>();
        let max_entries = size_of_val(&self.data) / entry_size;
        if index >= max_entries {
            None
        } else {
            // We play "confuse the compiler" here. If `size_of::<T>()` is 16 (or probably any
            // other power of two), the compiler optimizes `entries[index]` to `ptr + index << 4`.
            // When accessing a field of `T`, it does not add the field offset but rather OR it
            // to the value, knowing that the low 4 bits must be 0. However, the verifier does
            // not handle OR operations correctly. It assumes that a value of 2016 OR 12 can be
            // up to 2044, probably lookin only at the low bits of the number 12.
            // In order to prevent this, we add an extra round of computation if the size is
            // a power of two, hoping that the compiler won't optimize it.
            Some(if entry_size & (entry_size - 1) == 0 {
                // entry_size is a power of two
                let ptr = &self.data[0] as *const u8;
                let ptr = unsafe { ptr.add(index * (entry_size - 1)) };
                let ptr = unsafe { ptr.add(index) } as *const BinaryRule<BinaryEndpoint>;
                unsafe { &*ptr }
            } else {
                unsafe { &*self.entry_base_ptr().add(index) }
            })
        }
    }

    pub fn find_matching_port_table<'a, Map: FilterTable<Self>>(
        map: &'a Map,
        page_count: u32,
        exe_pattern_id: ExePatternId,
        search_term: &BinaryEndpoint::SearchTerm,
    ) -> Option<(*const PortTableEntry, PortTableReference)> {
        let search_term = (exe_pattern_id, search_term);
        if let Some((page, _)) = Self::search_for_page(&search_term, map, page_count)
            && let (match_len, entry_index) = page.search_in_page(&search_term, page.entry_count())
            && let Some(entry) = page.entry_at_index(entry_index)
            // In case of IPv6 table, we return non-matching entries (the entry before the
            // insertion point). This may be the last address entry for the previous executable.
            && entry.exe_pattern == exe_pattern_id
            && match_len >= entry.endpoint.min_match_len()
            && !entry.port_table.represents_no_match()
        {
            Some((page.base_pointer::<PortTableEntry>(), entry.port_table))
        } else {
            None
        }
    }
}

impl<BinaryEndpoint: BinaryEndpointTrait> BinarySearchablePage for RulePage<BinaryEndpoint> {
    type SearchTerm<'a> = (ExePatternId, &'a BinaryEndpoint::SearchTerm);

    fn entry_count(&self) -> u16 {
        self.entry_count
    }

    fn compare<'a>(
        &self,
        search_term: &(ExePatternId, &'a BinaryEndpoint::SearchTerm),
        entry_index: u16,
    ) -> ExtOrder {
        if let Some(entry) = self.entry_at_index(entry_index) {
            match search_term.0.cmp(&entry.exe_pattern) {
                Ordering::Equal => {}
                other => return ExtOrder::from(other, 0),
            }
            entry.endpoint.compare(search_term.1, self).reverse()
        } else {
            ExtOrder::from(Ordering::Greater, 0)
        }
    }
}

// Wildcard paths are implemented in user-space by adding more entries to exe pattern map
/// Executable pair expressed in `NodeId`s of concrete executable files.
#[cfg_attr(feature = "user", derive(Copy))]
#[derive(Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ExeNodePair {
    pub primary: NodeId,
    pub via: Option<NodeId>,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RuleMatch {
    pub rule_id: RuleId,
    pub generation: u32,
}

#[cfg(feature = "user")]
unsafe impl Pod for NameRulePage {}
#[cfg(feature = "user")]
unsafe impl Pod for Ipv4RulePage {}
#[cfg(feature = "user")]
unsafe impl Pod for Ipv6RulePage {}
#[cfg(feature = "user")]
unsafe impl Pod for AnyEndpointRulePage {}
#[cfg(feature = "user")]
unsafe impl Pod for ExeNodePair {}
