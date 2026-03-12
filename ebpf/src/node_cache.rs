// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    co_re::*,
    context::{StaticBuffers, StringAndZero},
    strings_cache::identifier_for_string,
    unique_id::{Purpose, UniqueId},
};
use aya_ebpf::{
    bindings::BPF_NOEXIST, cty::c_void, helpers::generated::bpf_probe_read_kernel, macros::map,
    maps::HashMap,
};
use common::{
    FileId, StringId,
    node_cache::{MAX_PATH_COMPONENTS, NodeCacheTrait, NodeId, PathNode, PathRep},
};
use core::ptr;

#[map]
static NODE_ID_FOR_NODE: HashMap<PathNode, NodeId> = HashMap::with_max_entries(65536, 0);
#[map]
static NODE_FOR_NODE_ID: HashMap<NodeId, PathNode> = HashMap::with_max_entries(65536, 0);

#[map]
static ROOT_NODES: HashMap<FileId, NodeId> = HashMap::with_max_entries(8192, 0);

pub struct NodeCache {
    buffers: *mut StaticBuffers,
    unique_id: Option<UniqueId>,
}

impl NodeCache {
    pub fn new(buffers: *mut StaticBuffers) -> Self {
        Self { buffers, unique_id: None }
    }
}

impl NodeCacheTrait<Path, StringAndZero> for NodeCache {
    fn root_node_id(&self, root_path: &Path) -> Option<NodeId> {
        let file_id = root_path.dentry.file_id();
        unsafe { ROOT_NODES.get(&file_id).cloned() }
    }

    fn id_for_node(&self, node: &PathNode) -> Option<NodeId> {
        unsafe { NODE_ID_FOR_NODE.get(node).cloned() }
    }

    fn node_for_id(&self, node_id: NodeId) -> Option<PathNode> {
        unsafe { NODE_FOR_NODE_ID.get(&node_id).cloned() }
    }

    fn string_id_buffer(&mut self) -> *mut [StringId; MAX_PATH_COMPONENTS] {
        unsafe { &mut (*self.buffers).string_ids }
    }

    fn name_id_context(&mut self) -> *mut StringAndZero {
        unsafe { &mut (*self.buffers).string }
    }

    fn insert_node(&mut self, node: &PathNode, node_id: NodeId) -> bool {
        // BPF_NOEXIST means that we don't want to overwrite existing entries
        if !NODE_ID_FOR_NODE.insert(node, node_id, BPF_NOEXIST as _).is_ok() {
            return false;
        }
        _ = NODE_FOR_NODE_ID.insert(node_id, node, 0);
        true
    }

    fn new_id(&mut self) -> NodeId {
        let unique_id = UniqueId::new(Purpose::NodeId);
        let node_id = NodeId(unique_id.get());
        self.unique_id = Some(unique_id);
        node_id
    }

    fn consume_id(&mut self) {
        if let Some(mut unique_id) = self.unique_id.take() {
            unique_id.consume();
        }
    }
}

// We currently obtain the path from a `struct path`, which represents a path within the mounted
// file system. In order to get absolute paths, we maintain a list of mount points (`ROOT_NODES`)
// and prepend the file system's mount point to the path's root.
// We cannot reconstruct the absolute path because we don't have the mount point's dentry and
// mnt_root. We only have `struct vfsmnt``, not `struct mnt`. `struct mnt` should be at a fixed
// offset from `struct vfsmnt` (see Linux implementation of `prepend_path()` in `d_path.c``),
// but that's hard to compute with BTF relocation. See the comment below for comments on how
// a path is constructed.

pub struct Path {
    // References are static for the time our program runs.
    pub dentry: &'static dentry,
}

impl Path {
    pub fn new(path: &path) -> Self {
        unsafe { Self { dentry: &*path_dentry(path as _) } }
    }
}

// This implementation of PathRep defines by what path an executable is identified. We choose
// to work with `struct dentry` for executable identification, not with path strings which
// would be available in `sched_process_exec()`, because they have all symlinks resolved.
// Although we work with `realpath()` paths here (symlinks resolved), there may still be
// ambiguities, in particular with bind mounts where the same directory is made available under
// different paths. Bind mounts are used in various places in Linux, e.g. in order to create
// a secure, isolated environment for untrusted processes or containers, so this is not an exotic
// feature.
// When an executable is visible multiple times via bind mounts, we prefer to report it at its
// "main" path. This is also easier because we otherwise would have to follow bind mounts from
// user space fast enough to have the mount point available in the `ROOT_NODES` map before any
// process is exec'd on the mount.
// By ignoring the `mnt.mnt_root` of Linux `struct path`, we always iterate up to the file system's
// root, which should be the "main" mount point of the device.

impl PathRep<StringAndZero> for Path {
    fn name_id(&self, buffer: *mut StringAndZero) -> StringId {
        let buffer = unsafe { &mut *buffer };
        buffer.string.clear(buffer.zero);
        buffer.string.update(|bytes| unsafe {
            let len = (&*dentry_name(self.dentry as _))
                .__bindgen_anon_1
                .__bindgen_anon_1
                .len
                .min(bytes.len() as _);
            let r = bpf_probe_read_kernel(
                bytes as *mut u8 as *mut c_void,
                len,
                (&*dentry_name(self.dentry as _)).name as *const c_void,
            );
            if r < 0 { 0 } else { len as _ }
        });
        identifier_for_string(&buffer.string)
    }

    fn parent(&self) -> Option<Self> {
        // We could stop parent iteration at the next mount point here, but rather iterate up
        // to the file system's root where no more parent nodes are available.
        if let Some(parent) = unsafe { dentry_parent(self.dentry as *const _).as_ref() }
            && !ptr::eq(parent, self.dentry)
        {
            Some(Path { dentry: parent })
        } else {
            None
        }
    }
}

trait FileIdRep {
    fn file_id(&self) -> FileId;
}

impl FileIdRep for &'static dentry {
    fn file_id(&self) -> FileId {
        let dentry = *self;
        unsafe {
            FileId {
                inode_number: dentry_ino(dentry as _),
                device: dentry_dev(dentry as _) as _,
            }
        }
    }
}
