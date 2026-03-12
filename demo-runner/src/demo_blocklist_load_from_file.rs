// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use std::{collections::HashMap, fs, path::Path};

use crate::{demo_blocklist::DemoBlocklistEntry, demo_filter_maps::DemoFilterMaps};

impl DemoFilterMaps {
    /// Returns the number of pages used for blocklists
    pub fn load_blocklists(&mut self, lists: &[(&Path, bool)]) -> usize {
        let mut merger = BlocklistMerger::new();
        let mut all_files = Vec::<(Vec<u8>, bool)>::new();
        for (path, is_domain) in lists {
            let data = match fs::read(*path) {
                Ok(data) => data,
                Err(error) => {
                    println!("Error reading file {:?}: {}", *path, error);
                    continue;
                }
            };
            all_files.push((data, *is_domain));
        }
        // We now have all the data in memory, we can work with slices now.
        for (data, is_domain) in all_files.iter() {
            let entry_iterator = data.split(|b| *b == b'\n').filter_map(|line| {
                if let Some(start) = line.iter().position(|&b| !b.is_ascii_whitespace()) {
                    if line[start] == b'#' {
                        None
                    } else {
                        let end = line.iter().rposition(|&b| !b.is_ascii_whitespace()).unwrap() + 1;
                        Some(&line[start..end])
                    }
                } else {
                    None
                }
            });
            for name in entry_iterator {
                merger.add_entry(name, *is_domain);
            }
        }
        self.set_name_blocklist_entries(merger.merged_entries())
    }
}

struct BlocklistMerger<'a> {
    pub entries: HashMap<&'a [u8], bool>,
}

impl<'a> BlocklistMerger<'a> {
    fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    fn add_entry(&mut self, name: &'a [u8], is_domain: bool) {
        let mutable_value = self.entries.entry(name).or_insert(is_domain);
        // If there are two entries, one for host and one for domain, merge as domain entry
        *mutable_value |= is_domain;
    }

    fn merged_entries(self) -> Vec<DemoBlocklistEntry<'a>> {
        self.entries
            .into_iter()
            .map(|(name, is_domain)| DemoBlocklistEntry { name, is_domain })
            .collect()
    }
}
