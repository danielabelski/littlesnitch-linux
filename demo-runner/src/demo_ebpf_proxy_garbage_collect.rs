// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{demo_ebpf_proxy::DemoEbpfProxy, nano_time};
use common::{
    flow_types::{FlowIdentifier},
};
use libc::IPPROTO_TCP;

impl DemoEbpfProxy {
    pub fn garbage_collect_flows(&mut self) {
        let now = nano_time::now();
        let mut flows_to_remove = Vec::<FlowIdentifier>::new();
        let mut flows_to_close = Vec::<FlowIdentifier>::new();
        let mut flow_count = 0u64;
        for keyvalue in self.active_flows.iter() {
            let (identifier, properties) = match keyvalue {
                Ok(keyvalue) => keyvalue,
                Err(error) => {
                    println!("Error iterating flows: {}", error);
                    break;
                }
            };
            let cookie = properties.socket_cookie;
            let mut should_close = false;
            let mut should_remove = false;
            if !properties.is_closed
                && cookie != 0
                && self.socket_properties.get(&cookie, 0).is_err()
            {
                should_close = true;
            }
            let age = now.0 - properties.last_activity.0;
            let min = 60_000_000_000_i64; // 1 minute in nanoseconds

            if identifier.protocol != IPPROTO_TCP as u32
                && (identifier.local_address.is_localhost() && age > 1 * min || age > 10 * min)
            {
                // In a localhost communication, both sockets, local and remote, are on the same
                // machine. When we ask for a socket cookie of the connection, it's ambiguous what
                // we get. If we get the server socket of a UDP connection, it is long lived and
                // we have to use heuristics to close the flow. Use a shorter timeout of 2 minutes.
                should_close = true;
                should_remove = true;
            }
            if properties.is_closed && age > 1 * min {
                // This is our TIME_WAIT
                // wait for a while before removing flow info for closed sockets because
                // network packets may still arrive after close.
                should_remove = true;
            }
            if should_close && !properties.is_closed {
                flows_to_close.push(identifier);
            }
            if should_remove {
                flows_to_remove.push(identifier);
            } else if !properties.is_closed {
                flow_count += 1;
            }
        }
        for identifier in flows_to_close {
            if let Ok(mut properties) = self.active_flows.get(&identifier, 0) {
                properties.is_closed = true;
                _ = self.active_flows.insert(&identifier, &properties, 0);
                // we could report close event to UI here
            }
        }
        for identifier in flows_to_remove {
            _ = self.active_flows.remove(&identifier);
        }
        println!("{} active flows", flow_count);
    }

    pub fn dump_active_flows(&self) {
        let now = nano_time::now();
        println!("--- Active Flows ---");
        let mut count = 0;
        for keyvalue in self.active_flows.iter() {
            if let Ok((identifier, properties)) = keyvalue {
                let age = now.0 - properties.last_activity.0;
                println!(
                    "active flow: {:?} with {:?} age {}",
                    identifier,
                    properties,
                    age as f64 / (1000_000_000.0)
                );
                count += 1;
            }
        }
        println!("{} active flows", count);
        println!("--- End Active Flows ---");
    }
}
