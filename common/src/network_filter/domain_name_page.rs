// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    ByteAtOffset, DOMAIN_SEP,
    ext_order::ExtOrder,
    bpf_string::BpfString,
    repeat::{LoopReturn, repeat},
};
use core::{cmp::Ordering, ops::Range};

/// This is a trait to be implemented by NameRulePage and NameBlocklistPage. It provides comparison
/// of names which are stored outside of list entries.
pub trait DomainNamePage: ByteAtOffset + Sized {
    /// Returns Ordering::Greater if search_term > entry, magnitude is length of substring match
    fn compare_domain_name(
        &self,
        search_term: &BpfString,
        entry_byte_range: Range<usize>,
        domain_compare: bool,
    ) -> ExtOrder {
        let mut context = CompareContext {
            page: self,
            search_term,
            entry_start: entry_byte_range.start,
            entry_index: entry_byte_range.end,
            search_term_index: search_term.len(),
            result: Ordering::Equal,
        };
        repeat(255, compare_inner, &mut context);
        let match_len = search_term.len() - context.search_term_index;
        if context.result != Ordering::Equal {
            return ExtOrder::from(context.result, match_len);
        }
        // All common elements compared equal so far. Check for domain match first:
        if domain_compare
            && context.search_term_index != 0
            && search_term.byte_at_offset(context.search_term_index - 1) == DOMAIN_SEP
        {
            ExtOrder::equal()
        } else if context.search_term_index != 0 {
            // entry did end before name, so name is greater:
            ExtOrder::greater(match_len)
        } else if context.entry_index != context.entry_start {
            // name did end before entry, so name is less
            ExtOrder::less(match_len)
        } else {
            // both strings are equal up to their end
            ExtOrder::equal()
        }
    }
}

struct CompareContext<'a, Page: ByteAtOffset> {
    page: &'a Page,
    search_term: &'a BpfString,
    entry_start: usize,       // limit, compare down to this index
    entry_index: usize,       // current compare index
    search_term_index: usize, // current compare index in BpfString
    result: Ordering,
}

extern "C" fn compare_inner<Page: ByteAtOffset>(
    _index: u64,
    context: &mut CompareContext<Page>,
) -> LoopReturn {
    if context.search_term_index == 0 || context.entry_index <= context.entry_start {
        return LoopReturn::LoopBreak;
    }
    context.entry_index -= 1;
    context.search_term_index -= 1;
    let entry_byte = context.page.byte_at_offset(context.entry_index);
    let name_byte = context.search_term.byte_at_offset(context.search_term_index);
    let ord = name_byte.cmp(&entry_byte);
    if ord != Ordering::Equal {
        context.result = ord;
        return LoopReturn::LoopBreak;
    }
    LoopReturn::LoopContinue
}
