// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

#![cfg(test)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::num::NonZeroU64;

use crate::StringId;
use crate::mock_strings_cache::MockStringsCache;
use crate::node_cache::{MAX_PATH_COMPONENTS, NodeCacheTrait, NodeId, PathNode, PathRep};

pub struct MockPath<'a> {
    strings_cache: &'a RefCell<MockStringsCache>,
    path: Vec<String>,
}

impl<'a> MockPath<'a> {
    pub fn new(path: &str, strings_cache: &'a RefCell<MockStringsCache>) -> Self {
        Self {
            strings_cache,
            // Skip the empty element at the root slash
            path: path.split('/').skip(1).map(|c| c.into()).collect(),
        }
    }
}

impl<'a> PathRep<()> for MockPath<'a> {
    fn name_id(&self, _context: *mut ()) -> crate::StringId {
        self.strings_cache
            .borrow_mut()
            .identifier_for_string(self.path.last().unwrap())
    }

    fn parent(&self) -> Option<MockPath<'a>> {
        if self.path.len() > 0 {
            let p: Vec<_> = self.path[0..(self.path.len() - 1)]
                .iter()
                .map(|c| c.clone())
                .collect();
            Some(Self {
                strings_cache: self.strings_cache,
                path: p,
            })
        } else {
            None
        }
    }
}

pub struct MockNodeCache {
    id_for_node: HashMap<PathNode, NodeId>,
    node_for_id: HashMap<NodeId, PathNode>,

    next_id: u64,
    buffer: [StringId; MAX_PATH_COMPONENTS],
}

impl MockNodeCache {
    pub fn new(strings_cache: &RefCell<MockStringsCache>) -> Self {
        let mut cache = Self {
            id_for_node: HashMap::new(),
            node_for_id: HashMap::new(),
            next_id: 2,
            buffer: [StringId::none(); _],
        };
        // We must make sure that all root objects are registered in the cache before
        // we allow anybody to work with it.
        let root_node_id = cache
            .root_node_id(&MockPath::new("/", strings_cache))
            .unwrap();
        let root_node = &PathNode {
            parent_id: None,
            name_id: strings_cache.borrow_mut().identifier_for_string(&"".into()),
        };
        cache.insert_node(&root_node, root_node_id);
        cache
    }
}

impl<'a> NodeCacheTrait<MockPath<'a>, ()> for MockNodeCache {
    fn root_node_id(&self, _root_path: &MockPath) -> Option<NodeId> {
        return Some(NodeId(NonZeroU64::new(1).unwrap()));
    }

    fn id_for_node(&self, node: &PathNode) -> Option<NodeId> {
        self.id_for_node.get(&node).map(|id| id.clone())
    }

    fn node_for_id(&self, node_id: NodeId) -> Option<PathNode> {
        self.node_for_id.get(&node_id).cloned()
    }

    fn string_id_buffer(&mut self) -> *mut [StringId; MAX_PATH_COMPONENTS] {
        &mut self.buffer as *mut _
    }

    fn name_id_context(&mut self) -> *mut () {
        &mut () // unused here
    }

    fn insert_node(&mut self, node: &PathNode, node_id: NodeId) -> bool {
        if self.id_for_node.contains_key(&node) {
            false
        } else {
            self.id_for_node.insert(node.clone(), node_id);
            self.node_for_id.insert(node_id, node.clone());
            true
        }
    }

    fn new_id(&mut self) -> NodeId {
        NodeId(NonZeroU64::new(self.next_id).unwrap())
    }

    fn consume_id(&mut self) {
        self.next_id += 1;
    }
}
