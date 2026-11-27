#![forbid(unsafe_code)]

//! Checked lower-bound elimination for a single-commodity DIMACS MCF instance.
//! This is preprocessing only; it does not solve or certify the residual MCF.

use crate::McfInstance;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfResidualArc {
    /// Original one-based input ordinal; parallel arcs remain distinct.
    pub ordinal: u32,
    pub tail: u32,
    pub head: u32,
    pub capacity: i64,
    pub unit_cost: i32,
    pub source_line: usize,
}

/// Residual network after fixing every original arc lower bound.
///
/// Positive adjusted supply means required residual net outflow. Supply
/// vectors are one-based with a zero sentinel at index zero. Lower bounds
/// align with arcs in original input order. Original objective equals residual
/// objective plus lower_bound_cost_offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfResidualInstance {
    pub node_count: u32,
    pub arc_count: u32,
    pub original_supplies: Vec<i64>,
    pub adjusted_supplies: Vec<i128>,
    pub lower_bounds: Vec<i64>,
    pub arcs: Vec<McfResidualArc>,
    pub lower_bound_cost_offset: i128,
}

impl McfResidualInstance {
    pub fn supply_balance(&self) -> i128 {
        self.adjusted_supplies.iter().skip(1).copied().sum()
    }

    pub fn is_supply_balanced(&self) -> bool {
        self.supply_balance() == 0
    }

    fn validate(&self) -> Result<(), McfTransformError> {
        let expected_supply_len = usize::try_from(self.node_count)
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| McfTransformError::new("node supply vector length overflow"))?;
        if self.original_supplies.len() != expected_supply_len
            || self.original_supplies.first().copied() != Some(0)
            || self.adjusted_supplies.len() != expected_supply_len
            || self.adjusted_supplies.first().copied() != Some(0)
        {
            return Err(McfTransformError::new(
                "original/adjusted supply vector shape or zero sentinel is invalid",
            ));
        }
        let mut expected_adjusted = Vec::new();
        reserve_exact(
            &mut expected_adjusted,
            expected_supply_len,
            "recomputed adjusted supply vector",
        )?;
        expected_adjusted.extend(
            self.original_supplies
                .iter()
                .map(|&supply| i128::from(supply)),
        );
        if self.arc_count as usize != self.arcs.len()
            || self.lower_bounds.len() != self.arcs.len()
        {
            return Err(McfTransformError::new(
                "residual arc and lower-bound vectors do not match declared arc count",
            ));
        }
        let mut expected_offset = 0_i128;
        for (index, (arc, &lower)) in self.arcs.iter().zip(&self.lower_bounds).enumerate() {
            let ordinal = u32::try_from(index + 1)
                .map_err(|_| McfTransformError::new("arc ordinal exceeds u32"))?;
            if arc.ordinal != ordinal
                || arc.tail == 0
                || arc.head == 0
                || arc.tail > self.node_count
                || arc.head > self.node_count
            {
                return Err(McfTransformError::new(format!(
                    "invalid ordinal or endpoint on residual arc {ordinal}"
                )));
            }
            if lower < 0 || arc.capacity < 0 {
                return Err(McfTransformError::new(format!(
                    "negative lower bound or residual capacity on arc {ordinal}"
                )));
            }
            lower.checked_add(arc.capacity).ok_or_else(|| {
                McfTransformError::new(format!(
                    "lower bound plus residual capacity overflows i64 on arc {ordinal}"
                ))
            })?;
            expected_adjusted[arc.tail as usize] = expected_adjusted[arc.tail as usize]
                .checked_sub(i128::from(lower))
                .ok_or_else(|| McfTransformError::new("recomputed tail supply underflow"))?;
            expected_adjusted[arc.head as usize] = expected_adjusted[arc.head as usize]
                .checked_add(i128::from(lower))
                .ok_or_else(|| McfTransformError::new("recomputed head supply overflow"))?;
            let contribution = i128::from(lower)
                .checked_mul(i128::from(arc.unit_cost))
                .ok_or_else(|| {
                    McfTransformError::new(format!(
                        "lower-bound cost multiplication overflow on arc {ordinal}"
                    ))
                })?;
            expected_offset = expected_offset.checked_add(contribution).ok_or_else(|| {
                McfTransformError::new("lower-bound cost offset overflowed i128")
            })?;
        }
        if expected_offset != self.lower_bound_cost_offset {
            return Err(McfTransformError::new(
                "lower-bound cost offset does not match arc baselines",
            ));
        }
        if expected_adjusted != self.adjusted_supplies {
            return Err(McfTransformError::new(
                "adjusted supplies do not match original supplies and lower bounds",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McfTransformError {
    pub message: String,
}

impl McfTransformError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for McfTransformError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DIMACS MCF lower-bound transform: {}", self.message)
    }
}

impl std::error::Error for McfTransformError {}

fn reserve_exact<T>(
    target: &mut Vec<T>,
    capacity: usize,
    label: &str,
) -> Result<(), McfTransformError> {
    target
        .try_reserve_exact(capacity)
        .map_err(|error| McfTransformError::new(format!("cannot reserve {label}: {error}")))
}

/// Remove lower bounds using the positive-supply convention:
///
///     sum(outflow) - sum(inflow) = supply
///
/// For each arc u -> v with lower bound l, residual capacity is upper - l;
/// adjusted supply at u is reduced by l; adjusted supply at v is increased by
/// l; and l * unit_cost is added to the objective constant. Thus original
/// flow f = l + x for residual flow x, and original cost is residual cost
/// plus the returned objective constant.
///
/// Node balances and the cost offset use i128. Capacities remain i64. The
/// source instance is validated rather than repaired.
pub fn remove_lower_bounds(
    instance: &McfInstance,
) -> Result<McfResidualInstance, McfTransformError> {
    let expected_supply_len = usize::try_from(instance.node_count)
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| McfTransformError::new("node supply vector length overflow"))?;
    if instance.node_supplies.len() != expected_supply_len
        || instance.node_supplies.first().copied() != Some(0)
    {
        return Err(McfTransformError::new(
            "node supply vector shape or zero sentinel is invalid",
        ));
    }
    if instance.arc_count as usize != instance.arcs.len() {
        return Err(McfTransformError::new(format!(
            "arc_count declares {}, but {} arc records are present",
            instance.arc_count,
            instance.arcs.len()
        )));
    }

    let mut adjusted_supplies = Vec::new();
    reserve_exact(
        &mut adjusted_supplies,
        expected_supply_len,
        "adjusted node supply vector",
    )?;
    adjusted_supplies.extend(instance.node_supplies.iter().map(|&supply| i128::from(supply)));

    let mut lower_bounds = Vec::new();
    reserve_exact(
        &mut lower_bounds,
        instance.arcs.len(),
        "lower-bound baseline vector",
    )?;

    let mut residual_arcs = Vec::new();
    reserve_exact(
        &mut residual_arcs,
        instance.arcs.len(),
        "residual arc vector",
    )?;

    let mut lower_bound_cost_offset = 0_i128;
    for (index, arc) in instance.arcs.iter().enumerate() {
        let expected_ordinal = u32::try_from(index + 1)
            .map_err(|_| McfTransformError::new("arc ordinal exceeds u32"))?;
        if arc.ordinal != expected_ordinal {
            return Err(McfTransformError::new(format!(
                "arc at input position {} has ordinal {}, expected {expected_ordinal}",
                index + 1,
                arc.ordinal
            )));
        }
        if arc.tail == 0
            || arc.head == 0
            || arc.tail > instance.node_count
            || arc.head > instance.node_count
        {
            return Err(McfTransformError::new(format!(
                "arc {} has endpoint outside 1..={}",
                arc.ordinal, instance.node_count
            )));
        }
        if arc.lower_bound < 0
            || arc.upper_bound < 0
            || arc.upper_bound < arc.lower_bound
        {
            return Err(McfTransformError::new(format!(
                "arc {} has invalid bounds [{}, {}]",
                arc.ordinal, arc.lower_bound, arc.upper_bound
            )));
        }

        let capacity = arc.upper_bound.checked_sub(arc.lower_bound).ok_or_else(|| {
            McfTransformError::new(format!("residual capacity overflow on arc {}", arc.ordinal))
        })?;
        let lower = i128::from(arc.lower_bound);
        let tail = arc.tail as usize;
        let head = arc.head as usize;
        adjusted_supplies[tail] = adjusted_supplies[tail].checked_sub(lower).ok_or_else(|| {
            McfTransformError::new(format!("adjusted supply underflow at node {}", arc.tail))
        })?;
        adjusted_supplies[head] = adjusted_supplies[head].checked_add(lower).ok_or_else(|| {
            McfTransformError::new(format!("adjusted supply overflow at node {}", arc.head))
        })?;

        let contribution = lower
            .checked_mul(i128::from(arc.unit_cost))
            .ok_or_else(|| {
                McfTransformError::new(format!(
                    "lower-bound cost multiplication overflow on arc {}",
                    arc.ordinal
                ))
            })?;
        lower_bound_cost_offset = lower_bound_cost_offset
            .checked_add(contribution)
            .ok_or_else(|| {
                McfTransformError::new("total lower-bound cost offset overflowed i128")
            })?;

        lower_bounds.push(arc.lower_bound);
        residual_arcs.push(McfResidualArc {
            ordinal: arc.ordinal,
            tail: arc.tail,
            head: arc.head,
            capacity,
            unit_cost: arc.unit_cost,
            source_line: arc.source_line,
        });
    }

    let mut original_supplies = Vec::new();
    reserve_exact(
        &mut original_supplies,
        expected_supply_len,
        "original supply witness",
    )?;
    original_supplies.extend(instance.node_supplies.iter().copied());

    let residual = McfResidualInstance {
        node_count: instance.node_count,
        arc_count: instance.arc_count,
        original_supplies,
        adjusted_supplies,
        lower_bounds,
        arcs: residual_arcs,
        lower_bound_cost_offset,
    };
    residual.validate()?;
    Ok(residual)
}

/// Restore one original-space flow amount per arc from a residual witness.
pub fn restore_original_flows(
    residual: &McfResidualInstance,
    residual_flows: &[i64],
) -> Result<Vec<i64>, McfTransformError> {
    residual.validate()?;
    if residual_flows.len() != residual.arcs.len() {
        return Err(McfTransformError::new(
            "residual flow vector must cover every original arc",
        ));
    }
    let mut original_flows = Vec::new();
    reserve_exact(
        &mut original_flows,
        residual.arcs.len(),
        "original flow witness",
    )?;
    for ((arc, &lower), &flow) in residual
        .arcs
        .iter()
        .zip(&residual.lower_bounds)
        .zip(residual_flows)
    {
        if flow < 0 || flow > arc.capacity {
            return Err(McfTransformError::new(format!(
                "residual flow {flow} is outside [0, {}] on arc {}",
                arc.capacity, arc.ordinal
            )));
        }
        let original = lower.checked_add(flow).ok_or_else(|| {
            McfTransformError::new(format!(
                "original flow reconstruction overflow on arc {}",
                arc.ordinal
            ))
        })?;
        original_flows.push(original);
    }
    Ok(original_flows)
}

/// Reconstruct the original objective from the baseline cost and residual
/// flow witness. The returned value is an exact i128 integer.
pub fn reconstruct_objective(
    residual: &McfResidualInstance,
    residual_flows: &[i64],
) -> Result<i128, McfTransformError> {
    residual.validate()?;
    if residual_flows.len() != residual.arcs.len() {
        return Err(McfTransformError::new(
            "residual flow vector must cover every original arc",
        ));
    }
    let mut objective = residual.lower_bound_cost_offset;
    for (arc, &flow) in residual.arcs.iter().zip(residual_flows) {
        if flow < 0 || flow > arc.capacity {
            return Err(McfTransformError::new(format!(
                "residual flow {flow} is outside [0, {}] on arc {}",
                arc.capacity, arc.ordinal
            )));
        }
        let contribution = i128::from(flow)
            .checked_mul(i128::from(arc.unit_cost))
            .ok_or_else(|| {
                McfTransformError::new(format!(
                    "residual cost multiplication overflow on arc {}",
                    arc.ordinal
                ))
            })?;
        objective = objective.checked_add(contribution).ok_or_else(|| {
            McfTransformError::new("reconstructed objective overflowed i128")
        })?;
    }
    Ok(objective)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_dimacs_mcf;

    #[test]
    fn lower_bounds_shift_supply_and_preserve_residual_capacity_and_cost() {
        let instance = parse_dimacs_mcf(
            "p min 3 2\nn 1 5\nn 2 -1\nn 3 -4\na 1 2 2 5 7\na 2 3 1 4 -3",
        )
        .unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert_eq!(residual.adjusted_supplies, vec![0, 3, 0, -3]);
        assert_eq!(residual.arcs[0].capacity, 3);
        assert_eq!(residual.arcs[1].capacity, 3);
        assert_eq!(residual.lower_bounds, vec![2, 1]);
        assert_eq!(residual.lower_bound_cost_offset, 11);
        assert!(residual.is_supply_balanced());
        assert_eq!(restore_original_flows(&residual, &[3, 3]).unwrap(), vec![5, 4]);
        assert_eq!(reconstruct_objective(&residual, &[3, 3]).unwrap(), 23);
    }

    #[test]
    fn lower_bound_reduction_preserves_node_balance_for_restored_flow() {
        let instance = parse_dimacs_mcf(
            "p min 3 2\nn 1 5\nn 2 -1\nn 3 -4\na 1 2 2 5 7\na 2 3 1 4 -3",
        )
        .unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        let residual_flows = [3_i64, 3_i64];
        let original_flows =
            restore_original_flows(&residual, &residual_flows).unwrap();

        let mut original_balance = vec![0_i128; 4];
        let mut residual_balance = vec![0_i128; 4];
        for (arc, &flow) in instance.arcs.iter().zip(&original_flows) {
            original_balance[arc.tail as usize] += i128::from(flow);
            original_balance[arc.head as usize] -= i128::from(flow);
        }
        for (arc, &flow) in residual.arcs.iter().zip(&residual_flows) {
            residual_balance[arc.tail as usize] += i128::from(flow);
            residual_balance[arc.head as usize] -= i128::from(flow);
        }
        assert_eq!(
            original_balance,
            instance.node_supplies.iter().map(|&x| i128::from(x)).collect::<Vec<_>>()
        );
        assert_eq!(residual_balance, residual.adjusted_supplies);
    }

    #[test]
    fn parallel_arc_identity_and_costs_survive_lower_bound_elimination() {
        let instance = parse_dimacs_mcf(
            "p min 2 3\nn 1 6\nn 2 -6\na 1 2 1 3 5\na 1 2 2 4 -2\na 1 2 0 1 9",
        )
        .unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert_eq!(
            residual.arcs.iter().map(|arc| arc.ordinal).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            residual.arcs.iter().map(|arc| arc.capacity).collect::<Vec<_>>(),
            vec![2, 2, 1]
        );
        assert_eq!(
            residual.arcs.iter().map(|arc| arc.unit_cost).collect::<Vec<_>>(),
            vec![5, -2, 9]
        );
        assert_eq!(residual.lower_bound_cost_offset, 1);
    }

    #[test]
    fn zero_lower_bounds_leave_supplies_and_objective_offset_unchanged() {
        let instance =
            parse_dimacs_mcf("p min 2 1\nn 1 3\nn 2 -3\na 1 2 0 3 12").unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert_eq!(residual.adjusted_supplies, vec![0, 3, -3]);
        assert_eq!(residual.arcs[0].capacity, 3);
        assert_eq!(residual.lower_bound_cost_offset, 0);
    }

    #[test]
    fn self_loop_lower_bound_does_not_change_node_balance() {
        let instance = parse_dimacs_mcf("p min 1 1\nn 1 1\na 1 1 2 5 -4").unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert_eq!(residual.adjusted_supplies, vec![0, 1]);
        assert_eq!(residual.arcs[0].capacity, 3);
        assert_eq!(residual.lower_bound_cost_offset, -8);
    }

    #[test]
    fn unbalanced_instance_remains_unbalanced_after_lower_bound_transform() {
        let instance = parse_dimacs_mcf("p min 2 1\nn 1 7\na 1 2 2 5 0").unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert_eq!(instance.supply_balance(), 7);
        assert_eq!(residual.supply_balance(), 7);
        assert!(!residual.is_supply_balanced());
    }

    #[test]
    fn invalid_public_instance_shapes_and_arc_ids_are_rejected() {
        let mut instance = parse_dimacs_mcf("p min 2 1\na 1 2 0 1 0").unwrap();
        instance.node_supplies.pop();
        assert!(
            remove_lower_bounds(&instance)
                .unwrap_err()
                .to_string()
                .contains("vector shape")
        );

        let mut instance = parse_dimacs_mcf("p min 2 1\na 1 2 0 1 0").unwrap();
        instance.arcs[0].ordinal = 9;
        assert!(
            remove_lower_bounds(&instance)
                .unwrap_err()
                .to_string()
                .contains("ordinal 9")
        );

        let mut instance = parse_dimacs_mcf("p min 2 1\na 1 2 0 1 0").unwrap();
        instance.arcs[0].upper_bound = -1;
        assert!(
            remove_lower_bounds(&instance)
                .unwrap_err()
                .to_string()
                .contains("invalid bounds")
        );
    }

    #[test]
    fn restore_rejects_out_of_range_or_incomplete_residual_witness() {
        let instance = parse_dimacs_mcf("p min 2 1\na 1 2 2 5 1").unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert!(restore_original_flows(&residual, &[-1]).is_err());
        assert!(restore_original_flows(&residual, &[4]).is_err());
        assert!(restore_original_flows(&residual, &[]).is_err());
    }

    #[test]
    fn objective_reconstruction_rejects_incomplete_or_out_of_range_witness() {
        let instance = parse_dimacs_mcf("p min 2 1\na 1 2 2 5 -3").unwrap();
        let residual = remove_lower_bounds(&instance).unwrap();
        assert!(reconstruct_objective(&residual, &[]).is_err());
        assert!(reconstruct_objective(&residual, &[4]).is_err());
        assert_eq!(reconstruct_objective(&residual, &[3]).unwrap(), -15);
    }
}
