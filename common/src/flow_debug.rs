// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

#![cfg(feature = "user")]

use std::fmt::Debug;

use crate::flow_types::*;

const IPPROTO_TCP: u8 = 6;
const IPPROTO_UDP: u8 = 17;
const IPPROTO_ICMP: u8 = 1;
const IPPROTO_ICMP6: u8 = 58;

impl Debug for IpAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = self.core_ip_addr().to_string();
        f.write_str(&s)
    }
}

pub fn string_for_protocol(protocol: u8) -> String {
    match protocol {
        IPPROTO_TCP => "TCP".into(),
        IPPROTO_UDP => "UDP".into(),
        IPPROTO_ICMP => "ICMP".into(),
        IPPROTO_ICMP6 => "ICMP6".into(),
        _ => protocol.to_string(),
    }
}

impl Debug for FlowIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let proto = match self.protocol as _ {
            IPPROTO_TCP => "TCP",
            IPPROTO_UDP => "UDP",
            IPPROTO_ICMP => "ICMP",
            IPPROTO_ICMP6 => "ICMP6",
            _ => {
                return f.write_fmt(format_args!(
                    "Proto {} {:?}[{}] <=> {:?}[{}]",
                    self.protocol,
                    self.local_address,
                    self.local_port,
                    self.remote_address,
                    self.remote_port
                ));
            }
        };
        f.write_fmt(format_args!(
            "{} {:?}[{}] <=> {:?}[{}]",
            proto, self.local_address, self.local_port, self.remote_address, self.remote_port
        ))
    }
}

impl Debug for FlowProperties {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let direction = if self.is_inbound { "<=" } else { "=>" };
        f.write_fmt(format_args!(
            "exe: {} {} remote_name: {}, socket: 0x{:x}",
            self.process_pair.pid,
            direction,
            self.remote_name.0,
            self.socket_cookie,
        ))
    }
}
