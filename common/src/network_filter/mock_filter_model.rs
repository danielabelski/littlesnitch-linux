// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

#![cfg(test)]

use crate::{flow_types::Verdict, network_filter::{
    binary_searchable_page::BinarySearchablePage,
    blocklist_page::{Ipv4BlocklistPage, Ipv6BlocklistPage, NameBlocklistPage},
    filter_model::{FilterMetainfo, FilterModel, FilterTable, TableInfo},
    rule_page::*,
    rule_types::RuleId,
}};
use std::cell::Cell;

pub struct MockFilterModel {
    metainfo: Cell<FilterMetainfo>,
    name_blocklist: ArrayMap<NameBlocklistPage>,
    ipv4_blocklist: ArrayMap<Ipv4BlocklistPage>,
    ipv6_blocklist: ArrayMap<Ipv6BlocklistPage>,
    name_rules: ArrayMap<NameRulePage>,
    ipv4_rules: ArrayMap<Ipv4RulePage>,
    ipv6_rules: ArrayMap<Ipv6RulePage>,
    any_endpoint_rules: ArrayMap<AnyEndpointRulePage>,
}

impl MockFilterModel {
    pub fn new() -> Self {
        Self {
            metainfo: Cell::new(FilterMetainfo::new(Verdict::Allow)),
            name_blocklist: ArrayMap::new(),
            ipv4_blocklist: ArrayMap::new(),
            ipv6_blocklist: ArrayMap::new(),
            name_rules: ArrayMap::new(),
            ipv4_rules: ArrayMap::new(),
            ipv6_rules: ArrayMap::new(),
            any_endpoint_rules: ArrayMap::new(),
        }
    }

    fn compute_metainfo(&self) -> FilterMetainfo {
        FilterMetainfo {
            default_verdict: Verdict::Allow,
            name_blocklist: self.name_blocklist.table_info(),
            ipv4_blocklist: self.ipv4_blocklist.table_info(),
            ipv6_blocklist: self.ipv6_blocklist.table_info(),
            name_rules: self.name_rules.table_info(),
            ipv4_rules: self.ipv4_rules.table_info(),
            ipv6_rules: self.ipv6_rules.table_info(),
            any_endpoint_rules: self.any_endpoint_rules.table_info(),
            name_blocklist_rule_id: RuleId::new(12345, false),
            ip_blocklist_rule_id: RuleId::new(12343, false),
            rule_id_generation: 0,
            ruleset_generation: 0,
        }
    }
}

impl FilterModel for MockFilterModel {
    type NameBlocklist = ArrayMap<NameBlocklistPage>;
    type Ipv4Blocklist = ArrayMap<Ipv4BlocklistPage>;
    type Ipv6Blocklist = ArrayMap<Ipv6BlocklistPage>;
    type NameRules = ArrayMap<NameRulePage>;
    type Ipv4Rules = ArrayMap<Ipv4RulePage>;
    type Ipv6Rules = ArrayMap<Ipv6RulePage>;
    type AnyEndpointRules = ArrayMap<AnyEndpointRulePage>;

    fn metainfo(&self) -> Option<&FilterMetainfo> {
        self.metainfo.set(self.compute_metainfo());
        Some(unsafe { &*self.metainfo.as_ptr() })
    }

    fn name_blocklist(&self) -> &Self::NameBlocklist {
        &self.name_blocklist
    }

    fn ipv4_blocklist(&self) -> &Self::Ipv4Blocklist {
        &self.ipv4_blocklist
    }

    fn ipv6_blocklist(&self) -> &Self::Ipv6Blocklist {
        &self.ipv6_blocklist
    }

    fn name_rules(&self) -> &Self::NameRules {
        &self.name_rules
    }

    fn ipv4_rules(&self) -> &Self::Ipv4Rules {
        &self.ipv4_rules
    }

    fn ipv6_rules(&self) -> &Self::Ipv6Rules {
        &self.ipv6_rules
    }

    fn any_endpoint_rules(&self) -> &Self::AnyEndpointRules {
        &self.any_endpoint_rules
    }
}

pub struct ArrayMap<PageType> {
    content: Cell<Vec<PageType>>,
}

impl<T: BinarySearchablePage> ArrayMap<T> {
    pub fn new() -> Self {
        Self { content: Cell::new(Vec::new()) }
    }

    pub fn set_pages(&self, pages: Vec<T>) {
        self.content.set(pages);
    }

    fn table_info(&self) -> TableInfo {
        let vec_ref = unsafe { &*self.content.as_ptr() };
        TableInfo {
            page_count: vec_ref.len() as _,
            last_page_entry_count: vec_ref.last().map(|e| e.entry_count()).unwrap_or(0),
            generation: 0,
        }
    }
}

impl<T: BinarySearchablePage> FilterTable<T> for ArrayMap<T> {
    fn get(&self, index: u32) -> Option<&T> {
        let index = index as usize;
        let vec_ref = unsafe { &*self.content.as_ptr() };
        if index < vec_ref.len() {
            Some(&vec_ref[index])
        } else {
            None
        }
    }
}
