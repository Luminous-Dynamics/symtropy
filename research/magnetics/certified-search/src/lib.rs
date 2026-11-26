// SPDX-License-Identifier: MIT
#![allow(clippy::float_cmp)] // Exact boundary comparisons are part of the certificate rules.

//! Conservative interval branch-and-bound for robust magnetic design search.
//!
//! This v0.1 implementation searches design coordinates while retaining the
//! complete nuisance/operating interval box. Its bounds are sound but may be
//! loose; nuisance-domain partitioning is a planned refinement.

use std::collections::VecDeque;

use magnetic_interval_ad::{
    certify_expression, Interval, IntervalAdError, IntervalDual, ParameterDomain,
    ValidatedDerivativeEnclosure,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchPolicy {
    pub maximum_nodes: usize,
    pub maximum_depth: u32,
    pub absolute_optimality_gap: f64,
    pub relative_optimality_gap: f64,
    /// Objective and margin scales make derivative contributions dimensionless.
    pub objective_sensitivity_scale: f64,
    pub margin_sensitivity_scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchTermination {
    OptimalityGapSatisfied,
    NodeBudgetExhausted,
    UnresolvedBoxesRemain,
    NoRobustFeasibleDesign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxDisposition {
    RobustInfeasible,
    ObjectiveDominated,
    Split,
    UnresolvedMaximumDepth,
    UnresolvedMinimumWidth,
    UnresolvedNoSensitivity,
    UnresolvedUnboundedExpression,
    UnresolvedNodeBudget,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesignBoxRecord {
    pub node_id: String,
    pub parent_node_id: Option<String>,
    pub depth: u32,
    pub design_bounds: Vec<Interval>,
    pub disposition: BoxDisposition,
    /// A valid lower bound on the worst-case objective over this design box.
    pub objective_lower_bound: Option<f64>,
    /// The interval-enclosed compliance margin over the design x nuisance box.
    pub margin_enclosure: Option<Interval>,
    pub split_parameter_id: Option<String>,
    pub child_node_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeasibleIncumbent {
    pub node_id: String,
    pub design_point: Vec<f64>,
    pub objective_lower_bound: f64,
    /// Upper enclosure of sup_u f(x,u) for this exact design point.
    pub worst_case_objective_upper_bound: f64,
    /// Lower enclosure of inf_u g(x,u) for this exact design point.
    pub robust_margin_lower_bound: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedSearchReport<const N: usize> {
    pub expression_id: String,
    pub objective_id: String,
    pub compliance_requirement_id: String,
    pub design_dimension: usize,
    pub total_parameter_ids: [String; N],
    pub design_parameter_ids: Vec<String>,
    pub nuisance_parameter_ids: Vec<String>,
    pub explored_node_count: usize,
    pub robust_infeasible_count: usize,
    pub objective_dominated_count: usize,
    pub unresolved_leaf_count: usize,
    pub incumbent: Option<FeasibleIncumbent>,
    pub global_objective_lower_bound: Option<f64>,
    pub global_objective_upper_bound: Option<f64>,
    pub absolute_optimality_gap: Option<f64>,
    pub relative_optimality_gap: Option<f64>,
    pub certified_within_gap: bool,
    pub termination: SearchTermination,
    pub nodes: Vec<DesignBoxRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchError {
    InvalidDesignDimension,
    InvalidPolicy,
    InvalidDesignWidth { index: usize },
    InvalidRootIdentity,
    IntervalAd(IntervalAdError),
}

impl From<IntervalAdError> for SearchError {
    fn from(error: IntervalAdError) -> Self {
        Self::IntervalAd(error)
    }
}

#[derive(Clone, Debug)]
struct WorkNode {
    node_id: String,
    parent_node_id: Option<String>,
    depth: u32,
    design_bounds: Vec<Interval>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn stable_midpoint(lower: f64, upper: f64) -> Option<f64> {
    let midpoint = if lower.is_sign_negative() == upper.is_sign_negative() {
        lower + (upper - lower) * 0.5
    } else {
        lower * 0.5 + upper * 0.5
    };
    (midpoint.is_finite() && midpoint > lower && midpoint < upper).then_some(midpoint)
}

fn validate_policy(policy: SearchPolicy) -> Result<(), SearchError> {
    if policy.maximum_nodes == 0
        || !policy.absolute_optimality_gap.is_finite()
        || policy.absolute_optimality_gap < 0.0
        || !policy.relative_optimality_gap.is_finite()
        || policy.relative_optimality_gap < 0.0
        || (policy.absolute_optimality_gap == 0.0
            && policy.relative_optimality_gap == 0.0)
        || !policy.objective_sensitivity_scale.is_finite()
        || policy.objective_sensitivity_scale <= 0.0
        || !policy.margin_sensitivity_scale.is_finite()
        || policy.margin_sensitivity_scale <= 0.0
    {
        return Err(SearchError::InvalidPolicy);
    }
    Ok(())
}

fn node_domain<const N: usize>(
    root: &ParameterDomain<N>,
    node: &WorkNode,
    design_dimension: usize,
    suffix: &str,
) -> ParameterDomain<N> {
    let mut domain = root.clone();
    domain.cell_id = node.node_id.clone();
    domain.record_id = format!("{}:{}:{suffix}", root.record_id, node.node_id);
    domain.parameter_bounds = std::array::from_fn(|index| {
        if index < design_dimension {
            node.design_bounds[index]
        } else {
            root.parameter_bounds[index]
        }
    });
    domain
}

fn point_node_domain<const N: usize>(
    root: &ParameterDomain<N>,
    node: &WorkNode,
    design_dimension: usize,
    point: &[f64],
    suffix: &str,
) -> Result<ParameterDomain<N>, SearchError> {
    let mut domain = node_domain(root, node, design_dimension, suffix);
    for (index, value) in point.iter().enumerate() {
        domain.parameter_bounds[index] =
            Interval::point(*value).map_err(|_| SearchError::InvalidRootIdentity)?;
    }
    Ok(domain)
}

fn design_midpoint(bounds: &[Interval]) -> Option<Vec<f64>> {
    bounds
        .iter()
        .map(|interval| stable_midpoint(interval.lower(), interval.upper()))
        .collect()
}

fn candidate_incumbent<const N: usize, O, G>(
    root: &ParameterDomain<N>,
    node: &WorkNode,
    design_dimension: usize,
    objective: &O,
    compliance_margin: &G,
) -> Result<Option<FeasibleIncumbent>, SearchError>
where
    O: Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
    G: Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
{
    let Some(point) = design_midpoint(&node.design_bounds) else {
        return Ok(None);
    };
    let objective_domain =
        point_node_domain(root, node, design_dimension, &point, "incumbent-objective")?;
    let margin_domain =
        point_node_domain(root, node, design_dimension, &point, "incumbent-margin")?;
    let margin = certify_expression(&margin_domain, compliance_margin)?;
    if margin.value_enclosure.lower() < 0.0 {
        return Ok(None);
    }
    let objective_record = certify_expression(&objective_domain, objective)?;

    Ok(Some(FeasibleIncumbent {
        node_id: node.node_id.clone(),
        design_point: point,
        objective_lower_bound: objective_record.value_enclosure.lower(),
        worst_case_objective_upper_bound: objective_record.value_enclosure.upper(),
        robust_margin_lower_bound: margin.value_enclosure.lower(),
    }))
}

fn normalized_gap(upper: f64, lower: f64) -> (f64, f64) {
    let absolute = (upper - lower).max(0.0);
    let scale = upper.abs().max(lower.abs());
    let relative = if scale == 0.0 { 0.0 } else { absolute / scale };
    (absolute, relative)
}

fn gap_satisfied(policy: SearchPolicy, upper: f64, lower: f64) -> bool {
    let (absolute, _) = normalized_gap(upper, lower);
    absolute
        <= policy.absolute_optimality_gap
            + policy.relative_optimality_gap * upper.abs().max(lower.abs())
}

fn choose_split_axis<const N: usize>(
    node: &WorkNode,
    root: &ParameterDomain<N>,
    design_dimension: usize,
    objective: &ValidatedDerivativeEnclosure<N>,
    margin: &ValidatedDerivativeEnclosure<N>,
    policy: SearchPolicy,
) -> Option<usize> {
    let mut best_axis = None;
    let mut best_score = 0.0;

    for index in 0..design_dimension {
        let width = node.design_bounds[index].upper() - node.design_bounds[index].lower();
        if !width.is_finite() || width <= 0.0 {
            continue;
        }
        let objective_score = objective.derivative_abs_supremum[index]
            * width
            / policy.objective_sensitivity_scale;
        let margin_score = margin.derivative_abs_supremum[index]
            * width
            / policy.margin_sensitivity_scale;
        let score = objective_score.max(margin_score);
        if score > best_score {
            best_score = score;
            best_axis = Some(index);
        }
    }

    // The root is retained only to make the axis-order dependency explicit:
    // parameter IDs, not memory/layout artefacts, are the tie-break contract.
    let _ = root;
    best_axis
}

fn objective_lower_bound<const N: usize>(
    root: &ParameterDomain<N>,
    node: &WorkNode,
    design_dimension: usize,
    objective: &impl Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
) -> Result<ValidatedDerivativeEnclosure<N>, SearchError> {
    let domain = node_domain(root, node, design_dimension, "objective");
    Ok(certify_expression(&domain, objective)?)
}

fn margin_enclosure<const N: usize>(
    root: &ParameterDomain<N>,
    node: &WorkNode,
    design_dimension: usize,
    margin: &impl Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
) -> Result<ValidatedDerivativeEnclosure<N>, SearchError> {
    let domain = node_domain(root, node, design_dimension, "compliance-margin");
    Ok(certify_expression(&domain, margin)?)
}

fn validate_root<const N: usize>(
    root: &ParameterDomain<N>,
    design_dimension: usize,
) -> Result<(), SearchError> {
    if design_dimension == 0 || design_dimension > N {
        return Err(SearchError::InvalidDesignDimension);
    }
    if !nonempty(&root.record_id)
        || !nonempty(&root.expression_id)
        || !nonempty(&root.input_sha256)
    {
        return Err(SearchError::InvalidRootIdentity);
    }
    Ok(())
}

/// Search design coordinates while keeping the suffix nuisance/operating
/// intervals intact. Objective and margin expressions are evaluated over the
/// full design × nuisance box at every node.
pub fn branch_and_bound<const N: usize, O, G>(
    root: &ParameterDomain<N>,
    design_dimension: usize,
    policy: SearchPolicy,
    objective: O,
    compliance_margin: G,
) -> Result<CertifiedSearchReport<N>, SearchError>
where
    O: Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
    G: Fn([IntervalDual<N>; N]) -> Result<IntervalDual<N>, IntervalAdError>,
{
    validate_root(root, design_dimension)?;
    validate_policy(policy)?;

    let root_design_bounds = root.parameter_bounds[..design_dimension].to_vec();
    for (index, bounds) in root_design_bounds.iter().enumerate() {
        if !bounds.is_finite() || bounds.lower() >= bounds.upper() {
            return Err(SearchError::InvalidDesignWidth { index });
        }
    }

    let mut queue = VecDeque::from([WorkNode {
        node_id: "root".to_owned(),
        parent_node_id: None,
        depth: 0,
        design_bounds: root_design_bounds,
    }]);
    let mut records = Vec::new();
    let mut incumbent: Option<FeasibleIncumbent> = None;
    let mut explored_node_count = 0usize;
    let mut robust_infeasible_count = 0usize;
    let mut objective_dominated_count = 0usize;
    let mut budget_exhausted = false;

    while let Some(node) = queue.pop_front() {
        if explored_node_count >= policy.maximum_nodes {
            budget_exhausted = true;
            let mut remainder = vec![node];
            remainder.extend(queue.into_iter());
            for pending in remainder {
                records.push(DesignBoxRecord {
                    node_id: pending.node_id,
                    parent_node_id: pending.parent_node_id,
                    depth: pending.depth,
                    design_bounds: pending.design_bounds,
                    disposition: BoxDisposition::UnresolvedNodeBudget,
                    objective_lower_bound: None,
                    margin_enclosure: None,
                    split_parameter_id: None,
                    child_node_ids: Vec::new(),
                });
            }
            break;
        }

        explored_node_count += 1;
        let objective_result =
            objective_lower_bound(root, &node, design_dimension, &objective);
        let margin_result = margin_enclosure(root, &node, design_dimension, &compliance_margin);

        let (objective, margin) = match (objective_result, margin_result) {
            (Ok(objective), Ok(margin)) => (objective, margin),
            (Err(SearchError::IntervalAd(IntervalAdError::UnboundedValueEnclosure
                | IntervalAdError::UnboundedDerivativeEnclosure { .. }
                | IntervalAdError::SquareRootOutsideRealDomain)),
             _)
            | (_, Err(SearchError::IntervalAd(IntervalAdError::UnboundedValueEnclosure
                | IntervalAdError::UnboundedDerivativeEnclosure { .. }
                | IntervalAdError::SquareRootOutsideRealDomain))) => {
                let mut split = None;
                let mut best_width = 0.0;
                for (index, bounds) in node.design_bounds.iter().enumerate() {
                    let width = bounds.upper() - bounds.lower();
                    if width > best_width
                        && width > 0.0
                        && stable_midpoint(bounds.lower(), bounds.upper()).is_some()
                    {
                        best_width = width;
                        split = Some(index);
                    }
                }

                if let Some(axis) = split {
                    let bounds = node.design_bounds[axis];
                    let midpoint = stable_midpoint(bounds.lower(), bounds.upper())
                        .expect("selected axis has midpoint");
                    let mut left_bounds = node.design_bounds.clone();
                    let mut right_bounds = node.design_bounds.clone();
                    left_bounds[axis] = Interval::new(bounds.lower(), midpoint)
                        .map_err(|_| SearchError::InvalidDesignWidth { index: axis })?;
                    right_bounds[axis] = Interval::new(midpoint, bounds.upper())
                        .map_err(|_| SearchError::InvalidDesignWidth { index: axis })?;
                    let left_id = format!("{}/{}-0", node.node_id, root.parameter_ids[axis]);
                    let right_id = format!("{}/{}-1", node.node_id, root.parameter_ids[axis]);
                    queue.push_back(WorkNode {
                        node_id: left_id.clone(),
                        parent_node_id: Some(node.node_id.clone()),
                        depth: node.depth + 1,
                        design_bounds: left_bounds,
                    });
                    queue.push_back(WorkNode {
                        node_id: right_id.clone(),
                        parent_node_id: Some(node.node_id.clone()),
                        depth: node.depth + 1,
                        design_bounds: right_bounds,
                    });
                    records.push(DesignBoxRecord {
                        node_id: node.node_id,
                        parent_node_id: node.parent_node_id,
                        depth: node.depth,
                        design_bounds: node.design_bounds,
                        disposition: BoxDisposition::Split,
                        objective_lower_bound: None,
                        margin_enclosure: None,
                        split_parameter_id: Some(root.parameter_ids[axis].clone()),
                        child_node_ids: vec![left_id, right_id],
                    });
                } else {
                    records.push(DesignBoxRecord {
                        node_id: node.node_id,
                        parent_node_id: node.parent_node_id,
                        depth: node.depth,
                        design_bounds: node.design_bounds,
                        disposition: BoxDisposition::UnresolvedUnboundedExpression,
                        objective_lower_bound: None,
                        margin_enclosure: None,
                        split_parameter_id: None,
                        child_node_ids: Vec::new(),
                    });
                }
                continue;
            }
            (Err(error), _) | (_, Err(error)) => return Err(error),
        };

        if margin.value_enclosure.upper() < 0.0 {
            robust_infeasible_count += 1;
            records.push(DesignBoxRecord {
                node_id: node.node_id,
                parent_node_id: node.parent_node_id,
                depth: node.depth,
                design_bounds: node.design_bounds,
                disposition: BoxDisposition::RobustInfeasible,
                objective_lower_bound: Some(objective.value_enclosure.lower()),
                margin_enclosure: Some(margin.value_enclosure),
                split_parameter_id: None,
                child_node_ids: Vec::new(),
            });
            continue;
        }

        if let Some(candidate) = candidate_incumbent(
            root,
            &node,
            design_dimension,
            &objective,
            &compliance_margin,
        )? {
            if incumbent.as_ref().is_none_or(|existing| {
                candidate.worst_case_objective_upper_bound
                    < existing.worst_case_objective_upper_bound
            }) {
                incumbent = Some(candidate);
            }
        }

        let objective_lower = objective.value_enclosure.lower();
        if incumbent.as_ref().is_some_and(|best| {
            objective_lower >= best.worst_case_objective_upper_bound
        }) {
            objective_dominated_count += 1;
            records.push(DesignBoxRecord {
                node_id: node.node_id,
                parent_node_id: node.parent_node_id,
                depth: node.depth,
                design_bounds: node.design_bounds,
                disposition: BoxDisposition::ObjectiveDominated,
                objective_lower_bound: Some(objective_lower),
                margin_enclosure: Some(margin.value_enclosure),
                split_parameter_id: None,
                child_node_ids: Vec::new(),
            });
            continue;
        }

        let mut selected_axis = None;
        let mut selected_score = 0.0;
        for axis in 0..design_dimension {
            let bounds = node.design_bounds[axis];
            let width = bounds.upper() - bounds.lower();
            let objective_score = objective.derivative_abs_supremum[axis]
                * width
                / policy.objective_sensitivity_scale;
            let margin_score = margin.derivative_abs_supremum[axis]
                * width
                / policy.margin_sensitivity_scale;
            let score = objective_score.max(margin_score);
            if width.is_finite() && stable_midpoint(bounds.lower(), bounds.upper()).is_some()
                && score > selected_score
            {
                selected_axis = Some(axis);
                selected_score = score;
            }
        }

        if node.depth >= policy.maximum_depth {
            records.push(DesignBoxRecord {
                node_id: node.node_id,
                parent_node_id: node.parent_node_id,
                depth: node.depth,
                design_bounds: node.design_bounds,
                disposition: BoxDisposition::UnresolvedMaximumDepth,
                objective_lower_bound: Some(objective_lower),
                margin_enclosure: Some(margin.value_enclosure),
                split_parameter_id: None,
                child_node_ids: Vec::new(),
            });
            continue;
        }

        let Some(axis) = selected_axis else {
            records.push(DesignBoxRecord {
                node_id: node.node_id,
                parent_node_id: node.parent_node_id,
                depth: node.depth,
                design_bounds: node.design_bounds,
                disposition: BoxDisposition::UnresolvedNoSensitivity,
                objective_lower_bound: Some(objective_lower),
                margin_enclosure: Some(margin.value_enclosure),
                split_parameter_id: None,
                child_node_ids: Vec::new(),
            });
            continue;
        };

        let bounds = node.design_bounds[axis];
        let midpoint = stable_midpoint(bounds.lower(), bounds.upper())
            .expect("selected axis midpoint");
        let mut left_bounds = node.design_bounds.clone();
        let mut right_bounds = node.design_bounds.clone();
        left_bounds[axis] = Interval::new(bounds.lower(), midpoint)
            .map_err(|_| SearchError::InvalidDesignWidth { index: axis })?;
        right_bounds[axis] = Interval::new(midpoint, bounds.upper())
            .map_err(|_| SearchError::InvalidDesignWidth { index: axis })?;
        let left_id = format!("{}/{}-0", node.node_id, root.parameter_ids[axis]);
        let right_id = format!("{}/{}-1", node.node_id, root.parameter_ids[axis]);

        queue.push_back(WorkNode {
            node_id: left_id.clone(),
            parent_node_id: Some(node.node_id.clone()),
            depth: node.depth + 1,
            design_bounds: left_bounds,
        });
        queue.push_back(WorkNode {
            node_id: right_id.clone(),
            parent_node_id: Some(node.node_id.clone()),
            depth: node.depth + 1,
            design_bounds: right_bounds,
        });
        records.push(DesignBoxRecord {
            node_id: node.node_id,
            parent_node_id: node.parent_node_id,
            depth: node.depth,
            design_bounds: node.design_bounds,
            disposition: BoxDisposition::Split,
            objective_lower_bound: Some(objective_lower),
            margin_enclosure: Some(margin.value_enclosure),
            split_parameter_id: Some(root.parameter_ids[axis].clone()),
            child_node_ids: vec![left_id, right_id],
        });
    }

    let mut unresolved_lower_bounds = Vec::new();
    let mut unresolved_leaf_count = 0usize;
    let mut has_unbounded_leaf = false;
    for record in &records {
        match record.disposition {
            BoxDisposition::UnresolvedMaximumDepth
            | BoxDisposition::UnresolvedMinimumWidth
            | BoxDisposition::UnresolvedNoSensitivity
            | BoxDisposition::UnresolvedUnboundedExpression
            | BoxDisposition::UnresolvedNodeBudget => {
                unresolved_leaf_count += 1;
                if let Some(lower) = record.objective_lower_bound {
                    unresolved_lower_bounds.push(lower);
                } else {
                    has_unbounded_leaf = true;
                }
            }
            _ => {}
        }
    }

    let incumbent_lower = incumbent
        .as_ref()
        .map(|candidate| candidate.objective_lower_bound);
    let global_objective_lower_bound = if has_unbounded_leaf {
        None
    } else {
        incumbent_lower
            .into_iter()
            .chain(unresolved_lower_bounds)
            .reduce(f64::min)
    };
    let global_objective_upper_bound =
        incumbent.as_ref().map(|candidate| candidate.worst_case_objective_upper_bound);
    let gaps = match (global_objective_lower_bound, global_objective_upper_bound) {
        (Some(lower), Some(upper)) => {
            let (absolute, relative) = normalized_gap(upper, lower);
            Some((absolute, relative))
        }
        _ => None,
    };
    let certified_within_gap = gaps.is_some_and(|(absolute, _)| {
        global_objective_lower_bound
            .zip(global_objective_upper_bound)
            .is_some_and(|(lower, upper)| {
                absolute
                    <= policy.absolute_optimality_gap
                        + policy.relative_optimality_gap * upper.abs().max(lower.abs())
            })
    });

    let termination = if certified_within_gap {
        SearchTermination::OptimalityGapSatisfied
    } else if budget_exhausted {
        SearchTermination::NodeBudgetExhausted
    } else if incumbent.is_none() && unresolved_leaf_count == 0 {
        SearchTermination::NoRobustFeasibleDesign
    } else {
        SearchTermination::UnresolvedBoxesRemain
    };

    Ok(CertifiedSearchReport {
        expression_id: root.expression_id.clone(),
        objective_id: root.requirement_id.clone(),
        compliance_requirement_id: root.requirement_id.clone(),
        design_dimension,
        total_parameter_ids: root.parameter_ids.clone(),
        design_parameter_ids: root.parameter_ids[..design_dimension].to_vec(),
        nuisance_parameter_ids: root.parameter_ids[design_dimension..].to_vec(),
        explored_node_count,
        robust_infeasible_count,
        objective_dominated_count,
        unresolved_leaf_count,
        incumbent,
        global_objective_lower_bound,
        global_objective_upper_bound,
        absolute_optimality_gap: gaps.map(|gap| gap.0),
        relative_optimality_gap: gaps.map(|gap| gap.1),
        certified_within_gap,
        termination,
        nodes: records,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("valid test interval")
    }

    fn root_domain() -> ParameterDomain<2> {
        ParameterDomain {
            record_id: "global-search-001".to_owned(),
            expression_id: "quadratic-robust-reference".to_owned(),
            cell_id: "root".to_owned(),
            input_sha256: DIGEST_A.to_owned(),
            source_revision: "test-model-v1".to_owned(),
            implementation_id: "magnetic-certified-search".to_owned(),
            artifact_sha256: DIGEST_B.to_owned(),
            environment_sha256: DIGEST_C.to_owned(),
            parameter_ids: ["x".to_owned(), "u".to_owned()],
            parameter_units: ["dimensionless".to_owned(), "dimensionless".to_owned()],
            parameter_bounds: [interval(-1.0, 1.0), interval(-0.1, 0.1)],
            output_unit: "dimensionless".to_owned(),
            coordinate_frame_id: "design-frame-v1".to_owned(),
            operating_condition_id: "full-tolerance-box-v1".to_owned(),
            model_set_id: "analytic-test-model".to_owned(),
            solver_run_id: "analytic-expression-run".to_owned(),
            geometry_sha256: DIGEST_A.to_owned(),
            material_sha256: DIGEST_B.to_owned(),
            requirement_id: "robust-margin-positive".to_owned(),
        }
    }

    fn policy(gap: f64) -> SearchPolicy {
        SearchPolicy {
            maximum_nodes: 256,
            maximum_depth: 20,
            absolute_optimality_gap: gap,
            relative_optimality_gap: 0.0,
            objective_sensitivity_scale: 1.0,
            margin_sensitivity_scale: 1.0,
        }
    }

    fn objective<const N: usize>(
        [x, u]: [IntervalDual<N>; N],
    ) -> Result<IntervalDual<N>, IntervalAdError> {
        let tenth = IntervalDual::constant(interval(0.1, 0.1));
        Ok(x.square() + tenth * u.square())
    }

    fn margin<const N: usize>(
        [x, u]: [IntervalDual<N>; N],
    ) -> Result<IntervalDual<N>, IntervalAdError> {
        let threshold = IntervalDual::constant(interval(0.51, 0.51));
        Ok(x - threshold - u.square())
    }

    #[test]
    fn root_candidate_gets_a_valid_robust_gap_when_already_feasible() {
        let mut domain = root_domain();
        domain.parameter_bounds[0] = interval(-0.1, 0.1);
        let result = branch_and_bound(&domain, 1, policy(0.002), objective::<2>, |[x, u]| {
            let one = IntervalDual::constant(Interval::ONE);
            Ok(one - x.square() - u.square())
        })
        .expect("valid search");
        assert!(result.incumbent.is_some());
        assert!(result.certified_within_gap);
        assert_eq!(result.termination, SearchTermination::OptimalityGapSatisfied);
        assert!(result.global_objective_lower_bound.is_some());
        assert!(result.global_objective_upper_bound.is_some());
    }

    #[test]
    fn search_discovers_a_robust_feasible_candidate_after_subdivision() {
        let result =
            branch_and_bound(&root_domain(), 1, policy(0.02), objective::<2>, margin::<2>)
                .expect("valid search");
        let incumbent = result.incumbent.expect("feasible candidate discovered");
        assert!(incumbent.design_point[0] >= 0.52);
        assert!(incumbent.robust_margin_lower_bound >= 0.0);
        assert!(result.robust_infeasible_count > 0);
        assert!(result.nodes.iter().any(|node| node.disposition == BoxDisposition::Split));
    }

    #[test]
    fn a_wholly_failing_design_box_is_pruned_as_robust_infeasible() {
        let mut domain = root_domain();
        domain.parameter_bounds[0] = interval(-1.0, 0.0);
        let result =
            branch_and_bound(&domain, 1, policy(0.01), objective::<2>, margin::<2>)
                .expect("valid infeasible search");
        assert_eq!(result.robust_infeasible_count, 1);
        assert!(result.incumbent.is_none());
        assert_eq!(result.termination, SearchTermination::NoRobustFeasibleDesign);
    }

    #[test]
    fn node_budget_exhaustion_never_claims_global_optimality() {
        let mut bounded_policy = policy(1.0e-12);
        bounded_policy.maximum_nodes = 1;
        let result =
            branch_and_bound(&root_domain(), 1, bounded_policy, objective::<2>, margin::<2>)
                .expect("valid bounded search");
        assert!(!result.certified_within_gap);
        assert_eq!(result.termination, SearchTermination::NodeBudgetExhausted);
        assert!(result.nodes.iter().any(|node| {
            node.disposition == BoxDisposition::UnresolvedNodeBudget
        }));
    }

    #[test]
    fn no_sensitivity_or_budget_is_not_mislabeled_optimal() {
        let mut bounded_policy = policy(1.0e-12);
        bounded_policy.maximum_depth = 0;
        let result =
            branch_and_bound(&root_domain(), 1, bounded_policy, objective::<2>, margin::<2>)
                .expect("valid depth-limited search");
        assert!(!result.certified_within_gap);
        assert_eq!(result.termination, SearchTermination::UnresolvedBoxesRemain);
        assert!(result.unresolved_leaf_count > 0);
    }

    #[test]
    fn deterministic_input_replay_produces_identical_report() {
        let first =
            branch_and_bound(&root_domain(), 1, policy(0.02), objective::<2>, margin::<2>)
                .expect("first search");
        let second =
            branch_and_bound(&root_domain(), 1, policy(0.02), objective::<2>, margin::<2>)
                .expect("replay");
        assert_eq!(first, second);
    }
}
