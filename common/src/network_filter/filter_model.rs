// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    flow_types::Verdict,
    network_filter::{
        blocklist_page::{Ipv4BlocklistPage, Ipv6BlocklistPage, NameBlocklistPage},
        rule_page::*,
        rule_types::RuleId,
    },
};

#[cfg(feature = "user")]
use aya::Pod;

pub trait FilterTable<Page> {
    fn get(&self, index: u32) -> Option<&Page>;
}

/// This trait defines the data model used by the network filter. In the kernel, it is backed
/// by eBPF maps. In user space, it can be backed by Rust Vec and HashMap types.
pub trait FilterModel {
    type NameBlocklist: FilterTable<NameBlocklistPage>;
    type Ipv4Blocklist: FilterTable<Ipv4BlocklistPage>;
    type Ipv6Blocklist: FilterTable<Ipv6BlocklistPage>;
    type NameRules: FilterTable<NameRulePage>;
    type Ipv4Rules: FilterTable<Ipv4RulePage>;
    type Ipv6Rules: FilterTable<Ipv6RulePage>;
    type AnyEndpointRules: FilterTable<AnyEndpointRulePage>;

    fn metainfo(&self) -> Option<&FilterMetainfo>;

    fn name_blocklist(&self) -> &Self::NameBlocklist;
    fn ipv4_blocklist(&self) -> &Self::Ipv4Blocklist;
    fn ipv6_blocklist(&self) -> &Self::Ipv6Blocklist;

    fn name_rules(&self) -> &Self::NameRules;
    fn ipv4_rules(&self) -> &Self::Ipv4Rules;
    fn ipv6_rules(&self) -> &Self::Ipv6Rules;
    fn any_endpoint_rules(&self) -> &Self::AnyEndpointRules;
}

/// Information for an Array map which is used as a match table for rules or blocklists.
/// We need to communicate the number of populated indexes (`page_count`) and a counter
/// which is incremented when the table is modified (`generation`). Since we implement `Pod`,
/// we make all padding explicit so that it is initialized and copied.
/// For tables with a fixed number of entries per page, the page itself does not provide an
/// entry count. In this case we use `last_page_entry_count` to communicate the number of
/// entries in the last, partially filled page.
#[derive(Copy, Clone)]
#[repr(C)]
pub struct TableInfo {
    pub page_count: u32,
    pub last_page_entry_count: u16,
    pub generation: u16,
}

impl TableInfo {
    pub const fn new() -> Self {
        Self {
            page_count: 0,
            last_page_entry_count: 0,
            generation: 0,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct FilterMetainfo {
    pub default_verdict: Verdict,
    pub name_blocklist: TableInfo,
    pub ipv4_blocklist: TableInfo,
    pub ipv6_blocklist: TableInfo,
    pub name_rules: TableInfo,
    pub ipv4_rules: TableInfo,
    pub ipv6_rules: TableInfo,
    pub any_endpoint_rules: TableInfo,

    pub name_blocklist_rule_id: RuleId,
    pub ip_blocklist_rule_id: RuleId,

    // The `rule_id_generation` is incremented each time rule_ids are re-assigned.
    pub rule_id_generation: u32,

    // The `ruleset_generation` is incremented each time a rule or blocklist entry
    // changes. It can be used to validate cached verdicts.
    pub ruleset_generation: u64,
}

impl FilterMetainfo {
    pub const fn new(default_verdict: Verdict) -> Self {
        Self {
            default_verdict,
            name_blocklist: TableInfo::new(),
            ipv4_blocklist: TableInfo::new(),
            ipv6_blocklist: TableInfo::new(),
            name_rules: TableInfo::new(),
            ipv4_rules: TableInfo::new(),
            ipv6_rules: TableInfo::new(),
            any_endpoint_rules: TableInfo::new(),
            name_blocklist_rule_id: RuleId::ANY,
            ip_blocklist_rule_id: RuleId::ANY,
            rule_id_generation: 0,
            ruleset_generation: 0,
        }
    }
}

#[cfg(feature = "user")]
unsafe impl Pod for FilterMetainfo {}
