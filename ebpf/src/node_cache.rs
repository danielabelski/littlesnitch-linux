// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    co_re::*,
    context::StaticBuffers,
    strings_cache::identifier_for_string,
    unique_id::{Purpose, UniqueId},
};
use aya_ebpf::{
    bindings::BPF_NOEXIST, cty::c_void, helpers::generated::bpf_probe_read_kernel, macros::map,
    maps::HashMap,
};
use common::{
    StringId,
    bpf_string::BpfString,
    node_cache::{MAX_PATH_COMPONENTS, NodeCacheTrait, NodeId, PathNode, PathRep},
};
use core::{mem::MaybeUninit, ptr};

#[map]
static NODE_ID_FOR_NODE: HashMap<PathNode, NodeId> = HashMap::with_max_entries(65536, 0);
#[map]
static NODE_FOR_NODE_ID: HashMap<NodeId, PathNode> = HashMap::with_max_entries(65536, 0);

pub struct NodeCache {
    buffers: *mut StaticBuffers,
    unique_id: Option<UniqueId>,
}

impl NodeCache {
    pub fn new(buffers: *mut StaticBuffers) -> Self {
        Self { buffers, unique_id: None }
    }
}

impl NodeCacheTrait<Path, BpfString> for NodeCache {
    fn root_node_id(&self, _root_path: &Path) -> Option<NodeId> {
        Some(NodeId::ROOT_ID)
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

    fn name_id_context(&mut self) -> *mut BpfString {
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
// file system. We reconstruct the absolte path by continuing at the mount point's dentry
// within the parent mount. This requires access to `struct mount` where we only have
// `struct vfsmount` from `struct path`. `struct vfsmount` is embedded in `struct mount`, and
// the kernel uses `container_of()` to get from `struct vfsmount` to `struct mount` (see
// Linux implementation of `prepend_path()` in `d_path.c`). We do the same here by obtaining
// the struct offsets from the co-re C-module.

#[derive(Clone)]
pub struct Path {
    // References are static for the time our program runs.
    pub dentry: &'static dentry,
    pub mnt: &'static vfsmount,
}

impl Path {
    pub fn new(path: &path) -> Self {
        unsafe {
            Self {
                dentry: &*path_dentry(path as _),
                mnt: &*path_mnt(path as _),
            }
        }
    }

    pub fn is_root(&self) -> bool {
        let root_dentry = vfsmount_root(self.mnt);
        ptr::eq(self.dentry, root_dentry)
    }
}

// This implementation of PathRep defines by what path an executable is identified. We choose
// to work with `struct dentry` for executable identification, not with path strings which
// would be available in `sched_process_exec()`, because dentry represents a "realpath".
// We iterate parent nodes up to the mount's root and then mounts up to the absolute root.
// We cannot use CO-RE access to struct fields here because there is no CO-RE compliant way to
// get to `struct mount`. Once we go through `struct mount`, pointers are no longer CO-RE tagged.
// This happen at the moment we cross a mount point. We therefore use bpf_probe_read_kernel()
// everywhere to access struct fields. The offsets of the struct fields are obtained in the co-re
// C-module.

impl PathRep<BpfString> for Path {
    fn name_id(&self, buffer: *mut BpfString) -> StringId {
        let buffer = unsafe { &mut *buffer };
        buffer.clear();
        qstr_string(dentry_name(self.dentry as _), buffer);
        identifier_for_string(buffer)
    }

    fn parent(&self) -> Option<Self> {
        let parent_dentry = unsafe { dentry_parent(self.dentry as *const _).as_ref() };
        let Some(parent_dentry) = parent_dentry else {
            return None;
        };
        let original_parent = Path { dentry: parent_dentry, mnt: self.mnt };
        let mut path = original_parent.clone();
        let mut i = 0;
        while path.is_root() && i < 4 {
            i += 1;
            let Some(p) = vfsmount_mount_path(path.mnt) else {
                // All crossings from the parent chain reach the absolute kernel root.
                if self.is_root() {
                    // self is itself a mount root with no further ancestry: this IS the
                    // process root. Terminate without recording its name.
                    return None;
                }
                // self is a normal dentry whose parent happens to be a mount root.
                // Return the uncrossed mount-root path so the caller records self's name;
                // the following call will see self.is_root() == true and return None.
                return Some(original_parent);
            };
            path = p;
        }
        // Self-referential parent: either a genuine filesystem root or a btrfs
        // alias/disconnected dentry whose d_parent points to itself.  Both cases
        // must terminate the walk.
        let path_parent = dentry_parent(path.dentry);
        if ptr::eq(path.dentry, path_parent) {
            return None;
        }
        Some(path)
    }
}

fn dentry_name(dentry: *const dentry) -> *const qstr {
    unsafe { ((dentry as *const u8).add(dentry_name_offset())) as *const qstr }
}

fn dentry_parent(dentry: *const dentry) -> *const dentry {
    let parent_ptr: *const dentry = ptr::null();
    unsafe {
        bpf_probe_read_kernel(
            &parent_ptr as *const *const dentry as _,
            size_of_val(&parent_ptr) as _,
            (dentry as *const u8).add(dentry_parent_offset()) as _,
        );
    }
    parent_ptr
}

fn vfsmount_root(vfsmount: *const vfsmount) -> *const dentry {
    let root: *const dentry = ptr::null();
    unsafe {
        bpf_probe_read_kernel(
            &root as *const *const dentry as _,
            size_of_val(&root) as _,
            (vfsmount as *const u8).add(vfsmount_root_offset()) as _,
        );
    }
    root
}
fn vfsmount_mount_path(vfsmount: *const vfsmount) -> Option<Path> {
    // we must obtain the parent vfsmount and the dentry covered by the mount
    unsafe {
        let vfsmount_offset = mount_vfsmount_offset();
        let mount = (vfsmount as *const u8).sub(vfsmount_offset) as *const mount;
        let parent_mount = mount_parent(mount);
        if ptr::eq(parent_mount, mount) {
            return None;
        }
        let parent_vfsmount = (parent_mount as *const u8).add(vfsmount_offset) as *const vfsmount;
        let mountpoint = mount_mountpoint(mount);
        Some(Path { dentry: &*mountpoint, mnt: &*parent_vfsmount })
    }
}

fn mount_parent(mount: *const mount) -> *const mount {
    let parent: *const mount = ptr::null();
    unsafe {
        bpf_probe_read_kernel(
            &parent as *const *const mount as _,
            size_of_val(&parent) as _,
            (mount as *const u8).add(mount_parent_offset()) as _,
        );
    }
    parent
}

fn mount_mountpoint(mount: *const mount) -> *const dentry {
    let mountpoint: *const dentry = ptr::null();
    unsafe {
        bpf_probe_read_kernel(
            &mountpoint as *const *const dentry as _,
            size_of_val(&mountpoint) as _,
            (mount as *const u8).add(mount_mountpoint_offset()) as _,
        );
    }
    mountpoint
}

fn qstr_string(qstr: *const qstr, result: &mut BpfString) {
    unsafe {
        let mut qstr_copy = MaybeUninit::<qstr>::uninit();
        let qstr_ptr = qstr_copy.as_mut_ptr();
        (*qstr_ptr).__bindgen_anon_1.hash_len = 0;
        // if the read below fails, our qstr stays at zero and we read an empty string.
        bpf_probe_read_kernel(qstr_ptr as _, size_of_val(&qstr_copy) as _, qstr as _);
        result.update(|bytes| {
            let len = (*qstr_ptr).__bindgen_anon_1.__bindgen_anon_1.len.min(bytes.len() as _);
            let r = bpf_probe_read_kernel(
                bytes as *mut u8 as *mut c_void,
                len,
                (*qstr_ptr).name as *const c_void,
            );
            if r < 0 { 0 } else { len as _ }
        });
    }
}
