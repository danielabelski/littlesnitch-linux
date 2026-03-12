// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::unique_id::{Purpose, UniqueId};
use aya_ebpf::{
    bindings::{BPF_F_NO_PREALLOC, BPF_NOEXIST},
    macros::map,
    maps::HashMap,
};
use common::{StringId, bpf_string::BpfString};

// BPF_F_NO_PREALLOC avoids pre-allocating memory for all 65536 slots at load time.
// A BpfString is large, so pre-allocation would consume hundreds of megabytes of kernel memory.
#[map]
static STRING_TO_IDENTIFIER: HashMap<BpfString, u64> =
    HashMap::with_max_entries(65536, BPF_F_NO_PREALLOC);

#[map]
static IDENTIFIER_TO_STRING: HashMap<u64, BpfString> =
    HashMap::with_max_entries(65536, BPF_F_NO_PREALLOC);

pub fn identifier_for_string(string: &BpfString) -> StringId {
    let mut unique_id = UniqueId::new(Purpose::StringId);
    let proposed_identifier = unique_id.get().get();
    let id = if STRING_TO_IDENTIFIER.insert(string, proposed_identifier, BPF_NOEXIST as _) == Ok(())
    {
        // The identifier cannot be used for a lookup before we insert
        // because we have not returned it yet.
        _ = IDENTIFIER_TO_STRING.insert(proposed_identifier, string, 0);
        unique_id.consume();
        proposed_identifier
    } else {
        unsafe { *STRING_TO_IDENTIFIER.get(string).unwrap_or(&0) }
    };
    StringId(id)
}

pub fn string_for_identifier(identifier: StringId) -> Option<&'static BpfString> {
    unsafe { IDENTIFIER_TO_STRING.get(&identifier.0) }
}
