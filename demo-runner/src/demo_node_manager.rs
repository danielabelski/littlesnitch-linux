// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::demo_node_cache::{DemoNodeCache, DemoExecutable};
use aya::{maps::MapData, Ebpf};
use common::{
    bitset::BitSet,
    node_cache::{NodeCacheTrait, NodeId},
    FileId, NodeFeatures,
};
use mountpoints::mountpaths;
use nix::sys::stat::stat;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub struct DemoNodeManager {
    // The mount points in user space are root nodes for the kernel.
    pub root_nodes: aya::maps::HashMap<MapData, FileId, NodeId>,
    pub node_features: aya::maps::HashMap<MapData, NodeId, NodeFeatures>,

    // Note that tthe PID here is a thread PID and many entries may not be shown in a normal
    // `ps` output. This becomes important if we ever need to celan up lost entries.
    pub pid_to_node_id: aya::maps::HashMap<MapData, i32, NodeId>,

    pub node_cache: DemoNodeCache,
}

impl DemoNodeManager {
    pub fn new(ebpf: &mut Ebpf) -> Self {
        let raw_map = ebpf.take_map("ROOT_NODES").unwrap();
        let root_nodes = aya::maps::HashMap::<_, FileId, NodeId>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("NODE_FEATURES").unwrap();
        let node_features =
            aya::maps::HashMap::<_, NodeId, NodeFeatures>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("PID_TO_NODE_ID").unwrap();
        let pid_to_node_id = aya::maps::HashMap::<_, i32, NodeId>::try_from(raw_map).unwrap();

        let node_cache = DemoNodeCache::new(ebpf);
        Self {
            root_nodes,
            node_features,
            pid_to_node_id,
            node_cache,
        }
    }

    pub fn update_mounts(&mut self) {
        match mountpaths() {
            Ok(paths) => {
                let mut old_file_ids: HashSet<_> =
                    self.root_nodes.keys().filter_map(|r| r.ok()).collect();
                for path in paths {
                    let file_id = match file_id_for_path(&path) {
                        Ok(id) => id,
                        Err(error) => {
                            println!("cannot stat mountpoint: {:?}: {}", path, error);
                            continue;
                        }
                    };
                    let node_id = match self.node_cache.node_id_for_path(DemoExecutable(path.clone())) {
                        Some(id) => id,
                        None => {
                            println!("Could not obtain node ID for {:?}", path);
                            continue;
                        }
                    };
                    println!("adding mountpoint: {:?}", path);
                    _ = self.root_nodes.insert(file_id, node_id, 0);
                    old_file_ids.remove(&file_id);
                }
                // old_file_ids contains all mounts which are no longer valid. Remove them.
                for file_id in old_file_ids.iter() {
                    _ = self.root_nodes.remove(file_id);
                }
            }
            Err(error) => {
                println!("*** Error obtaining mount paths: {}", error);
            }
        }
    }

    pub fn add_node_features(&mut self, features: BitSet, paths: &[&str]) {
        for path_str in paths {
            match self.node_cache.node_id_for_path(DemoExecutable(PathBuf::from(path_str))) {
                Some(node_id) => {
                    let mut feat = self.node_features.get(&node_id, 0).unwrap_or_default();
                    feat.0 += features;
                    _ = self.node_features.insert(&node_id, &feat, 0);
                }
                None => println!("*** Error adding feature for node: {}", path_str),
            }
        }
    }

    pub fn dump_pid_cache(&self) {
        println!("--- PID Cache ---");
        for keyvalue in self.pid_to_node_id.iter() {
            let (pid, node_id) = match keyvalue {
                Ok((pid, node_id)) => (pid, node_id),
                Err(err) => {
                    println!("aborting iteration with error: {:?}", err);
                    break;
                }
            };
            let path = self.node_cache.path_for_node_id(node_id);
            println!("pid {:6}: {}", pid, path);
        }
    }
}

fn file_id_for_path(path: &Path) -> anyhow::Result<FileId> {
    let stat = stat(path)?;
    // `struct stat` has a different major/minor encoding in `st_dev` than the kernel
    let major = stat.st_dev >> 8;
    let minor = stat.st_dev & 0xff;
    Ok(FileId {
        inode_number: stat.st_ino,
        device: (major << 20) | minor, // convert to format used by kernel
    })
}
