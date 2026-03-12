// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use aya::{
    Ebpf,
    maps::{MapData, PerCpuValues},
    util::nr_cpus,
};
use common::network_filter::{
    blocklist_page::*, filter_model::FilterMetainfo, rule_page::*, rule_types::ExePatternId,
};

/// eBPF maps carrying network filter data. There are maps for blocklists and particular
/// aspects of rules. Not all maps are used in this demo project.
#[allow(dead_code)]
pub struct DemoFilterMaps {
    pub metainfo: aya::maps::PerCpuArray<MapData, FilterMetainfo>,

    pub name_blocklist: aya::maps::Array<MapData, NameBlocklistPage>,
    pub ipv4_blocklist: aya::maps::Array<MapData, Ipv4BlocklistPage>,
    pub ipv6_blocklist: aya::maps::Array<MapData, Ipv6BlocklistPage>,

    pub name_rules: aya::maps::Array<MapData, NameRulePage>,
    pub ipv4_rules: aya::maps::Array<MapData, Ipv4RulePage>,
    pub ipv6_rules: aya::maps::Array<MapData, Ipv6RulePage>,
    pub any_endpoint_rules: aya::maps::Array<MapData, AnyEndpointRulePage>,

    pub exe_patterns: aya::maps::HashMap<MapData, ExeNodePair, ExePatternId>,
}

impl DemoFilterMaps {
    pub fn new(ebpf: &mut Ebpf) -> Self {
        let raw_map = ebpf.take_map("MATCH_TABLE_METAINFO").unwrap();
        let metainfo = aya::maps::PerCpuArray::<_, FilterMetainfo>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("NAME_BLOCKLIST").unwrap();
        let name_blocklist = aya::maps::Array::<_, NameBlocklistPage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("IPV4_BLOCKLIST").unwrap();
        let ipv4_blocklist = aya::maps::Array::<_, Ipv4BlocklistPage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("IPV6_BLOCKLIST").unwrap();
        let ipv6_blocklist = aya::maps::Array::<_, Ipv6BlocklistPage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("NAME_RULES").unwrap();
        let name_rules = aya::maps::Array::<_, NameRulePage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("IPV4_RULES").unwrap();
        let ipv4_rules = aya::maps::Array::<_, Ipv4RulePage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("IPV6_RULES").unwrap();
        let ipv6_rules = aya::maps::Array::<_, Ipv6RulePage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("ANY_ENDPOINT_RULES").unwrap();
        let any_endpoint_rules =
            aya::maps::Array::<_, AnyEndpointRulePage>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("EXE_PATTERNS").unwrap();
        let exe_patterns =
            aya::maps::HashMap::<_, ExeNodePair, ExePatternId>::try_from(raw_map).unwrap();

        Self {
            metainfo,
            name_blocklist,
            ipv4_blocklist,
            ipv6_blocklist,
            name_rules,
            ipv4_rules,
            ipv6_rules,
            any_endpoint_rules,
            exe_patterns,
        }
    }

    pub fn write_metainfo(&mut self, metainfo: &FilterMetainfo) {
        let cpu_count = nr_cpus().unwrap();
        let value = PerCpuValues::try_from(vec![metainfo.clone(); cpu_count]).unwrap();
        _ = self.metainfo.set(0, value, 0);
    }
}
