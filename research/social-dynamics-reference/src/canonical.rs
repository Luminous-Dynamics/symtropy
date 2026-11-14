//! Small formal social-dynamics baselines with explicit contracts.
//!
//! These functions implement canonical consensus mechanisms for invariant and
//! replication work. Their existence is not by itself evidence of empirical
//! validity or successful replication of a published study.

/// Construct an undirected ring lattice with an even, fixed degree.
///
/// Each node connects to the degree / 2 nearest neighbors on both sides.
/// The result has no self-edges or duplicate edges when inputs are valid.
pub fn ring_lattice(population: usize, degree: usize) -> Result<Vec<Vec<usize>>, String> {
    if population < 3 {
        return Err("population must be at least 3".into());
    }
    if degree == 0 || degree >= population || degree % 2 != 0 {
        return Err("degree must be positive, even, and smaller than population".into());
    }

    let half = degree / 2;
    let mut graph = vec![Vec::with_capacity(degree); population];
    for (agent, neighbors) in graph.iter_mut().enumerate() {
        for offset in 1..=half {
            neighbors.push((agent + population - offset) % population);
            neighbors.push((agent + offset) % population);
        }
        neighbors.sort_unstable();
        if neighbors.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("degree creates duplicate ring-lattice neighbors".into());
        }
    }
    Ok(graph)
}

fn validate_graph(
    graph: &[Vec<usize>],
    population: usize,
    require_neighbors: bool,
) -> Result<(), String> {
    if graph.len() != population {
        return Err("graph length must equal population".into());
    }
    for (agent, neighbors) in graph.iter().enumerate() {
        if require_neighbors && neighbors.is_empty() {
            return Err(format!("agent {agent} has no neighbors"));
        }
        for (slot, neighbor) in neighbors.iter().copied().enumerate() {
            if neighbor >= population {
                return Err(format!("neighbor index {neighbor} is out of range"));
            }
            if neighbor == agent {
                return Err(format!("agent {agent} has a self-edge"));
            }
            if neighbors[..slot].contains(&neighbor) {
                return Err(format!("agent {agent} has duplicate neighbor {neighbor}"));
            }
        }
    }
    Ok(())
}

/// Perform one asynchronous voter-model event.
///
/// The event schedule and sampled neighbor are supplied by the caller to make
/// replay and common-random-number experiments explicit. Exactly one selected
/// agent copies one of its neighbors; all other opinions remain unchanged.
pub fn voter_update(
    opinions: &[bool],
    graph: &[Vec<usize>],
    selected_agent: usize,
    selected_neighbor_slot: usize,
) -> Result<Vec<bool>, String> {
    validate_graph(graph, opinions.len(), true)?;
    let neighbors = graph
        .get(selected_agent)
        .ok_or_else(|| "selected agent is out of range".to_owned())?;
    let selected_neighbor = neighbors
        .get(selected_neighbor_slot)
        .ok_or_else(|| "selected neighbor slot is out of range".to_owned())?;
    let mut next = opinions.to_vec();
    next[selected_agent] = opinions[*selected_neighbor];
    Ok(next)
}

/// Perform one synchronous DeGroot averaging step.
///
/// Each agent averages its own opinion with those of its outgoing neighbors.
/// Opinions must be finite and lie in [0, 1]. The update is row-stochastic and
/// therefore remains inside the prior convex hull. The population mean is
/// preserved when the graph/update matrix is doubly stochastic (for example,
/// this undirected regular ring lattice).
pub fn degroot_step(
    opinions: &[f64],
    graph: &[Vec<usize>],
) -> Result<Vec<f64>, String> {
    if opinions.is_empty() {
        return Err("population must not be empty".into());
    }
    if opinions
        .iter()
        .any(|opinion| !opinion.is_finite() || !(0.0..=1.0).contains(opinion))
    {
        return Err("opinions must be finite values in [0, 1]".into());
    }
    validate_graph(graph, opinions.len(), false)?;

    let mut next = Vec::with_capacity(opinions.len());
    for (agent, neighbors) in graph.iter().enumerate() {
        let mut sum = opinions[agent];
        for neighbor in neighbors {
            sum += opinions[*neighbor];
        }
        next.push(sum / (neighbors.len() + 1) as f64);
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_lattice_has_unique_symmetric_neighbors() {
        let graph = ring_lattice(17, 4).unwrap();
        assert_eq!(graph.len(), 17);
        for (agent, neighbors) in graph.iter().enumerate() {
            assert_eq!(neighbors.len(), 4);
            assert!(!neighbors.contains(&agent));
            assert_eq!(
                neighbors.iter().copied().collect::<std::collections::BTreeSet<_>>().len(),
                4
            );
            for neighbor in neighbors {
                assert!(graph[*neighbor].contains(&agent));
            }
        }
    }

    #[test]
    fn ring_lattice_rejects_invalid_degrees() {
        assert!(ring_lattice(2, 2).is_err());
        assert!(ring_lattice(10, 0).is_err());
        assert!(ring_lattice(10, 3).is_err());
        assert!(ring_lattice(10, 10).is_err());
    }

    #[test]
    fn voter_model_consensus_states_are_absorbing() {
        let graph = ring_lattice(12, 4).unwrap();
        let all_zero = vec![false; 12];
        let all_one = vec![true; 12];
        assert_eq!(voter_update(&all_zero, &graph, 5, 2).unwrap(), all_zero);
        assert_eq!(voter_update(&all_one, &graph, 5, 2).unwrap(), all_one);
    }

    #[test]
    fn voter_update_changes_only_the_selected_agent() {
        let graph = ring_lattice(8, 2).unwrap();
        let opinions = vec![false, false, true, false, false, false, false, false];
        let next = voter_update(&opinions, &graph, 1, 1).unwrap();
        assert!(next[1]);
        for index in 0..opinions.len() {
            if index != 1 {
                assert_eq!(next[index], opinions[index]);
            }
        }
    }

    #[test]
    fn degroot_step_stays_in_convex_hull_and_preserves_regular_graph_mean() {
        let graph = ring_lattice(12, 4).unwrap();
        let opinions = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.35];
        let prior_mean = opinions.iter().sum::<f64>() / opinions.len() as f64;
        let next = degroot_step(&opinions, &graph).unwrap();
        let next_mean = next.iter().sum::<f64>() / next.len() as f64;
        assert!(next.iter().all(|value| (0.0..=1.0).contains(value)));
        assert!((next_mean - prior_mean).abs() < 1e-12);
    }

    #[test]
    fn degroot_rejects_nonfinite_and_out_of_range_opinions() {
        let graph = ring_lattice(4, 2).unwrap();
        assert!(degroot_step(&[0.0, 0.2, f64::NAN, 1.0], &graph).is_err());
        assert!(degroot_step(&[0.0, 0.2, 1.1, 1.0], &graph).is_err());
        assert!(degroot_step(&[], &[]).is_err());
    }

    #[test]
    fn consensus_baselines_fail_closed_on_bad_graphs() {
        let opinions = vec![false, true, false];
        let self_edge_graph = vec![vec![0], vec![0], vec![1]];
        assert!(voter_update(&opinions, &self_edge_graph, 0, 0).is_err());
        let out_of_range_graph = vec![vec![9], vec![0], vec![1]];
        assert!(degroot_step(&[0.1, 0.2, 0.3], &out_of_range_graph).is_err());
    }
}
