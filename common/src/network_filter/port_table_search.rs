// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{
    flow_types::{Verdict, VerdictReason},
    network_filter::{
        binary_rule::{PortTableEntry, PortTableReference},
        rule_page::port_table_entry,
        rule_types::{Port, RuleId},
    },
    repeat::{LoopReturn, repeat},
};

pub enum SearchTableType {
    AnyEndpoint,
    Ipv4,
    Ipv6,
    Name,
}

/// Types implementing this trait are used to customize how rule matching is done.
pub trait SearchSpecification {
    /// This function gets a pointer to the port table of a `RulePage` and has the freedom to
    /// honor as many parameters as it likes. Implementations can ignore the port number, the
    /// protocol, the direction or all of that. The result of the saerch is stored in the
    /// type implementing the trait and possibly merged with previous search results.
    fn search_port_table(
        &mut self,
        table_type: SearchTableType,
        page_base: *const PortTableEntry,
        port_table_ref: PortTableReference,
        search_term: &PortTableSearchTerm,
    );

    /// Blocklists have no port, protocol and direction table. Results from blocklist matches
    /// are supplied via this function and possibly merged with other search results.
    fn set_blocklist_match(&mut self, rule_id: RuleId, reason: VerdictReason);

    /// If the maximum precedence of a search is known in advance, the search can be skipped
    /// if previous searches returned a result with higher precedence. This is the highest
    /// precedence rule (or rule equivalent) found so far.
    fn benchmark_rule_id(&self) -> RuleId;
}

/// parameters which can be searched in a port table:
pub struct PortTableSearchTerm {
    pub port: Port,
    pub protocol_and_direction: u8,
}

// ---------------------------------------------------------------------------
// SpecificPortTableSearch: distinguish all properties of a connectionS
// ---------------------------------------------------------------------------

/// This is the search specification used in the kernel. It distinguishes all properties of
/// a connection. When the search is done, it provides a verdict and a reason as result.
pub struct SpecificPortTableSearch {
    rule_id: RuleId,
    reason: VerdictReason,
}

impl SpecificPortTableSearch {
    pub fn new(default_verdict: Verdict) -> Self {
        Self {
            rule_id: RuleId::low_precedence_with_verdict(default_verdict),
            reason: VerdictReason::DefaultAction,
        }
    }

    pub fn result(self) -> (Verdict, VerdictReason) {
        (self.rule_id.verdict(), self.reason)
    }
}

impl SearchSpecification for SpecificPortTableSearch {
    fn search_port_table(
        &mut self,
        _table_type: SearchTableType,
        page_base: *const PortTableEntry,
        port_table_ref: PortTableReference,
        search_term: &PortTableSearchTerm,
    ) {
        let mut ctx = MatchPortTableContext {
            page_base,
            index: port_table_ref.index_from_page_start(),
            port: search_term.port,
            protocol_and_direction: search_term.protocol_and_direction,
            rule_id: None,
        };
        repeat(port_table_ref.count() as _, match_port_table_inner, &mut ctx);
        if let Some(rule_id) = ctx.rule_id
            && rule_id.supersedes(self.rule_id)
        {
            self.rule_id = rule_id;
            self.reason = VerdictReason::Rule(rule_id);
        }
    }

    fn set_blocklist_match(&mut self, rule_id: RuleId, reason: VerdictReason) {
        if rule_id.supersedes(self.rule_id) {
            self.rule_id = rule_id;
            self.reason = reason;
        }
    }

    fn benchmark_rule_id(&self) -> RuleId {
        self.rule_id
    }
}

struct MatchPortTableContext {
    page_base: *const PortTableEntry,
    index: u16,
    port: Port,
    protocol_and_direction: u8,
    rule_id: Option<RuleId>,
}

extern "C" fn match_port_table_inner(_index: u64, ctx: &mut MatchPortTableContext) -> LoopReturn {
    let entry = match port_table_entry(ctx.page_base, ctx.index) {
        None => return LoopReturn::LoopBreak,
        Some(entry) => entry,
    };
    if entry.is_stop() {
        return LoopReturn::LoopBreak;
    }
    if entry.matches(ctx.port, ctx.protocol_and_direction) {
        ctx.rule_id = Some(entry.rule_id);
        return LoopReturn::LoopBreak;
    }
    ctx.index += 1;
    LoopReturn::LoopContinue
}
