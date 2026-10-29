//! Evidence-aware Pareto comparison for candidates evaluated on the same mission.
//!
//! This is a ranking aid for screening only. It intentionally refuses to mix different
//! missions or parameter evidence classes and never translates a Pareto result into flight
//! authorization, structural adequacy, or manufacturing release.

use crate::{ResultRow, MODEL_VERSION};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct ParetoReport {
    pub model_version: &'static str,
    pub evidence_level: &'static str,
    pub parameter_basis: String,
    pub eligible_candidate_ids: Vec<String>,
    pub infeasible_candidate_ids: Vec<String>,
    pub dominated_candidate_ids: Vec<String>,
    pub pareto_candidate_ids: Vec<String>,
}

pub fn compare_candidates(rows: &[ResultRow]) -> Result<ParetoReport, String> {
    if rows.is_empty() {
        return Err("candidate comparison requires at least one evaluated candidate".to_owned());
    }

    // Recompute metrics from the stored inputs instead of trusting caller-constructed
    // ResultRow fields; the comparator is an independent arithmetic boundary.
    let evaluated: Vec<ResultRow> = rows
        .iter()
        .map(|row| row.case.evaluate())
        .collect::<Result<_, _>>()?;

    let reference = &evaluated[0].case;
    let mut seen = HashSet::new();
    for row in &evaluated {
        if row.case.case_id.trim().is_empty() {
            return Err("candidate case_id must not be empty".to_owned());
        }
        if !seen.insert(row.case.case_id.clone()) {
            return Err(format!("duplicate candidate ID: {}", row.case.case_id));
        }
        if row.case.parameter_basis != reference.parameter_basis {
            return Err(
                "refusing to rank candidates with mixed parameter_basis evidence classes"
                    .to_owned(),
            );
        }
        if !same_mission(reference, &row.case) {
            return Err(format!(
                "candidate {} does not share the comparison mission with {}",
                row.case.case_id, reference.case_id
            ));
        }
    }

    let mut report = ParetoReport {
        model_version: MODEL_VERSION,
        evidence_level: "screening_only",
        parameter_basis: reference.parameter_basis.clone(),
        eligible_candidate_ids: Vec::new(),
        infeasible_candidate_ids: Vec::new(),
        dominated_candidate_ids: Vec::new(),
        pareto_candidate_ids: Vec::new(),
    };

    let eligible: Vec<&ResultRow> = evaluated
        .iter()
        .filter(|row| row.lift_screen_pass && row.energy_screen_pass)
        .collect();

    for row in &evaluated {
        if row.lift_screen_pass && row.energy_screen_pass {
            report.eligible_candidate_ids.push(row.case.case_id.clone());
        } else {
            report.infeasible_candidate_ids.push(row.case.case_id.clone());
        }
    }

    for candidate in &eligible {
        let is_dominated = eligible
            .iter()
            .any(|other| other.case.case_id != candidate.case.case_id && dominates(other, candidate));
        if is_dominated {
            report.dominated_candidate_ids.push(candidate.case.case_id.clone());
        } else {
            report.pareto_candidate_ids.push(candidate.case.case_id.clone());
        }
    }

    Ok(report)
}

fn same_mission(a: &crate::Case, b: &crate::Case) -> bool {
    a.payload_mass_kg == b.payload_mass_kg
        && a.cruise_speed_m_s == b.cruise_speed_m_s
        && a.air_density_kg_m3 == b.air_density_kg_m3
        && a.propulsive_efficiency == b.propulsive_efficiency
        && a.battery_energy_wh == b.battery_energy_wh
        && a.reserve_fraction == b.reserve_fraction
        && a.route_distance_km == b.route_distance_km
        && a.assumed_max_lift_coefficient == b.assumed_max_lift_coefficient
}

/// Lower route energy, lower total mass and higher estimated L/D are treated as objectives.
/// Mass is only a design proxy here; it does not establish strength or structural adequacy.
fn dominates(a: &ResultRow, b: &ResultRow) -> bool {
    let no_worse = a.route_energy_wh <= b.route_energy_wh
        && a.total_mass_kg <= b.total_mass_kg
        && a.estimated_lift_to_drag >= b.estimated_lift_to_drag;
    let strictly_better = a.route_energy_wh < b.route_energy_wh
        || a.total_mass_kg < b.total_mass_kg
        || a.estimated_lift_to_drag > b.estimated_lift_to_drag;
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Case;

    fn case(id: &str) -> Case {
        Case {
            case_id: id.to_owned(),
            parameter_basis: "synthetic".to_owned(),
            source_ref: "synthetic-test-fixture".to_owned(),
            source_sha256: "-".to_owned(),
            airframe_mass_kg: 2.0,
            payload_mass_kg: 0.5,
            wing_area_m2: 0.55,
            aspect_ratio: 8.0,
            oswald_efficiency: 0.8,
            cd0: 0.035,
            cruise_speed_m_s: 15.0,
            air_density_kg_m3: 1.225,
            propulsive_efficiency: 0.65,
            battery_energy_wh: 160.0,
            reserve_fraction: 0.2,
            route_distance_km: 20.0,
            assumed_max_lift_coefficient: 1.2,
        }
    }

    #[test]
    fn dominated_design_is_removed_without_weighted_score() {
        let baseline = case("baseline").evaluate().expect("baseline");
        let mut worse = case("higher-drag");
        worse.cd0 *= 1.1;
        let worse = worse.evaluate().expect("higher drag");
        let report = compare_candidates(&[baseline, worse]).expect("same mission");
        assert_eq!(report.eligible_candidate_ids, vec!["baseline".to_owned(), "higher-drag".to_owned()]);
        assert_eq!(report.dominated_candidate_ids, vec!["higher-drag".to_owned()]);
        assert_eq!(report.pareto_candidate_ids, vec!["baseline".to_owned()]);
        assert_eq!(report.evidence_level, "screening_only");
    }

    #[test]
    fn refuses_to_compare_different_missions() {
        let baseline = case("baseline").evaluate().expect("baseline");
        let mut different_route = case("different-route");
        different_route.route_distance_km = 25.0;
        let different_route = different_route.evaluate().expect("valid");
        assert!(compare_candidates(&[baseline, different_route]).is_err());
    }

    #[test]
    fn refuses_to_mix_evidence_classes() {
        let baseline = case("baseline").evaluate().expect("baseline");
        let mut declared = case("declared");
        declared.parameter_basis = "estimate".to_owned();
        declared.source_sha256 = "a".repeat(64);
        let declared = declared.evaluate().expect("well-formed declaration");
        assert!(compare_candidates(&[baseline, declared]).is_err());
    }

    #[test]
    fn infeasible_candidate_cannot_enter_pareto_front() {
        let baseline = case("baseline").evaluate().expect("baseline");
        let mut insufficient = case("insufficient-energy");
        // Keep mission inputs fixed; increase drag enough to fail the energy gate.
        insufficient.cd0 = 0.25;
        let insufficient = insufficient.evaluate().expect("valid input, infeasible screen");
        let report = compare_candidates(&[baseline, insufficient]).expect("same mission");
        assert_eq!(report.infeasible_candidate_ids, vec!["insufficient-energy".to_owned()]);
        assert_eq!(report.pareto_candidate_ids, ["baseline"]);
    }

    #[test]
    fn comparator_does_not_trust_caller_supplied_derived_metrics() {
        let baseline = case("baseline").evaluate().expect("baseline");
        let mut worse = case("worse");
        worse.cd0 = 0.040;
        let mut forged = worse.evaluate().expect("worse candidate");
        // Simulate a caller attempting to make a dominated candidate look best.
        forged.route_energy_wh = 0.0;
        forged.estimated_lift_to_drag = 1.0e9;
        let report = compare_candidates(&[baseline, forged]).expect("same mission");
        assert_eq!(report.pareto_candidate_ids, vec!["baseline".to_owned()]);
        assert_eq!(report.dominated_candidate_ids, vec!["worse".to_owned()]);
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let a = case("same").evaluate().expect("candidate");
        let b = case("same").evaluate().expect("duplicate candidate");
        assert!(compare_candidates(&[a, b]).is_err());
    }
}
