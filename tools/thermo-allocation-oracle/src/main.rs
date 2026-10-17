// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exhaustive finite-grid oracle for tiny overlapping-well allocation graphs.
//! Research verifier only; this is neither the production allocator nor a
//! continuous-rational proof. Integer quanta make this fixture arithmetic exact.

#[derive(Clone, Debug, Eq, PartialEq)]
struct Agent { id: u64, cap: u32 }

#[derive(Clone, Debug, Eq, PartialEq)]
struct Well { id: u64, supply: u32, eligible: Vec<u64> }

#[derive(Clone, Debug, Eq, PartialEq)]
struct Problem { wells: Vec<Well>, agents: Vec<Agent> }

#[derive(Clone, Debug, Eq, PartialEq)]
struct Allocation {
    // Rows: wells by stable ID; columns and totals: agents by stable ID.
    matrix: Vec<Vec<u32>>,
    totals: Vec<u32>,
    profile: Vec<u32>,
}

struct Search {
    best: Option<Allocation>,
    nodes: u64,
    leaves: u64,
}

fn fixture() -> Problem {
    // One quantum = 1/3 of an original unit: supplies 4,4 and caps 3,4,4.
    Problem {
        wells: vec![
            Well { id: 10, supply: 12, eligible: vec![1, 2] },
            Well { id: 20, supply: 12, eligible: vec![1, 3] },
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
    assert!(p.wells.len() <= 3 && p.agents.len() <= 4, "tiny-oracle size bound exceeded");
    assert!(p.wells.iter().all(|w| w.supply <= 12), "per-well supply bound exceeded");
    assert!(p.wells.windows(2).all(|x| x[0].id != x[1].id), "duplicate well ID");
    assert!(p.agents.windows(2).all(|x| x[0].id != x[1].id), "duplicate agent ID");
    for w in &p.wells {
        assert!(w.eligible.iter().all(|id| p.agents.iter().any(|a| a.id == *id)),
            "eligibility references unknown agent");
    }
}

fn profile(totals: &[u32]) -> Vec<u32> {
    let mut p = totals.to_vec();
    p.sort_unstable();
    p
}

fn feasible(p: &Problem, matrix: &[Vec<u32>], totals: &[u32]) -> bool {
    let p = canonical(p.clone());
    if matrix.len() != p.wells.len() || totals.len() != p.agents.len() {
        return false;
    }
    for (wi, w) in p.wells.iter().enumerate() {
        if matrix[wi].len() != p.agents.len()
            || matrix[wi].iter().copied().sum::<u32>() > w.supply {
        {
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
            Some(best) if candidate.profile > best.profile => true,
            Some(best) if candidate.profile == best.profile => candidate.matrix < best.matrix,
            Some(_) => false,
        };
        if replace { state.best = Some(candidate); }
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

fn oracle(problem: &Problem) -> (Allocation, u64, u64) {
    let p = canonical(problem.clone());
    validate(&p);
    let mut matrix = vec![vec![0; p.agents.len()]; p.wells.len()];
    let mut totals = vec![0; p.agents.len()];
    let mut state = Search { best: None, nodes: 0, leaves: 0 };
    visit(&p, 0, 0, p.wells.first().map_or(0, |w| w.supply),
        &mut matrix, &mut totals, &mut state);
    (state.best.expect("zero allocation is feasible"), state.nodes, state.leaves)
}

fn from_matrix(matrix: Vec<Vec<u32>>) -> Allocation {
    let n = matrix.first().map_or(0, Vec::len);
    let totals: Vec<u32> = (0..n).map(|ai| matrix.iter().map(|row| row[ai]).sum()).collect();
    Allocation { profile: profile(&totals), totals, matrix }
}

// Cap-safe but order-dependent: fully process each well before considering the next.
fn sequential(problem: &Problem, reverse: bool) -> Allocation {
    let p = canonical(problem.clone());
    validate(&p);
    let mut matrix = vec![vec![0; p.agents.len()]; p.wells.len()];
    let mut totals = vec![0; p.agents.len()];
    let mut order: Vec<usize> = (0..p.wells.len()).collect();
    if reverse { order.reverse(); }
    for wi in order {
        let mut remaining = p.wells[wi].supply;
        while remaining > 0 {
            let next = p.wells[wi].eligible.iter()
                .filter_map(|id| p.agents.iter().position(|a| a.id == *id))
                .filter(|&ai| totals[ai] < p.agents[ai].cap)
                .min_by_key(|&ai| (totals[ai], p.agents[ai].id));
            let Some(ai) = next else { break };
            matrix[wi][ai] += 1;
            totals[ai] += 1;
            remaining -= 1;
        }
    }
    Allocation { matrix, profile: profile(&totals), totals }
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
            let next = well.eligible.iter()
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
    canonical(p.clone()).agents.iter().zip(&a.totals)
        .map(|(agent, total)| total.saturating_sub(agent.cap)).sum()
}

fn l1(left: &Allocation, right: &Allocation) -> u32 {
    left.totals.iter().zip(&right.totals).map(|(a, b)| a.abs_diff(*b)).sum()
}

fn max_delta(left: &Allocation, right: &Allocation) -> u32 {
    left.totals.iter().zip(&right.totals).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
}

fn first_regret(optimum: &[u32], candidate: &[u32]) -> Option<(usize, u32, u32)> {
    optimum.iter().zip(candidate).enumerate()
        .find_map(|(i, (best, got))| (best != got).then_some((i, *best, *got)))
}

fn totals_by_id(p: &Problem, a: &Allocation) -> String {
    canonical(p.clone()).agents.iter().zip(&a.totals)
        .map(|(agent, total)| format!("{}:{}", agent.id, total))
        .collect::<Vec<_>>().join(",")
}

fn audit() {
    let p = fixture();
    let (best, nodes, leaves) = oracle(&p);
    let forward = sequential(&p, false);
    let reverse = sequential(&p, true);
    let independent = independent_sources(&p);

    assert_eq!(best.profile, vec![8, 8, 8]);
    assert_eq!(best.matrix, vec![vec![4, 8, 0], vec![4, 0, 8]]);
    assert!(feasible(&p, &best.matrix, &best.totals));
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
    println!("sequential_forward_totals_by_id={}", totals_by_id(&p, &forward));
    println!("sequential_reverse_totals_by_id={}", totals_by_id(&p, &reverse));
    println!("sequential_order_sensitivity_l1={}", l1(&forward, &reverse));
    println!("sequential_order_sensitivity_max_agent_delta={}", max_delta(&forward, &reverse));
    if let Some((rank, ideal, got)) = first_regret(&best.profile, &forward.profile) {
        println!("forward_first_regret_rank={rank}");
        println!("forward_optimum_at_rank={ideal}");
        println!("forward_observed_at_rank={got}");
        println!("forward_regret_at_first_difference={}", ideal.saturating_sub(got));
    }
    println!("independent_source_totals_by_id={}", totals_by_id(&p, &independent));
    println!("independent_source_cap_violation={}", cap_violation(&p, &independent));
    println!("audit_assertions=pass");
}

fn main() { audit(); }

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
    fn oracle_is_invariant_to_input_order() {
        let p = fixture();
        let mut permuted = p.clone();
        permuted.wells.reverse();
        permuted.agents.reverse();
        for well in &mut permuted.wells { well.eligible.reverse(); }
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
    fn zero_supply_and_zero_caps_remain_feasible() {
        let p = Problem {
            wells: vec![Well { id: 9, supply: 0, eligible: vec![1, 2] }],
            agents: vec![Agent { id: 1, cap: 0 }, Agent { id: 2, cap: 0 }],
        };
        let (best, _, _) = oracle(&p);
        assert_eq!(best.profile, vec![0, 0]);
        assert_eq!(best.matrix, vec![vec![0, 0]]);
        assert!(feasible(&p, &best.matrix, &best.totals));
    }
}
