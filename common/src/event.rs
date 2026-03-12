// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::StringId;
use crate::bitset::BitSet;
use crate::flow_types::{IpAddress, ProcessPair, VerdictReason};

pub const CONNECT: BitSet = BitSet(1 << 0);
pub const DISCONNECT: BitSet = BitSet(1 << 1);
pub const BLOCKED: BitSet = BitSet(1 << 2);
pub const EXECUTABLE_UPDATED: BitSet = BitSet(1 << 3);  // executable pair may have changed
pub const REMOTE_NAME_UPDATED: BitSet = BitSet(1 << 4); // remote name may have changed

/// This type combines all properties used for rule matching. It does not identify a flow
/// uniquely, though. If one process opens multiple connections to the same remote endpoint,
/// they are all attributed to the same `ConnectionIdentifier`. The `Event` below introduces
/// an `other_port` property to disambiguate during debugging.
#[repr(C)]
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "user", derive(Copy, Hash))]
pub struct ConnectionIdentifier {
    pub process_pair: ProcessPair,
    pub remote_address: IpAddress,
    pub remote_name: StringId,
    pub is_inbound: bool,
    pub protocol: u8,
    pub port: u16, // remote port for outbound, local port for inbound
}

#[repr(C)]
#[derive(Clone)]
pub struct EventPayload {
    pub ephemeral_port: u16, // The port at the other endpoint, for debugging, not merged correctly
    pub changes: BitSet,
    pub verdict_reason: VerdictReason, // only valid when CONNECT or BLOCKED is set
    pub bytes_sent: u64,
    pub bytes_received: u64,
}


/// The `Event` type represents things that can happen to a flow. It is designed to be mergeable
/// with other events for the same flow to represent all changes that occurred during the merge
/// period.
#[repr(C)]
#[derive(Clone)]
pub struct Event {
    pub connection_identifier: ConnectionIdentifier,
    pub payload: EventPayload,
}

impl EventPayload {
    #[cfg(feature = "user")]
    pub fn changes_debug(&self) -> String {
        let mut values = Vec::<&str>::new();
        if self.changes.contains(CONNECT) {
            values.push("con");
        }
        if self.changes.contains(DISCONNECT) {
            values.push("dis");
        }
        if self.changes.contains(BLOCKED) {
            values.push("blk");
        }
        if self.changes.contains(EXECUTABLE_UPDATED) {
            values.push("exe");
        }
        if self.changes.contains(REMOTE_NAME_UPDATED) {
            values.push("dns");
        }
        values.join("|")
    }
}
