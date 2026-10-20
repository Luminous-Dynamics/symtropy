// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compare declared recovery plans with an independent observation lane.
//!
//! Planned ledger values are never substituted for observations. The comparator
//! preserves signed residuals and does not force observed mass to balance.

use crate::{Check, World, validate_token};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    Measured,
    Synthetic,
    Derived,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryObservation {
    pub component_id: String,
    /// Independently observed mass of the component before dismantling.
    pub observed_input_mass_mg: u64,
    pub reused_mg: u64,
    pub recycled_mg: u64,
    pub contaminated_mg: u64,
    pub waste_mg: u64,
    pub evidence_class: EvidenceClass,
    pub source_ref: Option<String>,
    pub calibration_ref: Option<String>,
    /// Declared absolute uncertainty bound in the same integer-milligram basis.
    pub measurement_uncertainty_mg: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRecoveryDelta {
    pub component_id: String,
    pub planned_input_mass_mg: u64,
    pub observed_input_mass_mg: u64,
    /// Observed minus planned; a negative value means the observed input mass
    /// was lower than the planned component mass.
    pub input_mass_delta_mg: i128,
    pub planned_reused_mg: u64,
    pub observed_reused_mg: u64,
    pub reused_delta_mg: i128,
    pub planned_recycled_mg: u64,
    pub observed_recycled_mg: u64,
    pub recycled_delta_mg: i128,
    pub planned_contaminated_mg: u64,
    pub observed_contaminated_mg: u64,
    pub contaminated_delta_mg: i128,
    pub planned_waste_mg: u64,
    pub observed_waste_mg: u64,
    pub waste_delta_mg: i128,
    pub planned_unaccounted_mg: u64,
    /// Observed input mass minus observed reused/recycled/contaminated/waste
    /// mass. This can be negative when reported outputs exceed measured input.
    pub observed_unaccounted_mg: i128,
    pub unaccounted_delta_mg: i128,
    pub observed_mass_residual_mg: i128,
    pub evidence_class: EvidenceClass,
    pub measurement_uncertainty_mg: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStatus {
    Measured,
    Synthetic,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanAgreement {
    ExactMatch,
    Deviates,
    Unresolved,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AggregateRecoveryDelta {
    /// Totals are None unless every planned recovery record has an observation.
    pub input_mass_delta_mg: Option<i128>,
    pub reused_delta_mg: Option<i128>,
    pub recycled_delta_mg: Option<i128>,
    pub contaminated_delta_mg: Option<i128>,
    pub waste_delta_mg: Option<i128>,
    pub unaccounted_delta_mg: Option<i128>,
    /// Signed observed input minus observed outputs.
    pub observed_mass_residual_mg: Option<i128>,
    /// Conservative sum of declared absolute bounds; None unless all are known.
    pub conservative_uncertainty_bound_mg: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryComparison {
    pub source_revision: u64,
    /// Equality binding for the state being compared; not a cryptographic digest.
    pub source_state_snapshot: String,
    pub evidence_status: EvidenceStatus,
    pub plan_agreement: PlanAgreement,
    pub planned_component_count: usize,
    pub matched_observation_count: usize,
    pub missing_observation_ids: Vec<String>,
    pub components: Vec<ComponentRecoveryDelta>,
    pub aggregate: AggregateRecoveryDelta,
    pub diagnostics: Vec<String>,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryFixtureSummary {
    pub evidence_status: EvidenceStatus,
    pub plan_agreement: PlanAgreement,
    pub planned_component_count: usize,
    pub observed_mass_residual_mg: Option<i128>,
    pub reused_delta_mg: Option<i128>,
    pub recycled_delta_mg: Option<i128>,
    pub contaminated_delta_mg: Option<i128>,
    pub waste_delta_mg: Option<i128>,
    pub unaccounted_delta_mg: Option<i128>,
}

/// Compare observation records against declared recovery records in the world.
/// Missing observations remain missing; unknown/derived/mixed evidence cannot
/// promote the result to MEASURED.
pub fn compare(world: &World, observations: Vec<RecoveryObservation>) -> Check<RecoveryComparison> {
    if world.recovered_components.is_empty() {
        return Err("no recovered components exist in the source state".to_owned());
    }

    let mut by_component = BTreeMap::<String, RecoveryObservation>::new();
    for observation in observations {
        validate_token(
            &observation.component_id,
            "recovery observation component ID",
        )?;
        if observation.observed_input_mass_mg == 0 {
            return Err(format!(
                "observation {} has zero input mass",
                observation.component_id
            ));
        }
        if let Some(source_ref) = observation.source_ref.as_deref()
            && source_ref.trim().is_empty()
        {
            return Err(format!(
                "observation {} has an empty source reference",
                observation.component_id
            ));
        }
        if let Some(calibration_ref) = observation.calibration_ref.as_deref()
            && calibration_ref.trim().is_empty()
        {
            return Err(format!(
                "observation {} has an empty calibration reference",
                observation.component_id
            ));
        }
        let id = observation.component_id.clone();
        if by_component.insert(id.clone(), observation).is_some() {
            return Err(format!(
                "duplicate recovery observation for component: {id}"
            ));
        }
    }

    for id in by_component.keys() {
        if !world.recovered_components.contains_key(id) {
            return Err(format!(
                "observation references component without a recovery record: {id}"
            ));
        }
    }

    let mut components = Vec::new();
    let mut missing = Vec::new();
    let mut diagnostics = Vec::new();
    let mut all_measured_qualified = true;
    let mut all_synthetic = true;
    let mut uncertainty_complete = true;
    let mut uncertainty_bound = 0_u64;

    for planned in world.recovered_components.values() {
        let Some(observation) = by_component.get(&planned.component_id) else {
            missing.push(planned.component_id.clone());
            diagnostics.push(format!(
                "missing observation for recovered component {}",
                planned.component_id
            ));
            all_measured_qualified = false;
            all_synthetic = false;
            uncertainty_complete = false;
            continue;
        };

        let source_and_calibration_present = observation
            .source_ref
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            && observation
                .calibration_ref
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
        let uncertainty_present = observation.measurement_uncertainty_mg.is_some();

        if observation.evidence_class != EvidenceClass::Measured
            || !source_and_calibration_present
            || !uncertainty_present
        {
            all_measured_qualified = false;
        }
        if observation.evidence_class != EvidenceClass::Synthetic {
            all_synthetic = false;
        }
        match observation.measurement_uncertainty_mg {
            Some(value) => {
                uncertainty_bound = uncertainty_bound
                    .checked_add(value)
                    .ok_or_else(|| "aggregate recovery uncertainty overflow".to_owned())?;
            }
            None => {
                uncertainty_complete = false;
                diagnostics.push(format!(
                    "measurement uncertainty missing for component {}",
                    planned.component_id
                ));
            }
        }
        if observation.evidence_class == EvidenceClass::Measured && !source_and_calibration_present
        {
            diagnostics.push(format!(
                "measured observation {} lacks source or calibration provenance",
                planned.component_id
            ));
        }
        if observation.evidence_class == EvidenceClass::Measured && !uncertainty_present {
            diagnostics.push(format!(
                "measured observation {} lacks an uncertainty bound",
                planned.component_id
            ));
        }
        if matches!(
            observation.evidence_class,
            EvidenceClass::Derived | EvidenceClass::Unknown
        ) {
            diagnostics.push(format!(
                "observation {} is not wholly measured or wholly synthetic",
                planned.component_id
            ));
        }

        let observed_outputs = i128::from(observation.reused_mg)
            + i128::from(observation.recycled_mg)
            + i128::from(observation.contaminated_mg)
            + i128::from(observation.waste_mg);
        let observed_input = i128::from(observation.observed_input_mass_mg);
        let observed_unaccounted = observed_input - observed_outputs;
        let unaccounted_delta = observed_unaccounted - i128::from(planned.unaccounted_mg);
        let component_delta = ComponentRecoveryDelta {
            component_id: planned.component_id.clone(),
            planned_input_mass_mg: planned.input_mass_mg,
            observed_input_mass_mg: observation.observed_input_mass_mg,
            input_mass_delta_mg: observed_input - i128::from(planned.input_mass_mg),
            planned_reused_mg: planned.reused_mg,
            observed_reused_mg: observation.reused_mg,
            reused_delta_mg: i128::from(observation.reused_mg) - i128::from(planned.reused_mg),
            planned_recycled_mg: planned.recycled_mg,
            observed_recycled_mg: observation.recycled_mg,
            recycled_delta_mg: i128::from(observation.recycled_mg)
                - i128::from(planned.recycled_mg),
            planned_contaminated_mg: planned.contaminated_mg,
            observed_contaminated_mg: observation.contaminated_mg,
            contaminated_delta_mg: i128::from(observation.contaminated_mg)
                - i128::from(planned.contaminated_mg),
            planned_waste_mg: planned.waste_mg,
            observed_waste_mg: observation.waste_mg,
            waste_delta_mg: i128::from(observation.waste_mg) - i128::from(planned.waste_mg),
            planned_unaccounted_mg: planned.unaccounted_mg,
            observed_unaccounted_mg: observed_unaccounted,
            unaccounted_delta_mg: unaccounted_delta,
            observed_mass_residual_mg: observed_unaccounted,
            evidence_class: observation.evidence_class,
            measurement_uncertainty_mg: observation.measurement_uncertainty_mg,
        };
        if component_delta.observed_mass_residual_mg != 0 {
            diagnostics.push(format!(
                "observed mass residual for {} is {} mg",
                component_delta.component_id, component_delta.observed_mass_residual_mg
            ));
        }
        components.push(component_delta);
    }

    missing.sort();
    components.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    let fully_covered = missing.is_empty() && components.len() == world.recovered_components.len();

    let aggregate = if fully_covered {
        let mut sums = AggregateRecoveryDelta {
            input_mass_delta_mg: Some(0),
            reused_delta_mg: Some(0),
            recycled_delta_mg: Some(0),
            contaminated_delta_mg: Some(0),
            waste_delta_mg: Some(0),
            unaccounted_delta_mg: Some(0),
            observed_mass_residual_mg: Some(0),
            conservative_uncertainty_bound_mg: if uncertainty_complete {
                Some(uncertainty_bound)
            } else {
                None
            },
        };
        for component in &components {
            add_i128(&mut sums.input_mass_delta_mg, component.input_mass_delta_mg)?;
            add_i128(&mut sums.reused_delta_mg, component.reused_delta_mg)?;
            add_i128(&mut sums.recycled_delta_mg, component.recycled_delta_mg)?;
            add_i128(
                &mut sums.contaminated_delta_mg,
                component.contaminated_delta_mg,
            )?;
            add_i128(&mut sums.waste_delta_mg, component.waste_delta_mg)?;
            add_i128(
                &mut sums.unaccounted_delta_mg,
                component.unaccounted_delta_mg,
            )?;
            add_i128(
                &mut sums.observed_mass_residual_mg,
                component.observed_mass_residual_mg,
            )?;
        }
        sums
    } else {
        diagnostics.push(
            "aggregate deltas withheld because observation coverage is incomplete".to_owned(),
        );
        AggregateRecoveryDelta::default()
    };

    let evidence_status = if fully_covered && all_measured_qualified {
        EvidenceStatus::Measured
    } else if fully_covered && all_synthetic {
        EvidenceStatus::Synthetic
    } else {
        EvidenceStatus::Unresolved
    };
    let plan_agreement = if !fully_covered {
        PlanAgreement::Unresolved
    } else if components.iter().all(|component| {
        component.input_mass_delta_mg == 0
            && component.reused_delta_mg == 0
            && component.recycled_delta_mg == 0
            && component.contaminated_delta_mg == 0
            && component.waste_delta_mg == 0
            && component.unaccounted_delta_mg == 0
    }) {
        PlanAgreement::ExactMatch
    } else {
        PlanAgreement::Deviates
    };
    if evidence_status == EvidenceStatus::Unresolved {
        diagnostics.push(
            "comparison is unresolved: evidence classes/provenance/uncertainty or observation coverage are insufficient"
                .to_owned(),
        );
    }

    Ok(RecoveryComparison {
        source_revision: world.revision,
        source_state_snapshot: world.canonical_snapshot(),
        evidence_status,
        plan_agreement,
        planned_component_count: world.recovered_components.len(),
        matched_observation_count: components.len(),
        missing_observation_ids: missing,
        components,
        aggregate,
        diagnostics,
        claim_ceiling: "plan-versus-observation accounting only; no material certificate or environmental-benefit claim",
    })
}

/// Frozen synthetic case: the plan allocates 500/300/50/100/50 mg, while the
/// observation records 450/280/70/160 mg outputs from 1,000 mg input. The
/// resulting 40 mg residual is returned, not imputed away.
pub fn run_reference_fixture() -> Check<RecoveryFixtureSummary> {
    let mut world = World::default();
    world.apply(crate::Event {
        id: "fixture-install-A".to_owned(),
        operation: crate::Operation::InstallComponent(crate::Component {
            id: "A".to_owned(),
            material: "steel".to_owned(),
            mass_mg: 1_000,
        }),
    })?;
    world.apply(crate::Event {
        id: "fixture-recover-A".to_owned(),
        operation: crate::Operation::RecoverComponent {
            component_id: "A".to_owned(),
            reused_mg: 500,
            recycled_mg: 300,
            contaminated_mg: 50,
            waste_mg: 100,
            unaccounted_mg: 50,
        },
    })?;

    let result = compare(
        &world,
        vec![RecoveryObservation {
            component_id: "A".to_owned(),
            observed_input_mass_mg: 1_000,
            reused_mg: 450,
            recycled_mg: 280,
            contaminated_mg: 70,
            waste_mg: 160,
            evidence_class: EvidenceClass::Synthetic,
            source_ref: Some("fixture://recovery-A".to_owned()),
            calibration_ref: None,
            measurement_uncertainty_mg: None,
        }],
    )?;
    let summary = RecoveryFixtureSummary {
        evidence_status: result.evidence_status,
        plan_agreement: result.plan_agreement,
        planned_component_count: result.planned_component_count,
        observed_mass_residual_mg: result.aggregate.observed_mass_residual_mg,
        reused_delta_mg: result.aggregate.reused_delta_mg,
        recycled_delta_mg: result.aggregate.recycled_delta_mg,
        contaminated_delta_mg: result.aggregate.contaminated_delta_mg,
        waste_delta_mg: result.aggregate.waste_delta_mg,
        unaccounted_delta_mg: result.aggregate.unaccounted_delta_mg,
    };
    if summary.evidence_status != EvidenceStatus::Synthetic
        || summary.plan_agreement != PlanAgreement::Deviates
        || summary.planned_component_count != 1
        || summary.observed_mass_residual_mg != Some(40)
        || summary.reused_delta_mg != Some(-50)
        || summary.recycled_delta_mg != Some(-20)
        || summary.contaminated_delta_mg != Some(20)
        || summary.waste_delta_mg != Some(60)
        || summary.unaccounted_delta_mg != Some(-10)
    {
        return Err(format!(
            "frozen recovery comparison failed known-answer contract: {summary:?}"
        ));
    }
    Ok(summary)
}

fn add_i128(target: &mut Option<i128>, delta: i128) -> Check<()> {
    if let Some(current) = target {
        *current = current
            .checked_add(delta)
            .ok_or_else(|| "aggregate recovery delta overflow".to_owned())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Event, Operation};

    fn recovered_world() -> World {
        let mut world = World::default();
        world
            .apply(Event {
                id: "install-A".to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: "A".to_owned(),
                    material: "steel".to_owned(),
                    mass_mg: 1_000,
                }),
            })
            .expect("component installation");
        world
            .apply(Event {
                id: "recover-A".to_owned(),
                operation: Operation::RecoverComponent {
                    component_id: "A".to_owned(),
                    reused_mg: 500,
                    recycled_mg: 300,
                    contaminated_mg: 50,
                    waste_mg: 100,
                    unaccounted_mg: 50,
                },
            })
            .expect("planned recovery");
        world
    }

    fn synthetic_observation(id: &str) -> RecoveryObservation {
        RecoveryObservation {
            component_id: id.to_owned(),
            observed_input_mass_mg: 1_000,
            reused_mg: 450,
            recycled_mg: 280,
            contaminated_mg: 70,
            waste_mg: 160,
            evidence_class: EvidenceClass::Synthetic,
            source_ref: Some("fixture://scale-log-A".to_owned()),
            calibration_ref: None,
            measurement_uncertainty_mg: None,
        }
    }

    fn measured_observation(id: &str) -> RecoveryObservation {
        RecoveryObservation {
            evidence_class: EvidenceClass::Measured,
            source_ref: Some("urn:fixture:field-note-A".to_owned()),
            calibration_ref: Some("urn:fixture:scale-calibration-2026-01".to_owned()),
            measurement_uncertainty_mg: Some(5),
            ..synthetic_observation(id)
        }
    }

    #[test]
    fn synthetic_comparison_reports_category_delta_and_nonzero_residual() {
        let world = recovered_world();
        let result =
            compare(&world, vec![synthetic_observation("A")]).expect("synthetic observations");
        assert_eq!(result.evidence_status, EvidenceStatus::Synthetic);
        assert_eq!(result.plan_agreement, PlanAgreement::Deviates);
        assert_eq!(result.planned_component_count, 1);
        assert_eq!(result.aggregate.input_mass_delta_mg, Some(0));
        assert_eq!(result.aggregate.reused_delta_mg, Some(-50));
        assert_eq!(result.aggregate.recycled_delta_mg, Some(-20));
        assert_eq!(result.aggregate.contaminated_delta_mg, Some(20));
        assert_eq!(result.aggregate.waste_delta_mg, Some(60));
        assert_eq!(result.aggregate.unaccounted_delta_mg, Some(-10));
        assert_eq!(result.aggregate.observed_mass_residual_mg, Some(40));
    }

    #[test]
    fn exact_plan_match_can_include_declared_nonzero_unaccounted_mass() {
        let world = recovered_world();
        let mut observation = synthetic_observation("A");
        observation.reused_mg = 500;
        observation.recycled_mg = 300;
        observation.contaminated_mg = 50;
        observation.waste_mg = 100;
        let result = compare(&world, vec![observation]).expect("exact plan match");
        assert_eq!(result.evidence_status, EvidenceStatus::Synthetic);
        assert_eq!(result.plan_agreement, PlanAgreement::ExactMatch);
        assert_eq!(result.aggregate.observed_mass_residual_mg, Some(50));
        assert_eq!(result.aggregate.unaccounted_delta_mg, Some(0));
    }

    #[test]
    fn measured_status_requires_provenance_calibration_and_uncertainty() {
        let world = recovered_world();
        let result = compare(&world, vec![measured_observation("A")])
            .expect("qualified measured observation");
        assert_eq!(result.evidence_status, EvidenceStatus::Measured);
        assert_eq!(result.plan_agreement, PlanAgreement::Deviates);
        assert_eq!(result.aggregate.conservative_uncertainty_bound_mg, Some(5));

        let mut incomplete = measured_observation("A");
        incomplete.calibration_ref = None;
        let result = compare(&world, vec![incomplete]).expect("missing calibration is unresolved");
        assert_eq!(result.evidence_status, EvidenceStatus::Unresolved);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|value| value.contains("calibration provenance"))
        );

        let mut no_uncertainty = measured_observation("A");
        no_uncertainty.measurement_uncertainty_mg = None;
        let result = compare(&world, vec![no_uncertainty])
            .expect("missing uncertainty is explicitly unresolved");
        assert_eq!(result.evidence_status, EvidenceStatus::Unresolved);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|value| value.contains("uncertainty bound"))
        );
        assert_eq!(result.aggregate.conservative_uncertainty_bound_mg, None);
    }

    #[test]
    fn missing_observation_withholds_aggregate_totals() {
        let mut world = recovered_world();
        world
            .apply(Event {
                id: "install-B".to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: "B".to_owned(),
                    material: "timber".to_owned(),
                    mass_mg: 500,
                }),
            })
            .expect("second component");
        world
            .apply(Event {
                id: "recover-B".to_owned(),
                operation: Operation::RecoverComponent {
                    component_id: "B".to_owned(),
                    reused_mg: 200,
                    recycled_mg: 100,
                    contaminated_mg: 0,
                    waste_mg: 100,
                    unaccounted_mg: 100,
                },
            })
            .expect("second recovery");
        let result = compare(&world, vec![synthetic_observation("A")])
            .expect("missing observation remains explicit");
        assert_eq!(result.evidence_status, EvidenceStatus::Unresolved);
        assert_eq!(result.plan_agreement, PlanAgreement::Unresolved);
        assert_eq!(result.missing_observation_ids, vec!["B"]);
        assert_eq!(result.aggregate.input_mass_delta_mg, None);
        assert_eq!(result.aggregate.observed_mass_residual_mg, None);
    }

    #[test]
    fn mixed_evidence_classes_do_not_promote_comparison() {
        let mut world = recovered_world();
        world
            .apply(Event {
                id: "install-B".to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: "B".to_owned(),
                    material: "timber".to_owned(),
                    mass_mg: 500,
                }),
            })
            .expect("second component");
        world
            .apply(Event {
                id: "recover-B".to_owned(),
                operation: Operation::RecoverComponent {
                    component_id: "B".to_owned(),
                    reused_mg: 200,
                    recycled_mg: 100,
                    contaminated_mg: 0,
                    waste_mg: 100,
                    unaccounted_mg: 100,
                },
            })
            .expect("second recovery");
        let result = compare(
            &world,
            vec![measured_observation("A"), synthetic_observation("B")],
        )
        .expect("mixed evidence is representable");
        assert_eq!(result.evidence_status, EvidenceStatus::Unresolved);
        assert_eq!(result.plan_agreement, PlanAgreement::Deviates);
        assert_eq!(result.matched_observation_count, 2);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|value| value.contains("insufficient"))
        );
    }

    #[test]
    fn duplicate_and_foreign_observations_fail_closed() {
        let world = recovered_world();
        assert!(
            compare(
                &world,
                vec![synthetic_observation("A"), synthetic_observation("A")]
            )
            .unwrap_err()
            .contains("duplicate recovery observation")
        );
        assert!(
            compare(&world, vec![synthetic_observation("B")])
                .unwrap_err()
                .contains("without a recovery record")
        );
    }

    #[test]
    fn outputs_exceeding_input_are_reported_as_negative_residual() {
        let world = recovered_world();
        let mut observation = synthetic_observation("A");
        observation.reused_mg = 1_100;
        observation.recycled_mg = 0;
        observation.contaminated_mg = 0;
        observation.waste_mg = 0;
        let result =
            compare(&world, vec![observation]).expect("over-output is evidence, not parse failure");
        assert_eq!(result.evidence_status, EvidenceStatus::Synthetic);
        assert_eq!(result.plan_agreement, PlanAgreement::Deviates);
        assert_eq!(result.aggregate.observed_mass_residual_mg, Some(-100));
        assert!(
            result
                .diagnostics
                .iter()
                .any(|value| value.contains("mass residual"))
        );
    }

    #[test]
    fn observed_input_mass_must_be_positive() {
        let world = recovered_world();
        let mut observation = synthetic_observation("A");
        observation.observed_input_mass_mg = 0;
        assert!(
            compare(&world, vec![observation])
                .unwrap_err()
                .contains("zero input mass")
        );
    }

    #[test]
    fn frozen_recovery_comparison_receipt_matches_known_answers() {
        let result = run_reference_fixture().expect("frozen recovery comparison");
        assert_eq!(result.evidence_status, EvidenceStatus::Synthetic);
        assert_eq!(result.plan_agreement, PlanAgreement::Deviates);
        assert_eq!(result.observed_mass_residual_mg, Some(40));
        assert_eq!(result.reused_delta_mg, Some(-50));
        assert_eq!(result.recycled_delta_mg, Some(-20));
        assert_eq!(result.contaminated_delta_mg, Some(20));
        assert_eq!(result.waste_delta_mg, Some(60));
        assert_eq!(result.unaccounted_delta_mg, Some(-10));
    }
}
