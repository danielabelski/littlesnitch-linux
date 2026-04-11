// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use aya_ebpf::{macros::map, maps::RingBuf};
use common::event::Event;

#[map]
static EVENT_QUEUE: RingBuf = RingBuf::with_byte_size(1048576, 0);

/// Calls `initializer` to initialize the uninitialized event. If the initializer returns
/// true, the event is enqueued, if it returns false, it is discarded.
pub fn enqueue_event(initializer: impl FnOnce(&mut Event) -> bool) {
    if let Some(mut buffer) = EVENT_QUEUE.reserve::<Event>(0) {
        if initializer(unsafe { &mut *buffer.as_mut_ptr() }) {
            buffer.submit(0);
        } else {
            buffer.discard(0);
        }
    }
}
