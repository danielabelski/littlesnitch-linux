// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

#![cfg(test)]

use crate::StringId;
use std::collections::HashMap;

pub struct MockStringsCache {
    string_to_identifier: HashMap<String, StringId>,
    identifier_to_string: HashMap<StringId, String>,

    next_identifier: u64,
}

impl MockStringsCache {
    pub fn new() -> Self {
        Self {
            string_to_identifier: HashMap::new(),
            identifier_to_string: HashMap::new(),
            next_identifier: 1,
        }
    }

    pub fn identifier_for_string(&mut self, string: &String) -> StringId {
        if let Some(id) = self.string_to_identifier.get(string) {
            id.clone()
        } else {
            let new_id = StringId(self.next_identifier);
            self.next_identifier += 1;
            self.string_to_identifier.insert(string.clone(), new_id);
            self.identifier_to_string.insert(new_id, string.clone());
            new_id
        }
    }

    pub fn string_for_identifier(&self, id: StringId) -> String {
        self.identifier_to_string
            .get(&id)
            .map(|s| s.clone())
            .unwrap_or_else(|| "<invalid>".into())
    }
}
