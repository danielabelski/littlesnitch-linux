// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::demo_ebpf_proxy::DemoEbpfProxy;
use common::{StringId, event::Event, node_cache::NodeId};
use log::error;

impl DemoEbpfProxy {
    pub fn poll_events(&mut self) {
        while let Some(event) = self.next_event() {
            self.handle_event(&event);
        }
    }

    /// Call this function when `self.event_queue_ringbuf.as_fd()` becomes readable.
    fn next_event(&mut self) -> Option<Event> {
        while let Some(entry) = self.events.next() {
            if entry.len() == size_of::<Event>() {
                return Some(unsafe { (*(entry.as_ptr() as *const Event)).clone() });
            } else {
                error!("Found entry of size {} instead of {}", entry.len(), size_of::<Event>());
            }
        }
        None
    }

    fn dns_name(&self, string_id: StringId) -> String {
        if string_id == StringId::none() {
            "<none>".to_string()
        } else {
            let mut string =
                self.node_manager.node_cache.strings_cache.string_for_identifier(string_id);
            unsafe {
                for c in string.as_bytes_mut() {
                    if c == &0 {
                        *c = b'.';
                    }
                }
            }
            string
        }
    }

    fn exe_name(&mut self, node_id: Option<NodeId>) -> String {
        let executable = node_id
            .map(|n| self.node_manager.node_cache.executable_for_node_id(n));
        executable
            .map(|e| e.0.to_string_lossy().to_string())
            .unwrap_or_else(|| "<none>".to_string())
    }

    fn handle_event(&mut self, event: &Event) {
        println!(
            "{} parent {} -- {:?}:{} ({}) proto {} inbound {}",
            self.exe_name(event.connection_identifier.process_pair.executable_pair.connecting),
            self.exe_name(event.connection_identifier.process_pair.executable_pair.parent),
            event.connection_identifier.remote_address.core_ip_addr(),
            event.connection_identifier.port,
            self.dns_name(event.connection_identifier.remote_name),
            event.connection_identifier.protocol,
            event.connection_identifier.is_inbound
        );
        println!(
            "    in: {}  out: {}  flags: {:x}",
            event.payload.bytes_received,
            event.payload.bytes_sent,
            event.payload.changes.raw()
        );
    }
}
