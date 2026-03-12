// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use core::mem::transmute;

use aya_ebpf::{
    macros::map,
    maps::{Array, HashMap, PerCpuArray},
};
use common::network_filter::{
    blocklist_page::{Ipv4BlocklistPage, Ipv6BlocklistPage, NameBlocklistPage},
    filter_model::{FilterMetainfo, FilterModel, FilterTable},
    rule_page::*,
    rule_types::ExePatternId,
};

#[map]
pub static NAME_BLOCKLIST: Array<NameBlocklistPage> = Array::with_max_entries(30000, 0);

#[map]
pub static IPV4_BLOCKLIST: Array<Ipv4BlocklistPage> = Array::with_max_entries(20000, 0);

#[map]
pub static IPV6_BLOCKLIST: Array<Ipv6BlocklistPage> = Array::with_max_entries(50000, 0);

#[map]
pub static MATCH_TABLE_METAINFO: PerCpuArray<FilterMetainfo> = PerCpuArray::with_max_entries(1, 0);

#[map]
pub static NAME_RULES: Array<NameRulePage> = Array::with_max_entries(10000, 0);

#[map]
pub static IPV4_RULES: Array<Ipv4RulePage> = Array::with_max_entries(10000, 0);

#[map]
pub static IPV6_RULES: Array<Ipv6RulePage> = Array::with_max_entries(10000, 0);

#[map]
pub static ANY_ENDPOINT_RULES: Array<AnyEndpointRulePage> = Array::with_max_entries(256, 0);

#[map]
pub static EXE_PATTERNS: HashMap<ExeNodePair, ExePatternId> = HashMap::with_max_entries(10000, 0);

pub struct NameBlocklist {}
impl FilterTable<NameBlocklistPage> for NameBlocklist {
    fn get(&self, index: u32) -> Option<&NameBlocklistPage> {
        NAME_BLOCKLIST.get(index)
    }
}

pub struct Ipv4Blocklist {}
impl FilterTable<Ipv4BlocklistPage> for Ipv4Blocklist {
    fn get(&self, index: u32) -> Option<&Ipv4BlocklistPage> {
        IPV4_BLOCKLIST.get(index)
    }
}

pub struct Ipv6Blocklist {}
impl FilterTable<Ipv6BlocklistPage> for Ipv6Blocklist {
    fn get(&self, index: u32) -> Option<&Ipv6BlocklistPage> {
        IPV6_BLOCKLIST.get(index)
    }
}

pub struct NameRules {}
impl FilterTable<NameRulePage> for NameRules {
    fn get(&self, index: u32) -> Option<&NameRulePage> {
        NAME_RULES.get(index)
    }
}

pub struct Ipv4Rules {}
impl FilterTable<Ipv4RulePage> for Ipv4Rules {
    fn get(&self, index: u32) -> Option<&Ipv4RulePage> {
        IPV4_RULES.get(index)
    }
}

pub struct Ipv6Rules {}
impl FilterTable<Ipv6RulePage> for Ipv6Rules {
    fn get(&self, index: u32) -> Option<&Ipv6RulePage> {
        IPV6_RULES.get(index)
    }
}

pub struct AnyEndpointRules {}
impl FilterTable<AnyEndpointRulePage> for AnyEndpointRules {
    fn get(&self, index: u32) -> Option<&AnyEndpointRulePage> {
        ANY_ENDPOINT_RULES.get(index)
    }
}

pub struct KernelFilterModel {
    metainfo: FilterMetainfo,

    // all remaining types have a size of zero
    name_blocklist: NameBlocklist,
    ipv4_blocklist: Ipv4Blocklist,
    ipv6_blocklist: Ipv6Blocklist,

    name_rules: NameRules,
    ipv4_rules: Ipv4Rules,
    ipv6_rules: Ipv6Rules,
    any_endpoint_rules: AnyEndpointRules,
}

impl KernelFilterModel {
    pub fn shared() -> Option<&'static KernelFilterModel> {
        MATCH_TABLE_METAINFO.get(0).map(|m| unsafe { transmute(m) })
    }
}

impl FilterModel for KernelFilterModel {
    type NameBlocklist = NameBlocklist;
    type Ipv4Blocklist = Ipv4Blocklist;
    type Ipv6Blocklist = Ipv6Blocklist;
    type NameRules = NameRules;
    type Ipv4Rules = Ipv4Rules;
    type Ipv6Rules = Ipv6Rules;
    type AnyEndpointRules = AnyEndpointRules;

    fn metainfo(&self) -> Option<&FilterMetainfo> {
        Some(&self.metainfo)
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
