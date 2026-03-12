// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::demo_strings_cache::DemoStringsCache;
use aya::{Ebpf, maps::MapData};
use common::{
    StringId,
    bpf_string::BpfString,
    node_cache::{MAX_PATH_COMPONENTS, NodeCacheTrait, NodeId, PathNode, PathRep},
};
use std::{
    collections::HashMap, num::NonZeroU64, os::unix::ffi::OsStrExt, path::PathBuf, sync::Arc,
};

const BPF_NOEXIST: u64 = 1;

pub struct DemoNodeCache {
    // Garbage collection of nodes is missing in this demo code. Nodes are created by the eBPF
    // program, but never removed anywhere. That's OK as long as the number of executables ever
    // started has an upper bound which fits into the node cache. For production code, a
    // garbage collection should remove all nodes which are no longer referenced by a running
    // program, a mount point or have special features attached.
    pub node_id_for_node: aya::maps::HashMap<MapData, PathNode, NodeId>,
    pub node_for_node_id: aya::maps::HashMap<MapData, NodeId, PathNode>,
    node_id_counter: u64,
    root_node_id: Option<NodeId>,

    pub strings_cache: DemoStringsCache,

    pub executables_by_node_id: HashMap<NodeId, Arc<DemoExecutable>>,

    string_id_buffer: [StringId; MAX_PATH_COMPONENTS],
}

pub struct DemoExecutable(pub PathBuf);

impl DemoNodeCache {
    pub fn new(ebpf: &mut Ebpf) -> Self {
        let raw_map = ebpf.take_map("NODE_ID_FOR_NODE").unwrap();
        let node_id_for_node =
            aya::maps::HashMap::<_, PathNode, NodeId>::try_from(raw_map).unwrap();
        let raw_map = ebpf.take_map("NODE_FOR_NODE_ID").unwrap();
        let node_for_node_id =
            aya::maps::HashMap::<_, NodeId, PathNode>::try_from(raw_map).unwrap();

        let strings_cache = DemoStringsCache::new(ebpf);
        let mut instance = Self {
            node_id_for_node,
            node_for_node_id,
            node_id_counter: 1, // nonzero int
            root_node_id: None,
            strings_cache,
            executables_by_node_id: HashMap::new(),
            string_id_buffer: [StringId::none(); _],
        };
        let node_id = instance.new_id();
        instance.consume_id();
        let root_name_id =
            instance.strings_cache.identifier_for_string(&BpfString::from_bytes(b""));
        let root_node = PathNode { parent_id: None, name_id: root_name_id };
        instance.insert_node(&root_node, node_id);
        instance.root_node_id = Some(node_id);
        instance
    }

    pub fn path_for_node_id(&self, node_id: NodeId) -> String {
        let mut path_components = Vec::<String>::new();
        self.enumerate_path(node_id, &mut |string_id| {
            path_components.push(self.strings_cache.string_for_identifier(string_id));
        });
        path_components.reverse();
        path_components.join("/")
    }

    pub fn executable_for_node_id(&mut self, node_id: NodeId) -> Arc<DemoExecutable> {
        if let Some(executable) = self.executables_by_node_id.get(&node_id) {
            return executable.clone();
        }
        let path = self.path_for_node_id(node_id);
        let executable = Arc::new(DemoExecutable(PathBuf::from(path)));
        self.executables_by_node_id.insert(node_id, executable.clone());
        executable
    }
}

impl NodeCacheTrait<DemoExecutable, DemoStringsCache> for DemoNodeCache {
    fn root_node_id(&self, root_path: &DemoExecutable) -> Option<NodeId> {
        // in our view of the world, there is only one root: "/". All other nodes have parents.
        debug_assert!(root_path.parent().is_none());
        self.root_node_id
    }

    fn id_for_node(&self, node: &PathNode) -> Option<NodeId> {
        self.node_id_for_node.get(&node, 0).ok()
    }

    fn node_for_id(&self, node_id: NodeId) -> Option<PathNode> {
        self.node_for_node_id.get(&node_id, 0).ok()
    }

    fn string_id_buffer(&mut self) -> *mut [StringId; MAX_PATH_COMPONENTS] {
        &mut self.string_id_buffer
    }

    fn name_id_context(&mut self) -> *mut DemoStringsCache {
        &mut self.strings_cache
    }

    fn insert_node(&mut self, node: &PathNode, node_id: NodeId) -> bool {
        if self.node_id_for_node.insert(node, node_id, BPF_NOEXIST).is_err() {
            return false;
        }
        _ = self.node_for_node_id.insert(node_id, node, 0);
        true
    }

    fn new_id(&mut self) -> NodeId {
        // unique IDs have the CPU index in the lower 16 bits. User-space counts as CPU 0.
        NodeId(NonZeroU64::new(self.node_id_counter << 16).unwrap())
    }

    fn consume_id(&mut self) {
        self.node_id_counter += 1;
    }
}

impl PathRep<DemoStringsCache> for DemoExecutable {
    fn name_id(&self, strings_cache: *mut DemoStringsCache) -> StringId {
        if let Some(name) = self.0.file_name() {
            let strings_cache = unsafe { &mut *strings_cache };
            let bpf_string = BpfString::from_bytes(name.as_bytes());
            strings_cache.identifier_for_string(&bpf_string)
        } else {
            StringId::none()
        }
    }

    fn parent(&self) -> Option<Self> {
        let parent_path = self.0.parent()?;
        Some(Self(PathBuf::from(parent_path)))
    }
}
