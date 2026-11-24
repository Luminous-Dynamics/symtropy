#![forbid(unsafe_code)]

//! SAFRB v0.4: weighted disruption-scenario tournament scorecard.
//!
//! This module scores already-produced planning outcomes. It does not create
//! scenarios, prove their probability weights, or validate physical inputs.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneOutcome {
    pub zone_id: String,
    pub critical_demand_l: u64,
    pub critical_delivered_l: u64,
    pub critical_unserved_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioOutcome {
    pub scenario_id: String,
    /// Relative scenario weight. Weights are normalized within this tournament.
    /// Equal weights represent an unweighted empirical test suite.
    pub weight: u64,
    pub total_unserved_l: u64,
    pub total_critical_demand_l: u64,
    pub total_critical_delivered_l: u64,
    pub total_critical_unserved_l: u64,
    pub logistics_cost_units: u128,
    pub zones: Vec<ZoneOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneRisk {
    pub zone_id: String,
    pub var_unserved_l: u64,
    pub cvar_unserved_l_ceil: u128,
    pub worst_unserved_l: u64,
    pub expected_unserved_l_ceil: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentMetrics {
    pub scenario_count: usize,
    pub total_weight: u128,
    pub alpha_bps: u16,
    pub expected_total_unserved_l_ceil: u128,
    pub expected_critical_unserved_l_ceil: u128,
    pub critical_service_rate_bps: u16,
    pub var_critical_unserved_l: u64,
    pub cvar_critical_unserved_l_ceil: u128,
    pub worst_critical_unserved_l: u64,
    pub weighted_logistics_cost_units_ceil: u128,
    pub zone_risk: Vec<ZoneRisk>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchedComparison {
    pub baseline: TournamentMetrics,
    pub candidate: TournamentMetrics,
    /// Positive means candidate lowers CVaR; negative means it worsens it.
    /// None means the baseline CVaR is zero and percentage change is undefined.
    pub cvar_reduction_bps: Option<i128>,
    /// Positive means critical demand service improved.
    pub critical_service_delta_bps: i32,
    /// Positive means candidate cost increased.
    pub logistics_cost_delta_units: i128,
}

fn checked_add_u128(target: &mut u128, value: u128, what: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{what} overflowed u128"))?;
    Ok(())
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, String> {
    if denominator == 0 {
        return Err("cannot divide by zero".into());
    }
    let adjusted = numerator
        .checked_add(denominator - 1)
        .ok_or_else(|| "ceiling division overflowed u128".to_string())?;
    Ok(adjusted / denominator)
}

fn weighted_mean_ceil(losses: &[(u64, u64)], total_weight: u128) -> Result<u128, String> {
    let mut numerator = 0_u128;
    for (loss_l, weight) in losses {
        let term = u128::from(*loss_l)
            .checked_mul(u128::from(*weight))
            .ok_or_else(|| "weighted loss product overflowed u128".to_string())?;
        checked_add_u128(&mut numerator, term, "weighted loss sum")?;
    }
    ceil_div(numerator, total_weight)
}

fn weighted_var(losses: &[(u64, u64)], total_weight: u128, alpha_bps: u16) -> Result<u64, String> {
    if alpha_bps == 0 {
        return losses
            .iter()
            .map(|(loss, _)| *loss)
            .min()
            .ok_or_else(|| "no losses supplied".into());
    }

    let threshold = total_weight
        .checked_mul(u128::from(alpha_bps))
        .ok_or_else(|| "VaR threshold overflowed u128".to_string())?;

    let mut sorted = losses.to_vec();
    sorted.sort_by_key(|(loss, _)| *loss);

    let mut cumulative_scaled = 0_u128;
    for (loss_l, weight) in sorted {
        let contribution = u128::from(weight)
            .checked_mul(10_000)
            .ok_or_else(|| "scaled scenario weight overflowed u128".to_string())?;
        checked_add_u128(
            &mut cumulative_scaled,
            contribution,
            "cumulative scenario weight",
        )?;
        if cumulative_scaled >= threshold {
            return Ok(loss_l);
        }
    }
    Err("VaR threshold was not reached".into())
}

/// Computes upper-tail expected shortfall with an exact fractional boundary
/// scenario. Result is rounded upward to whole litres (conservative).
fn weighted_cvar_ceil(
    losses: &[(u64, u64)],
    total_weight: u128,
    alpha_bps: u16,
) -> Result<u128, String> {
    let tail_bps = 10_000_u128 - u128::from(alpha_bps);
    let target_tail_scaled = total_weight
        .checked_mul(tail_bps)
        .ok_or_else(|| "CVaR tail mass overflowed u128".to_string())?;
    if target_tail_scaled == 0 {
        return Err("CVaR tail mass is zero".into());
    }

    let mut sorted = losses.to_vec();
    sorted.sort_by_key(|item| std::cmp::Reverse(item.0));

    let mut remaining = target_tail_scaled;
    let mut weighted_tail_loss = 0_u128;

    for (loss_l, weight) in sorted {
        if remaining == 0 {
            break;
        }
        let scenario_mass = u128::from(weight)
            .checked_mul(10_000)
            .ok_or_else(|| "scaled scenario mass overflowed u128".to_string())?;
        let selected_mass = scenario_mass.min(remaining);
        let contribution = u128::from(loss_l)
            .checked_mul(selected_mass)
            .ok_or_else(|| "CVaR contribution overflowed u128".to_string())?;
        checked_add_u128(&mut weighted_tail_loss, contribution, "CVaR tail loss")?;
        remaining -= selected_mass;
    }

    if remaining != 0 {
        return Err("CVaR tail mass exceeds supplied scenario mass".into());
    }
    ceil_div(weighted_tail_loss, target_tail_scaled)
}

fn validate_outcomes(outcomes: &[ScenarioOutcome]) -> Result<(u128, Vec<String>), String> {
    if outcomes.is_empty() {
        return Err("tournament requires at least one scenario".into());
    }

    let mut ids = BTreeSet::new();
    let mut zones_expected: Option<BTreeSet<String>> = None;
    let mut total_weight = 0_u128;

    for outcome in outcomes {
        if outcome.scenario_id.trim().is_empty() {
            return Err("scenario_id must not be empty".into());
        }
        if !ids.insert(outcome.scenario_id.as_str()) {
            return Err(format!("duplicate scenario_id: {}", outcome.scenario_id));
        }
        if outcome.weight == 0 {
            return Err(format!("scenario {} has zero weight", outcome.scenario_id));
        }
        checked_add_u128(
            &mut total_weight,
            u128::from(outcome.weight),
            "total scenario weight",
        )?;

        if outcome
            .total_critical_delivered_l
            .checked_add(outcome.total_critical_unserved_l)
            != Some(outcome.total_critical_demand_l)
        {
            return Err(format!(
                "scenario {} critical demand does not balance",
                outcome.scenario_id
            ));
        }
        if outcome.total_unserved_l < outcome.total_critical_unserved_l {
            return Err(format!(
                "scenario {} total unserved is below critical unserved",
                outcome.scenario_id
            ));
        }

        let mut zone_ids = BTreeSet::new();
        let mut zone_demand = 0_u128;
        let mut zone_delivered = 0_u128;
        let mut zone_unserved = 0_u128;

        for zone in &outcome.zones {
            if zone.zone_id.trim().is_empty() {
                return Err(format!(
                    "scenario {} has an empty zone id",
                    outcome.scenario_id
                ));
            }
            if !zone_ids.insert(zone.zone_id.clone()) {
                return Err(format!(
                    "scenario {} has duplicate zone {}",
                    outcome.scenario_id, zone.zone_id
                ));
            }
            if zone
                .critical_delivered_l
                .checked_add(zone.critical_unserved_l)
                != Some(zone.critical_demand_l)
            {
                return Err(format!(
                    "scenario {} zone {} demand does not balance",
                    outcome.scenario_id, zone.zone_id
                ));
            }
            checked_add_u128(
                &mut zone_demand,
                u128::from(zone.critical_demand_l),
                "zone demand",
            )?;
            checked_add_u128(
                &mut zone_delivered,
                u128::from(zone.critical_delivered_l),
                "zone delivered",
            )?;
            checked_add_u128(
                &mut zone_unserved,
                u128::from(zone.critical_unserved_l),
                "zone unserved",
            )?;
        }

        if zone_demand != u128::from(outcome.total_critical_demand_l)
            || zone_delivered != u128::from(outcome.total_critical_delivered_l)
            || zone_unserved != u128::from(outcome.total_critical_unserved_l)
        {
            return Err(format!(
                "scenario {} zone totals do not reconcile to critical totals",
                outcome.scenario_id
            ));
        }

        match &zones_expected {
            None => zones_expected = Some(zone_ids),
            Some(expected) if *expected != zone_ids => {
                return Err(format!(
                    "scenario {} zone set differs from the tournament",
                    outcome.scenario_id
                ));
            }
            Some(_) => {}
        }
    }

    if total_weight == 0 {
        return Err("total scenario weight is zero".into());
    }

    let mut zones: Vec<String> = zones_expected.unwrap_or_default().into_iter().collect();
    zones.sort();
    Ok((total_weight, zones))
}

impl TournamentMetrics {
    pub fn evaluate(outcomes: &[ScenarioOutcome], alpha_bps: u16) -> Result<Self, String> {
        if alpha_bps >= 10_000 {
            return Err("alpha_bps must be in 0..10000".into());
        }
        let (total_weight, zone_ids) = validate_outcomes(outcomes)?;

        let total_losses: Vec<(u64, u64)> = outcomes
            .iter()
            .map(|o| (o.total_unserved_l, o.weight))
            .collect();
        let critical_losses: Vec<(u64, u64)> = outcomes
            .iter()
            .map(|o| (o.total_critical_unserved_l, o.weight))
            .collect();

        let expected_total_unserved_l_ceil = weighted_mean_ceil(&total_losses, total_weight)?;
        let expected_critical_unserved_l_ceil = weighted_mean_ceil(&critical_losses, total_weight)?;
        let var_critical_unserved_l = weighted_var(&critical_losses, total_weight, alpha_bps)?;
        let cvar_critical_unserved_l_ceil =
            weighted_cvar_ceil(&critical_losses, total_weight, alpha_bps)?;
        let worst_critical_unserved_l = critical_losses
            .iter()
            .map(|(loss, _)| *loss)
            .max()
            .unwrap_or(0);

        let mut demand_weighted = 0_u128;
        let mut served_weighted = 0_u128;
        let mut weighted_cost = 0_u128;

        for outcome in outcomes {
            let weight = u128::from(outcome.weight);
            let demand = weight
                .checked_mul(u128::from(outcome.total_critical_demand_l))
                .ok_or_else(|| "weighted demand overflowed u128".to_string())?;
            let delivered = weight
                .checked_mul(u128::from(outcome.total_critical_delivered_l))
                .ok_or_else(|| "weighted delivered volume overflowed u128".to_string())?;
            let cost = weight
                .checked_mul(outcome.logistics_cost_units)
                .ok_or_else(|| "weighted cost overflowed u128".to_string())?;
            checked_add_u128(&mut demand_weighted, demand, "weighted demand total")?;
            checked_add_u128(&mut served_weighted, delivered, "weighted delivered total")?;
            checked_add_u128(&mut weighted_cost, cost, "weighted cost total")?;
        }

        let numerator = served_weighted
            .checked_mul(10_000)
            .ok_or_else(|| "service-rate numerator overflowed u128".to_string())?;
        let service_rate = numerator.checked_div(demand_weighted).unwrap_or(10_000);
        let critical_service_rate_bps = u16::try_from(service_rate)
            .map_err(|_| "service rate exceeds 10000 bps".to_string())?;
        let weighted_logistics_cost_units_ceil = ceil_div(weighted_cost, total_weight)?;

        let mut zone_risk = Vec::with_capacity(zone_ids.len());
        for zone_id in zone_ids {
            let losses = outcomes
                .iter()
                .map(|outcome| {
                    let zone = outcome
                        .zones
                        .iter()
                        .find(|z| z.zone_id == zone_id)
                        .expect("zone sets were validated");
                    (zone.critical_unserved_l, outcome.weight)
                })
                .collect::<Vec<_>>();
            zone_risk.push(ZoneRisk {
                zone_id,
                var_unserved_l: weighted_var(&losses, total_weight, alpha_bps)?,
                cvar_unserved_l_ceil: weighted_cvar_ceil(&losses, total_weight, alpha_bps)?,
                worst_unserved_l: losses.iter().map(|(loss, _)| *loss).max().unwrap_or(0),
                expected_unserved_l_ceil: weighted_mean_ceil(&losses, total_weight)?,
            });
        }

        Ok(Self {
            scenario_count: outcomes.len(),
            total_weight,
            alpha_bps,
            expected_total_unserved_l_ceil,
            expected_critical_unserved_l_ceil,
            critical_service_rate_bps,
            var_critical_unserved_l,
            cvar_critical_unserved_l_ceil,
            worst_critical_unserved_l,
            weighted_logistics_cost_units_ceil,
            zone_risk,
        })
    }
}

pub fn compare_matched(
    baseline: &[ScenarioOutcome],
    candidate: &[ScenarioOutcome],
    alpha_bps: u16,
) -> Result<MatchedComparison, String> {
    let baseline_map: BTreeMap<&str, &ScenarioOutcome> = baseline
        .iter()
        .map(|o| (o.scenario_id.as_str(), o))
        .collect();
    let candidate_map: BTreeMap<&str, &ScenarioOutcome> = candidate
        .iter()
        .map(|o| (o.scenario_id.as_str(), o))
        .collect();

    if baseline_map.len() != baseline.len() || candidate_map.len() != candidate.len() {
        return Err("duplicate scenario ids are not allowed in matched comparison".into());
    }
    if baseline_map.len() != candidate_map.len()
        || baseline_map
            .keys()
            .any(|id| !candidate_map.contains_key(id))
    {
        return Err("baseline and candidate scenario sets do not match".into());
    }
    for (id, base) in &baseline_map {
        let next = candidate_map[id];
        if base.weight != next.weight {
            return Err(format!("scenario {} weights do not match", id));
        }
        if base.total_critical_demand_l != next.total_critical_demand_l
            || base
                .zones
                .iter()
                .map(|z| z.zone_id.as_str())
                .collect::<BTreeSet<_>>()
                != next
                    .zones
                    .iter()
                    .map(|z| z.zone_id.as_str())
                    .collect::<BTreeSet<_>>()
        {
            return Err(format!("scenario {} demand frame does not match", id));
        }
        for base_zone in &base.zones {
            let next_zone = next
                .zones
                .iter()
                .find(|z| z.zone_id == base_zone.zone_id)
                .ok_or_else(|| format!("scenario {} is missing zone {}", id, base_zone.zone_id))?;
            if base_zone.critical_demand_l != next_zone.critical_demand_l {
                return Err(format!(
                    "scenario {} zone {} demand does not match",
                    id, base_zone.zone_id
                ));
            }
        }
    }

    let baseline_metrics = TournamentMetrics::evaluate(baseline, alpha_bps)?;
    let candidate_metrics = TournamentMetrics::evaluate(candidate, alpha_bps)?;

    let cvar_reduction_bps = if baseline_metrics.cvar_critical_unserved_l_ceil == 0 {
        None
    } else {
        let base = i128::try_from(baseline_metrics.cvar_critical_unserved_l_ceil)
            .map_err(|_| "baseline CVaR exceeds i128".to_string())?;
        let next = i128::try_from(candidate_metrics.cvar_critical_unserved_l_ceil)
            .map_err(|_| "candidate CVaR exceeds i128".to_string())?;
        Some(
            base.checked_sub(next)
                .and_then(|delta| delta.checked_mul(10_000))
                .ok_or_else(|| "CVaR improvement overflowed i128".to_string())?
                / base,
        )
    };

    let critical_service_delta_bps = i32::from(candidate_metrics.critical_service_rate_bps)
        - i32::from(baseline_metrics.critical_service_rate_bps);

    let base_cost = i128::try_from(baseline_metrics.weighted_logistics_cost_units_ceil)
        .map_err(|_| "baseline cost exceeds i128".to_string())?;
    let candidate_cost = i128::try_from(candidate_metrics.weighted_logistics_cost_units_ceil)
        .map_err(|_| "candidate cost exceeds i128".to_string())?;
    let logistics_cost_delta_units = candidate_cost
        .checked_sub(base_cost)
        .ok_or_else(|| "logistics cost delta overflowed i128".to_string())?;

    Ok(MatchedComparison {
        baseline: baseline_metrics,
        candidate: candidate_metrics,
        cvar_reduction_bps,
        critical_service_delta_bps,
        logistics_cost_delta_units,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioProbabilityBound {
    pub scenario_id: String,
    /// Minimum probability mass in basis points (10,000 bps = 100%).
    pub lower_bps: u16,
    /// Maximum probability mass in basis points.
    pub upper_bps: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioProbability {
    pub scenario_id: String,
    pub probability_bps: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionallyRobustRisk {
    pub alpha_bps: u16,
    pub worst_case_expected_critical_unserved_l_ceil: u128,
    pub worst_case_cvar_critical_unserved_l_ceil: u128,
    /// A witness distribution that attains the upper-tail extremum for
    /// increasing loss functions under independent probability intervals and
    /// a sum-to-10000-bps constraint.
    pub worst_case_distribution: Vec<ScenarioProbability>,
}

/// Finds the worst-case probability vector inside a box-constrained simplex.
/// Residual mass is assigned to scenarios in descending loss order. This
/// maximizes the upper tail of the discrete loss distribution (first-order
/// stochastic dominance), and therefore maximizes both expectation and CVaR.
pub fn distributionally_robust_risk(
    outcomes: &[ScenarioOutcome],
    bounds: &[ScenarioProbabilityBound],
    alpha_bps: u16,
) -> Result<DistributionallyRobustRisk, String> {
    if alpha_bps >= 10_000 {
        return Err("alpha_bps must be in 0..10000".into());
    }
    validate_outcomes(outcomes)?;

    if outcomes.len() != bounds.len() {
        return Err("probability bounds must cover every scenario exactly once".into());
    }

    let outcome_by_id: BTreeMap<&str, &ScenarioOutcome> = outcomes
        .iter()
        .map(|outcome| (outcome.scenario_id.as_str(), outcome))
        .collect();
    let mut bound_by_id = BTreeMap::<String, &ScenarioProbabilityBound>::new();
    let mut lower_total = 0_u128;
    let mut upper_total = 0_u128;

    for bound in bounds {
        if bound.scenario_id.trim().is_empty() {
            return Err("probability bound scenario_id must not be empty".into());
        }
        if bound.lower_bps > bound.upper_bps || bound.upper_bps > 10_000 {
            return Err(format!(
                "scenario {} has invalid probability bounds",
                bound.scenario_id
            ));
        }
        if !outcome_by_id.contains_key(bound.scenario_id.as_str()) {
            return Err(format!(
                "probability bound references unknown scenario {}",
                bound.scenario_id
            ));
        }
        if bound_by_id
            .insert(bound.scenario_id.clone(), bound)
            .is_some()
        {
            return Err(format!(
                "duplicate probability bound for scenario {}",
                bound.scenario_id
            ));
        }
        checked_add_u128(
            &mut lower_total,
            u128::from(bound.lower_bps),
            "lower probability bound total",
        )?;
        checked_add_u128(
            &mut upper_total,
            u128::from(bound.upper_bps),
            "upper probability bound total",
        )?;
    }

    if lower_total > 10_000 || upper_total < 10_000 {
        return Err("probability intervals do not contain a sum-to-10000-bps distribution".into());
    }

    let mut selected: BTreeMap<String, u16> = bound_by_id
        .iter()
        .map(|(id, bound)| (id.clone(), bound.lower_bps))
        .collect();
    let mut remaining = 10_000_u128 - lower_total;

    let mut ordered: Vec<&ScenarioOutcome> = outcomes.iter().collect();
    ordered.sort_by(|a, b| {
        b.total_critical_unserved_l
            .cmp(&a.total_critical_unserved_l)
            .then_with(|| a.scenario_id.cmp(&b.scenario_id))
    });

    for outcome in ordered {
        if remaining == 0 {
            break;
        }
        let bound = bound_by_id
            .get(outcome.scenario_id.as_str())
            .ok_or_else(|| format!("missing probability bound for {}", outcome.scenario_id))?;
        let current = u128::from(selected[&outcome.scenario_id]);
        let available = u128::from(bound.upper_bps) - current;
        let assigned = available.min(remaining);
        let new_value = current
            .checked_add(assigned)
            .ok_or_else(|| "scenario probability assignment overflowed u128".to_string())?;
        let new_value =
            u16::try_from(new_value).map_err(|_| "scenario probability exceeds u16".to_string())?;
        selected.insert(outcome.scenario_id.clone(), new_value);
        remaining -= assigned;
    }

    if remaining != 0 {
        return Err("probability upper bounds cannot absorb remaining mass".into());
    }

    let worst_case_distribution: Vec<ScenarioProbability> = selected
        .iter()
        .map(|(scenario_id, probability_bps)| ScenarioProbability {
            scenario_id: scenario_id.clone(),
            probability_bps: *probability_bps,
        })
        .collect();

    let losses = outcomes
        .iter()
        .map(|outcome| {
            (
                outcome.total_critical_unserved_l,
                u64::from(selected[&outcome.scenario_id]),
            )
        })
        .collect::<Vec<_>>();

    let total_weight = 10_000_u128;
    Ok(DistributionallyRobustRisk {
        alpha_bps,
        worst_case_expected_critical_unserved_l_ceil: weighted_mean_ceil(&losses, total_weight)?,
        worst_case_cvar_critical_unserved_l_ceil: weighted_cvar_ceil(
            &losses,
            total_weight,
            alpha_bps,
        )?,
        worst_case_distribution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! outcome {
        (
            $id:expr, $weight:expr, $demand:expr, $delivered:expr,
            $unserved:expr, $total_unserved:expr, $zone_a_unserved:expr, $cost:expr
        ) => {
            ScenarioOutcome {
                scenario_id: $id.to_string(),
                weight: $weight,
                total_unserved_l: $total_unserved,
                total_critical_demand_l: $demand,
                total_critical_delivered_l: $delivered,
                total_critical_unserved_l: $unserved,
                logistics_cost_units: $cost,
                zones: vec![ZoneOutcome {
                    zone_id: "zone-a".into(),
                    critical_demand_l: $demand,
                    critical_delivered_l: $demand - $zone_a_unserved,
                    critical_unserved_l: $zone_a_unserved,
                }],
            }
        };
    }

    #[test]
    fn equal_weight_tail_average_uses_fractional_boundary_exactly() {
        let outcomes = (0..10)
            .map(|loss| outcome!(&format!("s{loss}"), 1, 10, 10 - loss, loss, loss, loss, 1))
            .collect::<Vec<_>>();
        let metrics = TournamentMetrics::evaluate(&outcomes, 8_000).unwrap();
        assert_eq!(metrics.var_critical_unserved_l, 7);
        assert_eq!(metrics.cvar_critical_unserved_l_ceil, 9);
        assert_eq!(metrics.worst_critical_unserved_l, 9);
    }

    #[test]
    fn weighted_tail_respects_probability_mass_not_scenario_count() {
        let outcomes = vec![
            outcome!("common", 95, 100, 100, 0, 0, 0, 1),
            outcome!("rare", 5, 100, 0, 100, 100, 100, 2),
        ];
        let metrics = TournamentMetrics::evaluate(&outcomes, 9_500).unwrap();
        assert_eq!(metrics.var_critical_unserved_l, 0);
        assert_eq!(metrics.cvar_critical_unserved_l_ceil, 100);
        assert_eq!(metrics.expected_critical_unserved_l_ceil, 5);
    }

    #[test]
    fn zone_specific_tail_metrics_expose_geographic_concentration() {
        let first = ScenarioOutcome {
            zones: vec![
                ZoneOutcome {
                    zone_id: "zone-a".into(),
                    critical_demand_l: 100,
                    critical_delivered_l: 100,
                    critical_unserved_l: 0,
                },
                ZoneOutcome {
                    zone_id: "zone-b".into(),
                    critical_demand_l: 100,
                    critical_delivered_l: 0,
                    critical_unserved_l: 100,
                },
            ],
            ..outcome!("s1", 1, 200, 100, 100, 100, 100, 1)
        };
        let second = ScenarioOutcome {
            zones: vec![
                ZoneOutcome {
                    zone_id: "zone-a".into(),
                    critical_demand_l: 100,
                    critical_delivered_l: 0,
                    critical_unserved_l: 100,
                },
                ZoneOutcome {
                    zone_id: "zone-b".into(),
                    critical_demand_l: 100,
                    critical_delivered_l: 100,
                    critical_unserved_l: 0,
                },
            ],
            ..outcome!("s2", 1, 200, 100, 100, 100, 0, 1)
        };
        let metrics = TournamentMetrics::evaluate(&[first, second], 5_000).unwrap();
        assert_eq!(metrics.zone_risk.len(), 2);
        assert_eq!(metrics.zone_risk[0].cvar_unserved_l_ceil, 100);
        assert_eq!(metrics.zone_risk[1].cvar_unserved_l_ceil, 100);
    }

    #[test]
    fn service_rate_is_weighted_by_demand_not_by_scenario_count() {
        let outcomes = vec![
            outcome!("large", 1, 1_000, 900, 100, 100, 100, 0),
            outcome!("small", 9, 10, 0, 10, 10, 10, 0),
        ];
        let metrics = TournamentMetrics::evaluate(&outcomes, 9_500).unwrap();
        assert_eq!(metrics.critical_service_rate_bps, 8_256);
    }

    #[test]
    fn matched_comparison_computes_cvar_reduction_and_service_delta() {
        let baseline = vec![
            outcome!("normal", 95, 100, 100, 0, 0, 0, 100),
            outcome!("disruption", 5, 100, 0, 100, 100, 100, 100),
        ];
        let candidate = vec![
            outcome!("normal", 95, 100, 100, 0, 0, 0, 110),
            outcome!("disruption", 5, 100, 50, 50, 50, 50, 120),
        ];
        let comparison = compare_matched(&baseline, &candidate, 9_500).unwrap();
        assert_eq!(comparison.baseline.cvar_critical_unserved_l_ceil, 100);
        assert_eq!(comparison.candidate.cvar_critical_unserved_l_ceil, 50);
        assert_eq!(comparison.cvar_reduction_bps, Some(5_000));
        assert_eq!(comparison.critical_service_delta_bps, 250);
        assert_eq!(comparison.logistics_cost_delta_units, 11);
    }

    #[test]
    fn matched_comparison_rejects_different_scenario_weights() {
        let baseline = vec![outcome!("s1", 1, 10, 10, 0, 0, 0, 1)];
        let candidate = vec![outcome!("s1", 2, 10, 10, 0, 0, 0, 1)];
        assert!(compare_matched(&baseline, &candidate, 9_500).is_err());
    }

    #[test]
    fn duplicate_scenarios_and_zero_weights_fail_closed() {
        let duplicate = outcome!("same", 1, 10, 10, 0, 0, 0, 1);
        assert!(TournamentMetrics::evaluate(&[duplicate.clone(), duplicate], 9_500).is_err());
        assert!(
            TournamentMetrics::evaluate(&[outcome!("zero", 0, 10, 10, 0, 0, 0, 1)], 9_500).is_err()
        );
    }

    #[test]
    fn inconsistent_zone_or_critical_totals_are_rejected() {
        let mut bad = outcome!("bad", 1, 10, 9, 1, 1, 1, 0);
        bad.zones[0].critical_unserved_l = 2;
        assert!(TournamentMetrics::evaluate(&[bad], 9_500).is_err());
    }

    #[test]
    fn distributional_ambiguity_allocates_mass_to_highest_losses() {
        let outcomes = vec![
            outcome!("low", 1, 100, 100, 0, 0, 0, 1),
            outcome!("medium", 1, 100, 50, 50, 50, 50, 1),
            outcome!("high", 1, 100, 0, 100, 100, 100, 1),
        ];
        let bounds = vec![
            ScenarioProbabilityBound {
                scenario_id: "low".into(),
                lower_bps: 2_000,
                upper_bps: 5_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "medium".into(),
                lower_bps: 2_000,
                upper_bps: 5_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "high".into(),
                lower_bps: 2_000,
                upper_bps: 5_000,
            },
        ];
        let risk = distributionally_robust_risk(&outcomes, &bounds, 8_000).unwrap();
        assert_eq!(risk.worst_case_expected_critical_unserved_l_ceil, 65);
        assert_eq!(risk.worst_case_cvar_critical_unserved_l_ceil, 100);
        assert_eq!(
            risk.worst_case_distribution,
            vec![
                ScenarioProbability {
                    scenario_id: "high".into(),
                    probability_bps: 5_000
                },
                ScenarioProbability {
                    scenario_id: "low".into(),
                    probability_bps: 2_000
                },
                ScenarioProbability {
                    scenario_id: "medium".into(),
                    probability_bps: 3_000
                },
            ]
        );
    }

    #[test]
    fn probability_bounds_must_form_a_feasible_simplex() {
        let outcomes = vec![
            outcome!("s1", 1, 10, 10, 0, 0, 0, 0),
            outcome!("s2", 1, 10, 0, 10, 10, 10, 0),
        ];
        let too_much_lower = vec![
            ScenarioProbabilityBound {
                scenario_id: "s1".into(),
                lower_bps: 6_000,
                upper_bps: 8_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "s2".into(),
                lower_bps: 6_000,
                upper_bps: 8_000,
            },
        ];
        assert!(distributionally_robust_risk(&outcomes, &too_much_lower, 9_500).is_err());

        let too_little_upper = vec![
            ScenarioProbabilityBound {
                scenario_id: "s1".into(),
                lower_bps: 1_000,
                upper_bps: 4_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "s2".into(),
                lower_bps: 1_000,
                upper_bps: 4_000,
            },
        ];
        assert!(distributionally_robust_risk(&outcomes, &too_little_upper, 9_500).is_err());
    }

    #[test]
    fn ambiguity_bounds_require_exact_scenario_identity() {
        let outcomes = vec![
            outcome!("s1", 1, 10, 10, 0, 0, 0, 0),
            outcome!("s2", 1, 10, 0, 10, 10, 10, 0),
        ];
        let missing = vec![
            ScenarioProbabilityBound {
                scenario_id: "s1".into(),
                lower_bps: 1_000,
                upper_bps: 9_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "other".into(),
                lower_bps: 1_000,
                upper_bps: 9_000,
            },
        ];
        assert!(distributionally_robust_risk(&outcomes, &missing, 9_500).is_err());

        let duplicate = vec![
            ScenarioProbabilityBound {
                scenario_id: "s1".into(),
                lower_bps: 1_000,
                upper_bps: 9_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "s1".into(),
                lower_bps: 1_000,
                upper_bps: 9_000,
            },
        ];
        assert!(distributionally_robust_risk(&outcomes, &duplicate, 9_500).is_err());
    }

    #[test]
    fn robust_expected_shortfall_dominates_a_feasible_nominal_distribution() {
        let outcomes = vec![
            outcome!("low", 8, 100, 100, 0, 0, 0, 0),
            outcome!("high", 2, 100, 0, 100, 100, 100, 0),
        ];
        let bounds = vec![
            ScenarioProbabilityBound {
                scenario_id: "low".into(),
                lower_bps: 0,
                upper_bps: 9_000,
            },
            ScenarioProbabilityBound {
                scenario_id: "high".into(),
                lower_bps: 1_000,
                upper_bps: 10_000,
            },
        ];
        let robust = distributionally_robust_risk(&outcomes, &bounds, 9_500).unwrap();
        let nominal = TournamentMetrics::evaluate(&outcomes, 9_500).unwrap();
        assert!(
            robust.worst_case_expected_critical_unserved_l_ceil
                >= nominal.expected_critical_unserved_l_ceil
        );
        assert!(
            robust.worst_case_cvar_critical_unserved_l_ceil
                >= nominal.cvar_critical_unserved_l_ceil
        );
    }

    #[test]
    fn alpha_out_of_range_is_rejected() {
        let input = [outcome!("s1", 1, 10, 10, 0, 0, 0, 1)];
        assert!(TournamentMetrics::evaluate(&input, 10_000).is_err());
    }
}
