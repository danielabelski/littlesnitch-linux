// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    co_re::*,
    context::{ConcurrencyGroup, StaticBuffers},
    node_cache::{NodeCache, Path},
};
use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, generated::bpf_get_current_task_btf},
    macros::map,
    maps::{HashMap, LruHashMap},
};
use common::{
    NodeFeatures,
    flow_types::ProcessPair,
    node_cache::{NodeCacheTrait, NodeId},
    repeat::{LoopReturn, repeat_closure},
};
use core::{mem::transmute, ptr};

#[map]
static NODE_FEATURES: HashMap<NodeId, NodeFeatures> = HashMap::with_max_entries(256, 0);

#[map]
static PID_TO_NODE_ID: HashMap<i32, NodeId> = HashMap::with_max_entries(65536, 0);

#[map]
static RUNNING_EXEC_PARAMS: LruHashMap<i32, NodeId> = LruHashMap::with_max_entries(512, 0);

// All reports from the Linux kernel are with PID == thread PID, not main thread PID as
// shown by `ps` in the shell. After a fork(), the distinction is not relevant because there
// is only the main thread which has thread PID == main PID. When more threads are forked,
// they inherit the executable because `sched_process_fork()` works with thread PIDs. When
// a thread does an exec, it becomes a new tasks main thread and gets a new node set. And
// exit is called when each thread terminates.
// Lookups can therefore be done with the main PID *or* with the thread PID because we have
// both in our cache.

/// Called from a tracing function where we can associate a file path with a PID. It is not yet
/// known whether the exec will succeed. There is a second call `report_exec_success(rval: i32)`
/// which tells whether we should store the new PID -> node association or discard it.
/// Although we get the function parameters in the exit hook, the struct pointed to has been
/// modified and the data we need is no longer available on exit.
/// We may have to move the PID later if it moves to a different CGROUP.
#[inline(always)]   // only one of the two call sites will ever be used
pub fn report_exec_attempt_with_path(path: Path) {
    // unsafe { bpf_printk!(b"%d%d%d exec_with_path %pks", 1, 1, 1, (*path.dentry).d_name.name); }
    let buffers = StaticBuffers::get(ConcurrencyGroup::FentryExec);
    if buffers.is_null() {
        return;
    }
    let mut node_cache = NodeCache::new(buffers);
    if let Some(node_id) = node_cache.node_id_for_path(path) {
        let pid = bpf_get_current_pid_tgid() as i32;
        _ = RUNNING_EXEC_PARAMS.insert(&pid, &node_id, 0);
    }
}

pub fn report_exec_success(return_value: i32) {
    let pid = bpf_get_current_pid_tgid() as i32;
    if let Some(node_id) = unsafe { RUNNING_EXEC_PARAMS.get(&pid) } {
        if return_value == 0 {
            _ = PID_TO_NODE_ID.insert(&pid, node_id, 0);
        }
        _ = RUNNING_EXEC_PARAMS.remove(&pid);
    }
}

pub fn report_sched_process_exec(old_pid: i32, new_pid: i32) {
    unsafe {
        // bpf_printk!(b"%d%d report_sched_process_exec old=%d new=%d", 1, 1, old_pid, new_pid);
        if new_pid != old_pid
            && let Some(&node_id) = PID_TO_NODE_ID.get(&old_pid)
        {
            _ = PID_TO_NODE_ID.insert(&new_pid, node_id, 0);
            _ = PID_TO_NODE_ID.remove(&old_pid);
        }
    }
}

pub fn report_sched_process_fork(parent_pid: i32, child_pid: i32) {
    unsafe {
        //bpf_printk!(b"%d%d report_sched_process_fork parent=%d child=%d", 1, 1, parent_pid, child_pid);
        if let Some(&node_id) = PID_TO_NODE_ID.get(&parent_pid) {
            _ = PID_TO_NODE_ID.insert(&child_pid, node_id, 0);
        }
    }
}

pub fn report_sched_process_exit(pid: i32) {
    // unsafe { bpf_printk!(b"%d%d%d exit pid=0x%d", 1, 1, 1, pid); }
    _ = PID_TO_NODE_ID.remove(&pid);
}

impl task_struct {
    #[inline]
    pub fn current() -> Option<&'static task_struct> {
        unsafe {
            let task: *const task_struct = transmute(bpf_get_current_task_btf());
            task.as_ref()
        }
    }

    #[inline]
    fn path(&self) -> Option<&'static path> {
        unsafe { task_struct_path(self as *const _).as_ref() }
    }

    fn get_node_id(&self, node_cache: &mut NodeCache) -> Option<NodeId> {
        let pid = unsafe { task_struct_tgid(self as _) };
        if let Some(node_id) = unsafe { PID_TO_NODE_ID.get(&pid).cloned() } {
            Some(node_id)
        } else if let Some(path) = self.path()
            && let Some(node_id) = node_cache.node_id_for_path(Path::new(&path))
        {
            _ = PID_TO_NODE_ID.insert(&pid, &node_id, 0);
            Some(node_id)
        } else {
            None
        }
    }

    #[inline]
    fn parent(&self) -> Option<&'static task_struct> {
        // `real_parent` is the process that forked this task; `parent` is the process currently
        // waiting on it, which may differ when a debugger attaches via ptrace. Using `real_parent`
        // gives a more stable process tree for tracking executable ancestry.
        unsafe { task_struct_real_parent(self as _).as_ref() }
    }

    #[inline]
    pub fn uid(&self) -> u32 {
        unsafe { task_struct_uid(self as _) }
    }
}

/// `result` is not touched in case of failure. The caller should initialize `result` to zero
/// before calling, at least as far as they are interested in the values.
pub fn get_current_process_pair(
    result: &mut ProcessPair,
    buffers: *mut StaticBuffers,
) -> Option<()> {
    result.executable_pair.connecting = None;
    result.executable_pair.parent = None;
    let mut task = task_struct::current()?;
    let mut node_cache = NodeCache::new(buffers);
    result.executable_pair.connecting = task.get_node_id(&mut node_cache);
    result.executable_pair.uid = task.uid();
    result.pid = unsafe { task_struct_tgid(task as _) };
    repeat_closure(256, |_| {
        let parent = match task.parent() {
            Some(parent) => parent,
            None => return LoopReturn::LoopBreak,
        };
        let parent_tgid = unsafe { task_struct_tgid(parent as _) };
        if parent_tgid == 1 || ptr::eq(parent, task) {
            return LoopReturn::LoopBreak; // root process: pid == 1 or is its own parent
        }
        task = parent;
        if let Some(parent_node_id) = parent.get_node_id(&mut node_cache)
            && Some(parent_node_id) != result.executable_pair.connecting
        {
            let features = parent_node_id.features();
            if features.contains(NodeFeatures::APP_MANAGER) {
                return LoopReturn::LoopBreak;
            } else if features.contains(NodeFeatures::NON_PARENT) {
                return LoopReturn::LoopContinue;
            }
            // Consider this as a parent app. The last process before the app manager counts.
            result.executable_pair.parent = Some(parent_node_id);
            result.parent_pid = parent_tgid;
        }
        LoopReturn::LoopContinue
    });
    Some(())
}
trait FileNodeAppManager {
    fn features(&self) -> NodeFeatures;
}

impl FileNodeAppManager for NodeId {
    fn features(&self) -> NodeFeatures {
        unsafe { *NODE_FEATURES.get(self).unwrap_or(&NodeFeatures::default()) }
    }
}
