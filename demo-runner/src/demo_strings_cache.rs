// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use aya::{Ebpf, maps::MapData};
use common::{StringId, bpf_string::BpfString};

pub struct DemoStringsCache {
    // The strings cache need garbage collection, which is not implemented in this demo code.
    // Strings originate in path nodes and from the DNS cache. The eBPF program only ever adds
    // new strings but never removes them. Use all path nodes and the DNS cache to find
    // unused strings and remove them in regular intervals.
    pub string_to_identifier: aya::maps::HashMap<MapData, BpfString, StringId>,
    pub identifier_to_string: aya::maps::HashMap<MapData, StringId, BpfString>,

    pub string_index_counter: u64,
}

// BPF map flag, declaration missing in Aya.
const BPF_NOEXIST: u64 = 1;

impl DemoStringsCache {
    pub fn new(ebpf: &mut Ebpf) -> Self {
        let raw_map = ebpf.take_map("STRING_TO_IDENTIFIER").unwrap();
        let string_to_identifier =
            aya::maps::HashMap::<_, BpfString, StringId>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("IDENTIFIER_TO_STRING").unwrap();
        let identifier_to_string =
            aya::maps::HashMap::<_, StringId, BpfString>::try_from(raw_map).unwrap();

        Self {
            string_to_identifier,
            identifier_to_string,
            string_index_counter: 1,
        }
    }

    pub fn identifier_for_string(&mut self, string: &BpfString) -> StringId {
        let cpu_index = 0; // user-space has CPU-Index 0
        let proposed_identifier = StringId(cpu_index + (self.string_index_counter << 16));
        if self
            .string_to_identifier
            .insert(string, proposed_identifier, BPF_NOEXIST)
            .is_ok()
        {
            _ = self.identifier_to_string.insert(proposed_identifier, string, 0);
            self.string_index_counter += 1;
            proposed_identifier
        } else {
            self.string_to_identifier.get(string, 0).unwrap()
        }
    }

    pub fn string_for_identifier(&self, id: StringId) -> String {
        let string = self.identifier_to_string.get(&id, 0).unwrap_or_default();
        string.as_str().into()
    }

    #[allow(dead_code)]
    pub fn dump_cache(&self) {
        println!("--- Strings Cache ---");
        for keyvalue in self.string_to_identifier.iter() {
            match keyvalue {
                Ok((key, value)) => {
                    println!("    {}", format_cache_entry(&key, value));
                }
                Err(err) => {
                    println!("aborting iteration with error: {:?}", err);
                    break;
                }
            }
        }
        println!("--- End Strings Cache ---");
    }
}

fn format_cache_entry(string: &BpfString, identifier: StringId) -> String {
    let str = string.as_str();
    let index = identifier.0 >> 16;
    let cpu = identifier.0 & 0xffff;
    format!("{str}: {cpu}/{index}")
}
