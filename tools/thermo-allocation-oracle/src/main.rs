// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exhaustive finite-grid oracle for tiny overlapping-well allocation graphs.
//! Research verifier only; this is neither the production allocator nor a
//! continuous-rational proof. Integer quanta make this fixture arithmetic exact.

#[derive(Clone, Debug, Eq, PartialEq)]
struct Agent {
    id: u64,
    cap: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Well {
    id: u64,
    supply: u32,
    eligible: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Problem {
    wells: Vec<Well>,
    agents: Vec<Agent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Allocation {
    // Rows: wells by stable ID; columns and totals: agents by stable ID.
    matrix: Vec<Vec<u32>>,
    totals: Vec<u32>,
    profile: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Objective {
    AbsoluteAmount,
    CapacityNormalized,
}

struct Search {
    best: Option<Allocation>,
    nodes: u64,
    leaves: u64,
    objective: Objective,
}

fn fixture() -> Problem {
    // One quantum = 1/3 of an original unit: supplies 4,4 and caps 3,4,4.
    Problem {
        wells: vec![
            Well {
                id: 10,
                supply: 12,
                eligible: vec![1, 2],
            },
            Well {
                id: 20,
                supply: 12,
                eligible: vec![1, 3],
            },
        ],
        agents: vec![
            Agent { id: 1, cap: 9 },
            Agent { id: 2, cap: 12 },
            Agent { id: 3, cap: 12 },
        ],
    }
}

fn canonical(mut p: Problem) -> Problem {
    p.wells.sort_by_key(|w| w.id);
    p.agents.sort_by_key(|a| a.id);
    for w in &mut p.wells {
        w.eligible.sort_unstable();
        w.eligible.dedup();
    }
    p
}

fn validate(p: &Problem) {
    assert!(
        p.wells.len() <= 3 && p.agents.len() <= 4,
        "tiny-oracle size bound exceeded"
    );
    assert!(
        p.wells.iter().all(|w| w.supply <= 12),
        "per-well supply bound exceeded"
    );
    assert!(
        p.wells.windows(2).all(|x| x[0].id != x[1].id),
        "duplicate well ID"
    );
    assert!(
        p.agents.windows(2).all(|x| x[0].id != x[1].id),
        "duplicate agent ID"
    );
    for w in &p.wells {
        assert!(
            w.eligible.iter().all(|id| p.agents.iter().any(|a| a.id == *id)),
            "eligibility references unknown agent"
        );
    }
}

fn profile(totals: &[u32]) -> Vec<u32> {
    let mut p = totals.to_vec();
    p.sort_unstable();
    p
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

// Zero-cap recipients have no normalized service ratio and are excluded.
fn capacity_profile(agents: &[Agent], totals: &[u32]) -> Vec<(u32, u32)> {
    let mut ratios: Vec<(u32, u32)> = agents
        .iter()
        .zip(totals)
        .filter(|(agent, _)| agent.cap > 0)
        .map(|(agent, total)| {
            let divisor = gcd(*total, agent.cap);
            (*total / divisor, agent.cap / divisor)
        })
        .collect();
    ratios.sort_by(|left, right| compare_ratio(*left, *right));
    ratios
}

fn compare_ratio(left: (u32, u32), right: (u32, u32)) -> std::cmp::Ordering {
    (u64::from(left.0) * u64::from(right.1))
        .cmp(&(u64::from(right.0) * u64::from(left.1)))
}

fn compare_ratio_profiles(
    left: &[(u32, u32)],
    right: &[(u32, u32)],
) -> std::cmp::Ordering {
    for (left_ratio, right_ratio) in left.iter().zip(right) {
        let ordering = compare_ratio(*left_ratio, *right_ratio);
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn compare_allocations(
    problem: &Problem,
    left: &Allocation,
    right: &Allocation,
    objective: Objective,
) -> std::cmp::Ordering {
    match objective {
        Objective::AbsoluteAmount => left.profile.cmp(&right.profile),
        Objective::CapacityNormalized => compare_ratio_profiles(
            &capacity_profile(&problem.agents, &left.totals),
            &capacity_profile(&problem.agents, &right.totals),
        ),
    }
}

fn feasible(p: &Problem, matrix: &[Vec<u32>], totals: &[u32]) -> bool {
    let p = canonical(p.clone());
    if matrix.len() != p.wells.len() || totals.len() != p.agents.len() {
        return false;
    }
    for (wi, w) in p.wells.iter().enumerate() {
        if matrix[wi].len() != p.agents.len() || matrix[wi].iter().copied().sum::<u32>() > w.supply {
            return false;
        }
        for (ai, a) in p.agents.iter().enumerate() {
            if !w.eligible.contains(&a.id) && matrix[wi][ai] != 0 {
                return false;
            }
        }
    }
    for (ai, a) in p.agents.iter().enumerate() {
        let sum: u32 = matrix.iter().map(|row| row[ai]).sum();
        if sum != totals[ai] || sum > a.cap {
            return false;
        }
    }
    true
}

fn visit(
    p: &Problem,
    wi: usize,
    ai: usize,
    remaining: u32,
    matrix: &mut [Vec<u32>],
    totals: &mut [u32],
    state: &mut Search,
) {
    state.nodes += 1;
    if wi == p.wells.len() {
        state.leaves += 1;
        let candidate = Allocation {
            matrix: matrix.to_vec(),
            totals: totals.to_vec(),
            profile: profile(totals),
        };
        let replace = match &state.best {
            None => true,
            Some(best) => {
                let ordering = compare_allocations(p, &candidate, best, state.objective);
                ordering == std::cmp::Ordering::Greater
                    || (ordering == std::cmp::Ordering::Equal && candidate.matrix < best.matrix)
            }
        };
        if replace {
            state.best = Some(candidate);
        }
        debug_assert!(feasible(p, matrix, totals));
        return;
    }
    if ai == p.agents.len() {
        let next = wi + 1;
        let next_supply = p.wells.get(next).map_or(0, |w| w.supply);
        visit(p, next, 0, next_supply, matrix, totals, state);
        return;
    }

    let well = &p.wells[wi];
    let agent = &p.agents[ai];
    let residual_cap = agent.cap.saturating_sub(totals[ai]);
    let max_edge = if well.eligible.binary_search(&agent.id).is_ok() {
        remaining.min(residual_cap)
    } else {
        0
    };
    for amount in 0..=max_edge {
        matrix[wi][ai] = amount;
        totals[ai] += amount;
        visit(p, wi, ai + 1, remaining - amount, matrix, totals, state);
        totals[ai] -= amount;
    }
    matrix[wi][ai] = 0;
}

fn oracle_with_objective(
    problem: &Problem,
    objective: Objective,
) -> (Allocation, u64, u64) {
    let p = canonical(problem.clone());
    validate(&p);
    let mut matrix = vec![vec![0; p.agents.len()]; p.wells.len()];
    let mut totals = vec![0; p.agents.len()];
    let mut state = Search {
        best: None,
        nodes: 0,
        leaves: 0,
        objective,
    };
    visit(
        &p,
        0,
        0,
        p.wells.first().map_or(0, |w| w.supply),
        &mut matrix,
        &mut totals,
        &mut state,
    );
    (
        state.best.expect("zero allocation is feasible"),
        state.nodes,
        state.leaves,
    )
}

fn oracle(problem: &Problem) -> (Allocation, u64, u64) {
    oracle_with_objective(problem, Objective::AbsoluteAmount)
}

fn from_matrix(matrix: Vec<Vec<u32>>) -> Allocation {
    let n = matrix.first().map_or(0, Vec::len);
    let totals: Vec<u32> = (0..n).map(|ai| matrix.iter().map(|row| row[ai]).sum()).collect();
    Allocation {
        profile: profile(&totals),
        totals,
        matrix,
    }
}

// Cap-safe but order-dependent: fully process each well before considering the next.
fn sequential(problem: &Problem, reverse: bool) -> Allocation {
    let p = canonical(problem.clone());
    validate(&p);
    let mut matrix = vec![vec![0; p.agents.len()]; p.wells.len()];
    let mut totals = vec![0; p.agents.len()];
    let mut order: Vec<usize> = (0..p.wells.len()).collect();
    if reverse {
        order.reverse();
    }
    for wi in order {
        let mut remaining = p.wells[wi].supply;
        while remaining > 0 {
            let next = p.wells[wi]
                .eligible
                .iter()
                .filter_map(|id| p.agents.iter().position(|a| a.id == *id))
                .filter(|&ai| totals[ai] < p.agents[ai].cap)
                .min_by_key(|&ai| (totals[ai], p.agents[ai].id));
            let Some(ai) = next else { break };
            matrix[wi][ai] += 1;
            totals[ai] += 1;
            remaining -= 1;
        }
    }
    Allocation {
        matrix,
        profile: profile(&totals),
        totals,
    }
}

// Negative control: each source sees a fresh, local recipient-cap ledger.
fn independent_sources(problem: &Problem) -> Allocation {
    let p = canonical(problem.clone());
    validate(&p);
    let mut matrix = vec![vec![0; p.agents.len()]; p.wells.len()];
    for (wi, well) in p.wells.iter().enumerate() {
        let mut local = vec![0; p.agents.len()];
        let mut remaining = well.supply;
        while remaining > 0 {
            let next = well
                .eligible
                .iter()
                .filter_map(|id| p.agents.iter().position(|a| a.id == *id))
                .filter(|&ai| local[ai] < p.agents[ai].cap)
                .min_by_key(|&ai| (local[ai], p.agents[ai].id));
            let Some(ai) = next else { break };
            matrix[wi][ai] += 1;
            local[ai] += 1;
            remaining -= 1;
        }
    }
    from_matrix(matrix)
}

fn cap_violation(p: &Problem, a: &Allocation) -> u32 {
    canonical(p.clone())
        .agents
        .iter()
        .zip(&a.totals)
        .map(|(agent, total)| total.saturating_sub(agent.cap))
        .sum()
}

fn l1(left: &Allocation, right: &Allocation) -> u32 {
    left.totals
        .iter()
        .zip(&right.totals)
        .map(|(a, b)| a.abs_diff(*b))
        .sum()
}

fn max_delta(left: &Allocation, right: &Allocation) -> u32 {
    left.totals
        .iter()
        .zip(&right.totals)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

fn first_regret(optimum: &[u32], candidate: &[u32]) -> Option<(usize, u32, u32)> {
    optimum
        .iter()
        .zip(candidate)
        .enumerate()
        .find_map(|(i, (best, got))| (best != got).then_some((i, *best, *got)))
}

fn totals_by_id(p: &Problem, a: &Allocation) -> String {
    canonical(p.clone())
        .agents
        .iter()
        .zip(&a.totals)
        .map(|(agent, total)| format!("{}:{}", agent.id, total))
        .collect::<Vec<_>>()
        .join(",")
}

fn audit() {
    let p = fixture();
    let (best, nodes, leaves) = oracle(&p);
    let (capacity_best, capacity_nodes, capacity_leaves) =
        oracle_with_objective(&p, Objective::CapacityNormalized);
    let forward = sequential(&p, false);
    let reverse = sequential(&p, true);
    let independent = independent_sources(&p);

    assert_eq!(best.profile, vec![8, 8, 8]);
    assert_eq!(best.matrix, vec![vec![4, 8, 0], vec![4, 0, 8]]);
    assert_eq!(capacity_best.totals, vec![7, 9, 8]);
    assert_eq!(capacity_best.matrix, vec![vec![3, 9, 0], vec![4, 0, 8]]);
    assert_eq!(
        capacity_profile(&canonical(p.clone()).agents, &capacity_best.totals),
        vec![(2, 3), (3, 4), (7, 9)]
    );
    assert_eq!(capacity_nodes, nodes);
    assert_eq!(capacity_leaves, leaves);
    assert!(feasible(&p, &best.matrix, &best.totals));
    assert!(feasible(&p, &capacity_best.matrix, &capacity_best.totals));
    assert_eq!(
        compare_allocations(&p, &capacity_best, &best, Objective::CapacityNormalized),
        std::cmp::Ordering::Greater
    );
    assert!(feasible(&p, &forward.matrix, &forward.totals));
    assert!(feasible(&p, &reverse.matrix, &reverse.totals));
    assert_eq!(forward.totals, vec![9, 6, 9]);
    assert_eq!(reverse.totals, vec![9, 9, 6]);
    assert_eq!(l1(&forward, &reverse), 6);
    assert_eq!(max_delta(&forward, &reverse), 3);
    assert_eq!(cap_violation(&p, &independent), 3);

    println!("oracle_version=0.1.0");
    println!("scenario=overlapping_two_wells_three_agents");
    println!("quantum=one_third_of_original_unit");
    println!("oracle_profile={:?}", best.profile);
    println!("oracle_totals_by_agent_id={}", totals_by_id(&p, &best));
    println!("oracle_matrix={:?}", best.matrix);
    println!("oracle_search_nodes={nodes}");
    println!("oracle_feasible_leaves={leaves}");
    println!(
        "capacity_normalized_oracle_profile={:?}",
        capacity_profile(&canonical(p.clone()).agents, &capacity_best.totals)
    );
    println!(
        "capacity_normalized_oracle_totals_by_agent_id={}",
        totals_by_id(&p, &capacity_best)
    );
    println!("capacity_normalized_oracle_matrix={:?}", capacity_best.matrix);
    println!("sequential_forward_totals_by_id={}", totals_by_id(&p, &forward));
    println!("sequential_reverse_totals_by_id={}", totals_by_id(&p, &reverse));
    println!("sequential_order_sensitivity_l1={}", l1(&forward, &reverse));
    println!(
        "sequential_order_sensitivity_max_agent_delta={}",
        max_delta(&forward, &reverse)
    );
    if let Some((rank, ideal, got)) = first_regret(&best.profile, &forward.profile) {
        println!("forward_first_regret_rank={rank}");
        println!("forward_optimum_at_rank={ideal}");
        println!("forward_observed_at_rank={got}");
        println!("forward_regret_at_first_difference={}", ideal.saturating_sub(got));
    }
    println!(
        "independent_source_totals_by_id={}",
        totals_by_id(&p, &independent)
    );
    println!(
        "independent_source_cap_violation={}",
        cap_violation(&p, &independent)
    );
    println!("audit_assertions=pass");
}

fn main() {
    audit();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_finds_exact_joint_leximin_fixture() {
        let p = fixture();
        let (best, nodes, leaves) = oracle(&p);
        assert_eq!(best.profile, vec![8, 8, 8]);
        assert_eq!(best.totals, vec![8, 8, 8]);
        assert_eq!(best.matrix, vec![vec![4, 8, 0], vec![4, 0, 8]]);
        assert!(nodes > 0 && leaves > 0);
        assert!(feasible(&p, &best.matrix, &best.totals));
    }

    #[test]
    fn capacity_normalized_objective_can_choose_a_different_valid_allocation() {
        let p = fixture();
        let (absolute, _, _) = oracle_with_objective(&p, Objective::AbsoluteAmount);
        let (normalized, _, _) = oracle_with_objective(&p, Objective::CapacityNormalized);
        assert_eq!(absolute.totals, vec![8, 8, 8]);
        assert_eq!(normalized.totals, vec![7, 9, 8]);
        assert_eq!(normalized.profile, vec![7, 8, 9]);
        assert_eq!(
            capacity_profile(&canonical(p.clone()).agents, &normalized.totals),
            vec![(2, 3), (3, 4), (7, 9)]
        );
        assert_eq!(
            compare_allocations(&p, &normalized, &absolute, Objective::CapacityNormalized),
            std::cmp::Ordering::Greater
        );
        assert!(feasible(&p, &normalized.matrix, &normalized.totals));
    }

    #[test]
    fn oracle_is_invariant_to_input_order() {
        let p = fixture();
        let mut permuted = p.clone();
        permuted.wells.reverse();
        permuted.agents.reverse();
        for well in &mut permuted.wells {
            well.eligible.reverse();
        }
        assert_eq!(oracle(&p).0, oracle(&permuted).0);
    }

    #[test]
    fn sequential_policy_exposes_order_sensitivity_without_breaking_caps() {
        let p = fixture();
        let forward = sequential(&p, false);
        let reverse = sequential(&p, true);
        assert_eq!(forward.totals, vec![9, 6, 9]);
        assert_eq!(reverse.totals, vec![9, 9, 6]);
        assert_eq!(forward.profile, vec![6, 9, 9]);
        assert_eq!(l1(&forward, &reverse), 6);
        assert_eq!(max_delta(&forward, &reverse), 3);
        assert!(feasible(&p, &forward.matrix, &forward.totals));
        assert!(feasible(&p, &reverse.matrix, &reverse.totals));
    }

    #[test]
    fn independent_per_well_negative_control_detects_double_counting() {
        let p = fixture();
        let bad = independent_sources(&p);
        assert_eq!(bad.totals, vec![12, 6, 6]);
        assert_eq!(cap_violation(&p, &bad), 3);
        assert!(!feasible(&p, &bad.matrix, &bad.totals));
    }

    #[test]
    fn exhaustive_small_overlap_graphs_are_feasible_and_order_invariant() {
        let ids = [1_u64, 2, 3];
        let caps = [2_u32, 2, 1];
        let mut checked = 0_u32;

        for left_mask in 0_u32..8 {
            for right_mask in 0_u32..8 {
                for left_supply in 0_u32..=2 {
                    for right_supply in 0_u32..=2 {
                        let eligible_for = |mask: u32| {
                            ids.iter()
                                .enumerate()
                                .filter_map(|(index, id)| (mask & (1_u32 << index) != 0).then_some(*id))
                                .collect::<Vec<_>>()
                        };
                        let p = Problem {
                            wells: vec![
                                Well {
                                    id: 10,
                                    supply: left_supply,
                                    eligible: eligible_for(left_mask),
                                },
                                Well {
                                    id: 20,
                                    supply: right_supply,
                                    eligible: eligible_for(right_mask),
                                },
                            ],
                            agents: ids
                                .iter()
                                .zip(caps)
                                .map(|(id, cap)| Agent { id: *id, cap })
                                .collect(),
                        };

                        let (expected, _, _) = oracle(&p);
                        let (expected_normalized, _, _) =
                            oracle_with_objective(&p, Objective::CapacityNormalized);
                        assert!(
                            feasible(&p, &expected.matrix, &expected.totals),
                            "infeasible absolute oracle output for masks {left_mask}/{right_mask}, supplies {left_supply}/{right_supply}"
                        );
                        assert!(
                            feasible(&p, &expected_normalized.matrix, &expected_normalized.totals),
                            "infeasible normalized oracle output for masks {left_mask}/{right_mask}, supplies {left_supply}/{right_supply}"
                        );

                        let mut permuted = p.clone();
                        permuted.wells.reverse();
                        permuted.agents.reverse();
                        for well in &mut permuted.wells {
                            well.eligible.reverse();
                        }
                        let (actual, _, _) = oracle(&permuted);
                        let (actual_normalized, _, _) =
                            oracle_with_objective(&permuted, Objective::CapacityNormalized);
                        assert_eq!(
                            expected, actual,
                            "absolute-objective permutation mismatch for masks {left_mask}/{right_mask}, supplies {left_supply}/{right_supply}"
                        );
                        assert_eq!(
                            expected_normalized, actual_normalized,
                            "normalized-objective permutation mismatch for masks {left_mask}/{right_mask}, supplies {left_supply}/{right_supply}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert_eq!(checked, 576);
    }

    #[test]
    fn zero_supply_and_zero_caps_remain_feasible() {
        let p = Problem {
            wells: vec![Well {
                id: 9,
                supply: 0,
                eligible: vec![1, 2],
            }],
            agents: vec![Agent { id: 1, cap: 0 }, Agent { id: 2, cap: 0 }],
        };
        let (best, _, _) = oracle(&p);
        assert_eq!(best.profile, vec![0, 0]);
        assert_eq!(best.matrix, vec![vec![0, 0]]);
        assert!(feasible(&p, &best.matrix, &best.totals));
    }
}
