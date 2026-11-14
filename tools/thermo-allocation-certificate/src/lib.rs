// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent feasibility checker for shared-capacity well-to-agent allocations.
//!
//! This checker verifies a supplied allocation; it does not prove that a solver
//! found a lexicographic max-min optimum. All quantities are integer quanta.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Agent {
    pub id: u64,
    pub cap: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Well {
    pub id: u64,
    pub supply: u64,
    pub eligible_agents: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeFlow {
    pub well_id: u64,
    pub agent_id: u64,
    pub amount: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdAmount {
    pub id: u64,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FairnessObjective {
    AbsoluteAmount,
    CapacityNormalized,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceValue {
    Absolute(u64),
    Ratio { numerator: u64, denominator: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Certificate {
    pub case_id: String,
    pub wells: Vec<Well>,
    pub agents: Vec<Agent>,
    pub flows: Vec<EdgeFlow>,
    pub claimed_source_debits: Vec<IdAmount>,
    pub claimed_recipient_credits: Vec<IdAmount>,
    pub objective: FairnessObjective,
    pub claimed_service_profile: Vec<ServiceValue>,
    pub claims_optimality: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violation {
    pub code: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    /// True means only that all checks in this checker passed.
    pub feasible: bool,
    pub violations: Vec<Violation>,
    pub warnings: Vec<String>,
    pub computed_source_debits: Vec<IdAmount>,
    pub unused_source_supply: Vec<IdAmount>,
    pub computed_recipient_credits: Vec<IdAmount>,
    pub remaining_recipient_caps: Vec<IdAmount>,
    pub computed_service_profile: Vec<ServiceValue>,
    pub total_debited: u128,
    pub total_credited: u128,
    /// This checker never proves optimality; a separate oracle or proof is needed.
    pub optimality_proven: bool,
}

fn push_violation(violations: &mut Vec<Violation>, code: &str, detail: impl Into<String>) {
    violations.push(Violation {
        code: code.to_owned(),
        detail: detail.into(),
    });
}

fn empty_report(violations: Vec<Violation>) -> VerificationReport {
    VerificationReport {
        feasible: false,
        violations,
        warnings: Vec::new(),
        computed_source_debits: Vec::new(),
        unused_source_supply: Vec::new(),
        computed_recipient_credits: Vec::new(),
        remaining_recipient_caps: Vec::new(),
        computed_service_profile: Vec::new(),
        total_debited: 0,
        total_credited: 0,
        optimality_proven: false,
    }
}

fn insert_total(
    totals: &mut BTreeMap<u64, u64>,
    id: u64,
    amount: u64,
    side: &str,
    violations: &mut Vec<Violation>,
) {
    let Some(current) = totals.get_mut(&id) else {
        push_violation(
            violations,
            "internal_unknown_total",
            format!("{side} accumulator has no entry for ID {id}"),
        );
        return;
    };
    match current.checked_add(amount) {
        Some(sum) => *current = sum,
        None => push_violation(
            violations,
            "arithmetic_overflow",
            format!("{side} total overflowed u64 for ID {id}"),
        ),
    }
}

fn verify_claims(
    label: &str,
    claimed: &[IdAmount],
    expected: &BTreeMap<u64, u64>,
    violations: &mut Vec<Violation>,
) {
    let mut seen = BTreeSet::new();
    for item in claimed {
        if !seen.insert(item.id) {
            push_violation(
                violations,
                &format!("duplicate_{label}_claim"),
                format!("ID {} is claimed more than once", item.id),
            );
            continue;
        }
        match expected.get(&item.id) {
            None => push_violation(
                violations,
                &format!("unknown_{label}_claim"),
                format!("claim references unknown ID {}", item.id),
            ),
            Some(expected_amount) if *expected_amount != item.amount => push_violation(
                violations,
                &format!("{label}_claim_mismatch"),
                format!(
                    "ID {} claims {}, computed {}",
                    item.id, item.amount, expected_amount
                ),
            ),
            Some(_) => {}
        }
    }
    for id in expected.keys() {
        if !seen.contains(id) {
            push_violation(
                violations,
                &format!("missing_{label}_claim"),
                format!("no claim supplied for ID {id}"),
            );
        }
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn ratio_cmp(left: (u64, u64), right: (u64, u64)) -> std::cmp::Ordering {
    (u128::from(left.0) * u128::from(right.1))
        .cmp(&(u128::from(right.0) * u128::from(left.1)))
}

fn service_profile(
    agents: &[Agent],
    credits: &BTreeMap<u64, u64>,
    objective: FairnessObjective,
) -> Vec<ServiceValue> {
    let mut profile = match objective {
        FairnessObjective::AbsoluteAmount => agents
            .iter()
            .map(|agent| ServiceValue::Absolute(*credits.get(&agent.id).unwrap_or(&0)))
            .collect::<Vec<_>>(),
        FairnessObjective::CapacityNormalized => agents
            .iter()
            .filter(|agent| agent.cap > 0)
            .map(|agent| {
                let amount = *credits.get(&agent.id).unwrap_or(&0);
                let divisor = gcd(amount, agent.cap);
                ServiceValue::Ratio {
                    numerator: amount / divisor,
                    denominator: agent.cap / divisor,
                }
            })
            .collect::<Vec<_>>(),
    };
    profile.sort_by(|left, right| match (left, right) {
        (ServiceValue::Absolute(a), ServiceValue::Absolute(b)) => a.cmp(b),
        (
            ServiceValue::Ratio {
                numerator: a_num,
                denominator: a_den,
            },
            ServiceValue::Ratio {
                numerator: b_num,
                denominator: b_den,
            },
        ) => ratio_cmp((*a_num, *a_den), (*b_num, *b_den)),
        (ServiceValue::Absolute(_), ServiceValue::Ratio { .. }) => std::cmp::Ordering::Less,
        (ServiceValue::Ratio { .. }, ServiceValue::Absolute(_)) => std::cmp::Ordering::Greater,
    });
    profile
}

fn map_to_id_amount(values: &BTreeMap<u64, u64>) -> Vec<IdAmount> {
    values
        .iter()
        .map(|(id, amount)| IdAmount {
            id: *id,
            amount: *amount,
        })
        .collect()
}

/// Verify feasibility and accounting for a proposed shared-capacity allocation.
///
/// A true feasibility flag means the supplied data is internally consistent
/// with supply, cap and eligibility constraints. It is not an optimality proof.
/// Inputs and outputs are sorted by stable numeric IDs in the report.
pub fn verify(certificate: &Certificate) -> VerificationReport {
    let mut violations = Vec::new();
    let mut well_ids = BTreeSet::new();
    let mut agent_ids = BTreeSet::new();

    for well in &certificate.wells {
        if !well_ids.insert(well.id) {
            push_violation(
                &mut violations,
                "duplicate_well_id",
                format!("well ID {} is not unique", well.id),
            );
        }
    }
    for agent in &certificate.agents {
        if !agent_ids.insert(agent.id) {
            push_violation(
                &mut violations,
                "duplicate_agent_id",
                format!("agent ID {} is not unique", agent.id),
            );
        }
    }

    // Duplicate entity IDs make any mapping ambiguous; fail closed before tallying.
    if violations
        .iter()
        .any(|issue| issue.code == "duplicate_well_id" || issue.code == "duplicate_agent_id")
    {
        violations.sort_by(|left, right| {
            (&left.code, &left.detail).cmp(&(&right.code, &right.detail))
        });
        return empty_report(violations);
    }

    let well_specs: BTreeMap<u64, &Well> =
        certificate.wells.iter().map(|well| (well.id, well)).collect();
    let agent_specs: BTreeMap<u64, &Agent> =
        certificate.agents.iter().map(|agent| (agent.id, agent)).collect();

    let mut eligibility = BTreeMap::<u64, BTreeSet<u64>>::new();
    for well in &certificate.wells {
        let mut eligible = BTreeSet::new();
        for agent_id in &well.eligible_agents {
            if !agent_ids.contains(agent_id) {
                push_violation(
                    &mut violations,
                    "eligibility_unknown_agent",
                    format!("well {} references missing agent {}", well.id, agent_id),
                );
            }
            if !eligible.insert(*agent_id) {
                push_violation(
                    &mut violations,
                    "duplicate_eligibility_id",
                    format!("well {} repeats agent {} in eligibility", well.id, agent_id),
                );
            }
        }
        eligibility.insert(well.id, eligible);
    }

    let mut source_debits: BTreeMap<u64, u64> =
        certificate.wells.iter().map(|well| (well.id, 0)).collect();
    let mut recipient_credits: BTreeMap<u64, u64> =
        certificate.agents.iter().map(|agent| (agent.id, 0)).collect();
    let mut seen_edges = BTreeSet::new();

    for flow in &certificate.flows {
        if !seen_edges.insert((flow.well_id, flow.agent_id)) {
            push_violation(
                &mut violations,
                "duplicate_edge_flow",
                format!(
                    "edge well {} -> agent {} is listed more than once",
                    flow.well_id, flow.agent_id
                ),
            );
        }

        match well_specs.get(&flow.well_id) {
            None => push_violation(
                &mut violations,
                "flow_unknown_well",
                format!("flow references missing well {}", flow.well_id),
            ),
            Some(_) => {
                insert_total(
                    &mut source_debits,
                    flow.well_id,
                    flow.amount,
                    "source debit",
                    &mut violations,
                );
                if flow.amount > 0
                    && !eligibility
                        .get(&flow.well_id)
                        .is_some_and(|agents| agents.contains(&flow.agent_id))
                {
                    push_violation(
                        &mut violations,
                        "ineligible_flow",
                        format!(
                            "well {} sends {} quanta to ineligible agent {}",
                            flow.well_id, flow.amount, flow.agent_id
                        ),
                    );
                }
            }
        }

        match agent_specs.get(&flow.agent_id) {
            None => push_violation(
                &mut violations,
                "flow_unknown_agent",
                format!("flow references missing agent {}", flow.agent_id),
            ),
            Some(_) => insert_total(
                &mut recipient_credits,
                flow.agent_id,
                flow.amount,
                "recipient credit",
                &mut violations,
            ),
        }
    }

    let mut unused_supply = BTreeMap::new();
    for well in &certificate.wells {
        let debit = *source_debits.get(&well.id).unwrap_or(&0);
        if debit > well.supply {
            push_violation(
                &mut violations,
                "source_overspend",
                format!(
                    "well {} debits {} quanta from supply {}",
                    well.id, debit, well.supply
                ),
            );
        }
        unused_supply.insert(well.id, well.supply.saturating_sub(debit));
    }

    let mut remaining_caps = BTreeMap::new();
    for agent in &certificate.agents {
        let credit = *recipient_credits.get(&agent.id).unwrap_or(&0);
        if credit > agent.cap {
            push_violation(
                &mut violations,
                "recipient_cap_violation",
                format!(
                    "agent {} receives {} quanta with cap {}",
                    agent.id, credit, agent.cap
                ),
            );
        }
        remaining_caps.insert(agent.id, agent.cap.saturating_sub(credit));
    }

    let total_debited: u128 = source_debits.values().map(|value| u128::from(*value)).sum();
    let total_credited: u128 = recipient_credits
        .values()
        .map(|value| u128::from(*value))
        .sum();
    if total_debited != total_credited {
        push_violation(
            &mut violations,
            "conservation_mismatch",
            format!(
                "computed source debit total {total_debited} differs from recipient credit total {total_credited}"
            ),
        );
    }

    verify_claims(
        "source_debit",
        &certificate.claimed_source_debits,
        &source_debits,
        &mut violations,
    );
    verify_claims(
        "recipient_credit",
        &certificate.claimed_recipient_credits,
        &recipient_credits,
        &mut violations,
    );

    let computed_profile = service_profile(
        &certificate.agents,
        &recipient_credits,
        certificate.objective,
    );
    if computed_profile != certificate.claimed_service_profile {
        push_violation(
            &mut violations,
            "service_profile_mismatch",
            format!(
                "claimed profile {:?} differs from computed profile {:?}",
                certificate.claimed_service_profile, computed_profile
            ),
        );
    }

    let mut warnings = Vec::new();
    if certificate.claims_optimality {
        warnings.push(
            "optimality is claimed but not established by this feasibility checker".to_owned(),
        );
    }

    violations.sort_by(|left, right| {
        (&left.code, &left.detail).cmp(&(&right.code, &right.detail))
    });

    VerificationReport {
        feasible: violations.is_empty(),
        violations,
        warnings,
        computed_source_debits: map_to_id_amount(&source_debits),
        unused_source_supply: map_to_id_amount(&unused_supply),
        computed_recipient_credits: map_to_id_amount(&recipient_credits),
        remaining_recipient_caps: map_to_id_amount(&remaining_caps),
        computed_service_profile: computed_profile,
        total_debited,
        total_credited,
        optimality_proven: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Certificate {
        Certificate {
            case_id: "two-overlapping-wells".to_owned(),
            wells: vec![
                Well {
                    id: 10,
                    supply: 12,
                    eligible_agents: vec![1, 2],
                },
                Well {
                    id: 20,
                    supply: 12,
                    eligible_agents: vec![1, 3],
                },
            ],
            agents: vec![
                Agent { id: 1, cap: 9 },
                Agent { id: 2, cap: 12 },
                Agent { id: 3, cap: 12 },
            ],
            flows: vec![
                EdgeFlow { well_id: 10, agent_id: 1, amount: 4 },
                EdgeFlow { well_id: 10, agent_id: 2, amount: 8 },
                EdgeFlow { well_id: 20, agent_id: 1, amount: 4 },
                EdgeFlow { well_id: 20, agent_id: 3, amount: 8 },
            ],
            claimed_source_debits: vec![
                IdAmount { id: 10, amount: 12 },
                IdAmount { id: 20, amount: 12 },
            ],
            claimed_recipient_credits: vec![
                IdAmount { id: 1, amount: 8 },
                IdAmount { id: 2, amount: 8 },
                IdAmount { id: 3, amount: 8 },
            ],
            objective: FairnessObjective::AbsoluteAmount,
            claimed_service_profile: vec![
                ServiceValue::Absolute(8),
                ServiceValue::Absolute(8),
                ServiceValue::Absolute(8),
            ],
            claims_optimality: false,
        }
    }

    fn has_code(report: &VerificationReport, code: &str) -> bool {
        report.violations.iter().any(|issue| issue.code == code)
    }

    #[test]
    fn accepts_feasible_overlap_fixture_and_reconciles_conservation() {
        let report = verify(&fixture());
        assert!(report.feasible, "{:?}", report.violations);
        assert_eq!(report.total_debited, 24);
        assert_eq!(report.total_credited, 24);
        assert_eq!(
            report.computed_recipient_credits,
            vec![
                IdAmount { id: 1, amount: 8 },
                IdAmount { id: 2, amount: 8 },
                IdAmount { id: 3, amount: 8 },
            ]
        );
        assert!(!report.optimality_proven);
    }

    #[test]
    fn detects_source_overspend() {
        let mut certificate = fixture();
        certificate.flows[0].amount = 13;
        let report = verify(&certificate);
        assert!(!report.feasible);
        assert!(has_code(&report, "source_overspend"));
    }

    #[test]
    fn detects_shared_recipient_cap_violation() {
        let mut certificate = fixture();
        certificate.flows[0].amount = 5;
        certificate.flows[1].amount = 7;
        certificate.flows[2].amount = 5;
        certificate.flows[3].amount = 7;
        let report = verify(&certificate);
        assert!(!report.feasible);
        assert!(has_code(&report, "recipient_cap_violation"));
    }

    #[test]
    fn detects_ineligible_edges_and_unknown_recipients() {
        let mut certificate = fixture();
        certificate.flows.push(EdgeFlow {
            well_id: 10,
            agent_id: 3,
            amount: 1,
        });
        let report = verify(&certificate);
        assert!(has_code(&report, "ineligible_flow"));
        assert!(!report.feasible);

        let mut unknown = fixture();
        unknown.flows.push(EdgeFlow {
            well_id: 10,
            agent_id: 999,
            amount: 1,
        });
        let report = verify(&unknown);
        assert!(has_code(&report, "flow_unknown_agent"));
    }

    #[test]
    fn detects_mismatched_accounting_claims_and_profiles() {
        let mut certificate = fixture();
        certificate.claimed_source_debits[0].amount = 11;
        certificate.claimed_service_profile = vec![
            ServiceValue::Absolute(7),
            ServiceValue::Absolute(8),
            ServiceValue::Absolute(9),
        ];
        let report = verify(&certificate);
        assert!(has_code(&report, "source_debit_claim_mismatch"));
        assert!(has_code(&report, "service_profile_mismatch"));
    }

    #[test]
    fn detects_duplicate_edges_and_duplicate_entity_ids() {
        let mut certificate = fixture();
        certificate.flows.push(certificate.flows[0].clone());
        assert!(has_code(&verify(&certificate), "duplicate_edge_flow"));

        let mut duplicate_agent = fixture();
        duplicate_agent.agents.push(Agent { id: 1, cap: 9 });
        assert!(has_code(&verify(&duplicate_agent), "duplicate_agent_id"));
        assert!(!verify(&duplicate_agent).feasible);
    }

    #[test]
    fn normalized_profile_uses_exact_reduced_fractions() {
        let mut certificate = fixture();
        certificate.flows = vec![
            EdgeFlow { well_id: 10, agent_id: 1, amount: 3 },
            EdgeFlow { well_id: 10, agent_id: 2, amount: 9 },
            EdgeFlow { well_id: 20, agent_id: 1, amount: 4 },
            EdgeFlow { well_id: 20, agent_id: 3, amount: 8 },
        ];
        certificate.claimed_source_debits = vec![
            IdAmount { id: 10, amount: 12 },
            IdAmount { id: 20, amount: 12 },
        ];
        certificate.claimed_recipient_credits = vec![
            IdAmount { id: 1, amount: 7 },
            IdAmount { id: 2, amount: 9 },
            IdAmount { id: 3, amount: 8 },
        ];
        certificate.objective = FairnessObjective::CapacityNormalized;
        certificate.claimed_service_profile = vec![
            ServiceValue::Ratio { numerator: 2, denominator: 3 },
            ServiceValue::Ratio { numerator: 3, denominator: 4 },
            ServiceValue::Ratio { numerator: 7, denominator: 9 },
        ];
        let report = verify(&certificate);
        assert!(report.feasible, "{:?}", report.violations);
        assert_eq!(report.computed_service_profile, certificate.claimed_service_profile);
    }

    #[test]
    fn zero_cap_agents_are_excluded_only_from_normalized_profile() {
        let mut certificate = fixture();
        certificate.agents.push(Agent { id: 4, cap: 0 });
        certificate.wells[0].eligible_agents.push(4);
        certificate.claimed_recipient_credits.push(IdAmount { id: 4, amount: 0 });
        certificate.objective = FairnessObjective::CapacityNormalized;
        certificate.claimed_service_profile = vec![
            ServiceValue::Ratio { numerator: 2, denominator: 3 },
            ServiceValue::Ratio { numerator: 2, denominator: 3 },
            ServiceValue::Ratio { numerator: 8, denominator: 9 },
        ];
        let report = verify(&certificate);
        assert!(report.feasible, "{:?}", report.violations);
        assert_eq!(report.computed_service_profile.len(), 3);
    }

    #[test]
    fn storage_order_does_not_change_report() {
        let original = verify(&fixture());
        let mut permuted = fixture();
        permuted.wells.reverse();
        permuted.agents.reverse();
        permuted.flows.reverse();
        permuted.claimed_source_debits.reverse();
        permuted.claimed_recipient_credits.reverse();
        for well in &mut permuted.wells {
            well.eligible_agents.reverse();
        }
        assert_eq!(original, verify(&permuted));
    }

    #[test]
    fn does_not_silently_accept_arithmetic_overflow() {
        let certificate = Certificate {
            case_id: "overflow".to_owned(),
            wells: vec![Well {
                id: 1,
                supply: u64::MAX,
                eligible_agents: vec![1, 2],
            }],
            agents: vec![
                Agent { id: 1, cap: u64::MAX },
                Agent { id: 2, cap: 1 },
            ],
            flows: vec![
                EdgeFlow { well_id: 1, agent_id: 1, amount: u64::MAX },
                EdgeFlow { well_id: 1, agent_id: 2, amount: 1 },
            ],
            claimed_source_debits: vec![IdAmount { id: 1, amount: u64::MAX }],
            claimed_recipient_credits: vec![
                IdAmount { id: 1, amount: u64::MAX },
                IdAmount { id: 2, amount: 1 },
            ],
            objective: FairnessObjective::AbsoluteAmount,
            claimed_service_profile: vec![
                ServiceValue::Absolute(1),
                ServiceValue::Absolute(u64::MAX),
            ],
            claims_optimality: false,
        };
        let report = verify(&certificate);
        assert!(!report.feasible);
        assert!(has_code(&report, "arithmetic_overflow"));
    }

    #[test]
    fn optimality_claim_is_explicitly_not_proven() {
        let mut certificate = fixture();
        certificate.claims_optimality = true;
        let report = verify(&certificate);
        assert!(report.feasible);
        assert!(!report.optimality_proven);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("not established"));
    }
}
