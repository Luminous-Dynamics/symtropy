//! Evidence-bounded service-recovery curve metrics.
//!
//! This module consumes an explicitly supplied service curve. It does not simulate
//! tunnel damage, calculate fragility, resolve network dependencies, schedule repairs,
//! certify safety, or authorize re-entry. The curve's source and assumptions must be
//! validated outside this kernel.

use std::collections::BTreeSet;

use super::{EvidenceClass, GateState, ServiceBasis};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryEventClass {
    ShakingOnly,
    FaultDisplacement,
    LiquefactionOrLateralSpread,
    PortalOrSlopeFailure,
    WaterIngress,
    UtilityCommonCause,
    AftershockSequence,
    CombinedScenario,
}

impl RecoveryEventClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::ShakingOnly => "shaking_only",
            Self::FaultDisplacement => "fault_displacement",
            Self::LiquefactionOrLateralSpread => "liquefaction_or_lateral_spread",
            Self::PortalOrSlopeFailure => "portal_or_slope_failure",
            Self::WaterIngress => "water_ingress",
            Self::UtilityCommonCause => "utility_common_cause",
            Self::AftershockSequence => "aftershock_sequence",
            Self::CombinedScenario => "combined_scenario",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecoverySample {
    /// Time since the event, in hours.
    pub time_hours: f64,
    /// Demand rate in the scenario's declared service basis per hour.
    pub demand_units_per_hour: f64,
    /// Fraction of that demand delivered safely, in [0, 1].
    ///
    /// This is an externally supplied model result or observation, not computed here.
    pub safe_delivered_fraction: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryGates {
    /// Re-entry approval from the responsible authority/owner.
    pub post_event_inspection: GateState,
    /// Safe emergency egress remains available for the assessed service.
    pub emergency_egress: GateState,
    /// Critical route, power, control, drainage, ventilation, and other dependencies
    /// have been assessed for the declared scenario. This flag is not a dependency solver.
    pub critical_dependencies_assessed: GateState,
}

impl RecoveryGates {
    pub fn all_satisfied() -> Self {
        Self {
            post_event_inspection: GateState::Satisfied,
            emergency_egress: GateState::Satisfied,
            critical_dependencies_assessed: GateState::Satisfied,
        }
    }

    fn states(self) -> [(&'static str, GateState); 3] {
        [
            ("post_event_inspection", self.post_event_inspection),
            ("emergency_egress", self.emergency_egress),
            (
                "critical_dependencies_assessed",
                self.critical_dependencies_assessed,
            ),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryScenario {
    pub scenario_id: String,
    /// Service contract identifier; scenarios with different contracts are incomparable.
    pub comparison_basis_id: String,
    pub event_class: RecoveryEventClass,
    pub service_basis: ServiceBasis,
    pub evidence_class: EvidenceClass,
    /// Provenance identifiers supplied by the caller; not authenticated by this module.
    pub source_refs: Vec<String>,
    /// Samples must start at hour zero and be strictly increasing in time.
    /// The final sample time defines the report horizon.
    pub samples: Vec<RecoverySample>,
    pub gates: RecoveryGates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDisposition {
    InvalidInput,
    Ineligible,
    NeedsEvidence,
    ComparableSynthetic,
    ComparableDeclaredEvidence,
}

impl RecoveryDisposition {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::Ineligible => "INELIGIBLE",
            Self::NeedsEvidence => "NEEDS_EVIDENCE",
            Self::ComparableSynthetic => "COMPARABLE_SYNTHETIC_ONLY",
            Self::ComparableDeclaredEvidence => "COMPARABLE_DECLARED_EVIDENCE",
        }
    }

    fn is_comparable(self) -> bool {
        matches!(
            self,
            Self::ComparableSynthetic | Self::ComparableDeclaredEvidence
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryMetrics {
    pub scenario_id: String,
    pub comparison_basis_id: String,
    pub event_class: RecoveryEventClass,
    pub service_basis: ServiceBasis,
    pub evidence_class: EvidenceClass,
    pub horizon_hours: f64,
    pub demanded_service_units: f64,
    pub delivered_service_units: f64,
    pub cumulative_unmet_service_units: f64,
    pub delivered_fraction_over_horizon: f64,
    /// First time a threshold is reached and remains reached for all later samples.
    /// None means it was not sustained within the declared horizon (right-censored).
    pub t50_hours: Option<f64>,
    pub t90_hours: Option<f64>,
    pub t95_hours: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryEvaluation {
    pub scenario_id: String,
    pub disposition: RecoveryDisposition,
    pub reasons: Vec<String>,
    pub metrics: Option<RecoveryMetrics>,
}

impl RecoveryEvaluation {
    pub fn is_comparable(&self) -> bool {
        self.disposition.is_comparable() && self.metrics.is_some()
    }
}

fn invalid(scenario_id: &str, reason: impl Into<String>) -> RecoveryEvaluation {
    RecoveryEvaluation {
        scenario_id: scenario_id.to_owned(),
        disposition: RecoveryDisposition::InvalidInput,
        reasons: vec![reason.into()],
        metrics: None,
    }
}

fn validate(scenario: &RecoveryScenario) -> Result<(), String> {
    if scenario.scenario_id.trim().is_empty() || scenario.comparison_basis_id.trim().is_empty() {
        return Err("EMPTY_SCENARIO_OR_COMPARISON_ID".to_owned());
    }
    if scenario.samples.len() < 2 {
        return Err("AT_LEAST_TWO_RECOVERY_SAMPLES_REQUIRED".to_owned());
    }
    if scenario.samples[0].time_hours.abs() > f64::EPSILON {
        return Err("RECOVERY_CURVE_MUST_START_AT_HOUR_ZERO".to_owned());
    }

    let mut previous_time = -1.0;
    for sample in &scenario.samples {
        if !sample.time_hours.is_finite() || sample.time_hours < 0.0 {
            return Err("INVALID_SAMPLE_TIME".to_owned());
        }
        if sample.time_hours <= previous_time {
            return Err("SAMPLE_TIMES_MUST_BE_STRICTLY_INCREASING".to_owned());
        }
        if !sample.demand_units_per_hour.is_finite() || sample.demand_units_per_hour <= 0.0 {
            return Err("DEMAND_RATE_MUST_BE_FINITE_AND_POSITIVE".to_owned());
        }
        if !sample.safe_delivered_fraction.is_finite()
            || !(0.0..=1.0).contains(&sample.safe_delivered_fraction)
        {
            return Err("SAFE_DELIVERED_FRACTION_OUT_OF_RANGE".to_owned());
        }
        previous_time = sample.time_hours;
    }

    if scenario.evidence_class != EvidenceClass::Synthetic && scenario.source_refs.is_empty() {
        return Err("DECLARED_EVIDENCE_REQUIRES_SOURCE_REFERENCES".to_owned());
    }
    let mut refs = BTreeSet::new();
    for source_ref in &scenario.source_refs {
        if source_ref.trim().is_empty() || !refs.insert(source_ref) {
            return Err("EMPTY_OR_DUPLICATE_SOURCE_REFERENCE".to_owned());
        }
    }
    Ok(())
}

fn sustainable_crossing(samples: &[RecoverySample], threshold: f64) -> Option<f64> {
    let stays_above = |from: usize| {
        samples[from..]
            .iter()
            .all(|sample| sample.safe_delivered_fraction >= threshold)
    };

    if samples[0].safe_delivered_fraction >= threshold && stays_above(0) {
        return Some(0.0);
    }

    for index in 1..samples.len() {
        let before = samples[index - 1];
        let after = samples[index];
        if before.safe_delivered_fraction < threshold
            && after.safe_delivered_fraction >= threshold
            && stays_above(index)
        {
            let fraction = (threshold - before.safe_delivered_fraction)
                / (after.safe_delivered_fraction - before.safe_delivered_fraction);
            return Some(before.time_hours + fraction * (after.time_hours - before.time_hours));
        }
    }
    None
}

/// Evaluate a supplied, demand-normalized service curve.
///
/// Demand rate and unmet-service rate are linearly interpolated between samples and
/// integrated with the trapezoidal rule. Threshold times are returned only when the
/// threshold remains satisfied for every later sample, so later aftershock-related
/// losses can invalidate an earlier apparent recovery. All times are hours.
pub fn evaluate_recovery(scenario: &RecoveryScenario) -> RecoveryEvaluation {
    if let Err(reason) = validate(scenario) {
        return invalid(&scenario.scenario_id, reason);
    }

    let mut failed = Vec::new();
    let mut unresolved = Vec::new();
    for (name, state) in scenario.gates.states() {
        match state {
            GateState::Satisfied => {}
            GateState::Failed => failed.push(format!("GATE_FAILED:{name}")),
            GateState::Unknown | GateState::NotApplicable => {
                unresolved.push(format!("NEEDS_EVIDENCE:{name}"));
            }
        }
    }
    if !failed.is_empty() {
        failed.extend(unresolved);
        return RecoveryEvaluation {
            scenario_id: scenario.scenario_id.clone(),
            disposition: RecoveryDisposition::Ineligible,
            reasons: failed,
            metrics: None,
        };
    }
    if !unresolved.is_empty() {
        return RecoveryEvaluation {
            scenario_id: scenario.scenario_id.clone(),
            disposition: RecoveryDisposition::NeedsEvidence,
            reasons: unresolved,
            metrics: None,
        };
    }

    let mut demanded = 0.0;
    let mut unmet = 0.0;
    for pair in scenario.samples.windows(2) {
        let before = pair[0];
        let after = pair[1];
        let duration = after.time_hours - before.time_hours;
        let unmet_before = before.demand_units_per_hour * (1.0 - before.safe_delivered_fraction);
        let unmet_after = after.demand_units_per_hour * (1.0 - after.safe_delivered_fraction);
        demanded += (before.demand_units_per_hour + after.demand_units_per_hour) * 0.5 * duration;
        unmet += (unmet_before + unmet_after) * 0.5 * duration;
    }
    let delivered = demanded - unmet;
    let delivered_fraction = delivered / demanded;
    let horizon_hours = scenario
        .samples
        .last()
        .expect("validated nonempty curve")
        .time_hours;
    let numeric_outputs = [
        demanded,
        unmet,
        delivered,
        delivered_fraction,
        horizon_hours,
    ];
    if !numeric_outputs.iter().all(|value| value.is_finite())
        || demanded <= 0.0
        || unmet < 0.0
        || delivered < -1.0e-9
        || delivered_fraction < -1.0e-9
        || delivered_fraction > 1.0 + 1.0e-9
    {
        return invalid(
            &scenario.scenario_id,
            "INVALID_OR_OVERFLOWED_RECOVERY_METRICS",
        );
    }

    let metrics = RecoveryMetrics {
        scenario_id: scenario.scenario_id.clone(),
        comparison_basis_id: scenario.comparison_basis_id.clone(),
        event_class: scenario.event_class,
        service_basis: scenario.service_basis,
        evidence_class: scenario.evidence_class,
        horizon_hours,
        demanded_service_units: demanded,
        delivered_service_units: delivered.max(0.0),
        cumulative_unmet_service_units: unmet,
        delivered_fraction_over_horizon: delivered_fraction.clamp(0.0, 1.0),
        t50_hours: sustainable_crossing(&scenario.samples, 0.50),
        t90_hours: sustainable_crossing(&scenario.samples, 0.90),
        t95_hours: sustainable_crossing(&scenario.samples, 0.95),
    };
    let disposition = match scenario.evidence_class {
        EvidenceClass::Synthetic => RecoveryDisposition::ComparableSynthetic,
        EvidenceClass::Sourced | EvidenceClass::Measured | EvidenceClass::IndependentlyReviewed => {
            RecoveryDisposition::ComparableDeclaredEvidence
        }
    };
    RecoveryEvaluation {
        scenario_id: scenario.scenario_id.clone(),
        disposition,
        reasons: Vec::new(),
        metrics: Some(metrics),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_scenario() -> RecoveryScenario {
        RecoveryScenario {
            scenario_id: "fixture".to_owned(),
            comparison_basis_id: "SYNTHETIC-100-UNITS-PER-HOUR-V1".to_owned(),
            event_class: RecoveryEventClass::AftershockSequence,
            service_basis: ServiceBasis::PassengerTrip,
            evidence_class: EvidenceClass::Synthetic,
            source_refs: Vec::new(),
            samples: vec![
                RecoverySample {
                    time_hours: 0.0,
                    demand_units_per_hour: 100.0,
                    safe_delivered_fraction: 0.0,
                },
                RecoverySample {
                    time_hours: 10.0,
                    demand_units_per_hour: 100.0,
                    safe_delivered_fraction: 0.5,
                },
                RecoverySample {
                    time_hours: 20.0,
                    demand_units_per_hour: 100.0,
                    safe_delivered_fraction: 0.9,
                },
                RecoverySample {
                    time_hours: 30.0,
                    demand_units_per_hour: 100.0,
                    safe_delivered_fraction: 0.95,
                },
            ],
            gates: RecoveryGates::all_satisfied(),
        }
    }

    #[test]
    fn hand_calculated_curve_reports_thresholds_and_unmet_service() {
        let evaluation = evaluate_recovery(&base_scenario());
        assert_eq!(
            evaluation.disposition,
            RecoveryDisposition::ComparableSynthetic
        );
        let metrics = evaluation.metrics.unwrap();
        assert!((metrics.t50_hours.unwrap() - 10.0).abs() < 1.0e-9);
        assert!((metrics.t90_hours.unwrap() - 20.0).abs() < 1.0e-9);
        assert!((metrics.t95_hours.unwrap() - 30.0).abs() < 1.0e-9);
        assert!((metrics.demanded_service_units - 3_000.0).abs() < 1.0e-9);
        assert!((metrics.cumulative_unmet_service_units - 1_125.0).abs() < 1.0e-9);
        assert!((metrics.delivered_service_units - 1_875.0).abs() < 1.0e-9);
        assert!((metrics.delivered_fraction_over_horizon - 0.625).abs() < 1.0e-9);
    }

    #[test]
    fn later_aftershock_loss_invalidates_earlier_crossing() {
        let mut scenario = base_scenario();
        scenario.samples = vec![
            RecoverySample {
                time_hours: 0.0,
                demand_units_per_hour: 100.0,
                safe_delivered_fraction: 0.0,
            },
            RecoverySample {
                time_hours: 10.0,
                demand_units_per_hour: 100.0,
                safe_delivered_fraction: 0.8,
            },
            RecoverySample {
                time_hours: 20.0,
                demand_units_per_hour: 100.0,
                safe_delivered_fraction: 0.3,
            },
            RecoverySample {
                time_hours: 30.0,
                demand_units_per_hour: 100.0,
                safe_delivered_fraction: 0.9,
            },
            RecoverySample {
                time_hours: 40.0,
                demand_units_per_hour: 100.0,
                safe_delivered_fraction: 0.95,
            },
        ];
        let metrics = evaluate_recovery(&scenario).metrics.unwrap();
        assert!((metrics.t50_hours.unwrap() - (20.0 + 10.0 / 3.0)).abs() < 1.0e-9);
        assert!((metrics.t90_hours.unwrap() - 30.0).abs() < 1.0e-9);
        assert!((metrics.t95_hours.unwrap() - 40.0).abs() < 1.0e-9);
    }

    #[test]
    fn thresholds_not_sustained_by_horizon_are_right_censored() {
        let mut scenario = base_scenario();
        scenario.samples.truncate(3);
        scenario.samples[2].safe_delivered_fraction = 0.7;
        let metrics = evaluate_recovery(&scenario).metrics.unwrap();
        assert!(metrics.t50_hours.is_some());
        assert!(metrics.t90_hours.is_none());
        assert!(metrics.t95_hours.is_none());
    }

    #[test]
    fn unresolved_reentry_gate_blocks_all_metrics() {
        let mut scenario = base_scenario();
        scenario.gates.post_event_inspection = GateState::Unknown;
        let evaluation = evaluate_recovery(&scenario);
        assert_eq!(evaluation.disposition, RecoveryDisposition::NeedsEvidence);
        assert!(evaluation.metrics.is_none());
    }

    #[test]
    fn failed_egress_gate_blocks_all_metrics() {
        let mut scenario = base_scenario();
        scenario.gates.emergency_egress = GateState::Failed;
        let evaluation = evaluate_recovery(&scenario);
        assert_eq!(evaluation.disposition, RecoveryDisposition::Ineligible);
        assert!(evaluation.metrics.is_none());
    }

    #[test]
    fn declared_evidence_requires_source_references() {
        let mut scenario = base_scenario();
        scenario.evidence_class = EvidenceClass::Measured;
        assert_eq!(
            evaluate_recovery(&scenario).disposition,
            RecoveryDisposition::InvalidInput
        );
        scenario.source_refs = vec!["measurement-campaign-001".to_owned()];
        assert_eq!(
            evaluate_recovery(&scenario).disposition,
            RecoveryDisposition::ComparableDeclaredEvidence
        );
    }

    #[test]
    fn bad_curve_inputs_are_rejected() {
        let mut scenario = base_scenario();
        scenario.samples[1].time_hours = scenario.samples[0].time_hours;
        assert_eq!(
            evaluate_recovery(&scenario).disposition,
            RecoveryDisposition::InvalidInput
        );

        let mut out_of_range = base_scenario();
        out_of_range.samples[1].safe_delivered_fraction = 1.1;
        assert_eq!(
            evaluate_recovery(&out_of_range).disposition,
            RecoveryDisposition::InvalidInput
        );
    }
    #[test]
    fn thirty_two_synthetic_curves_preserve_service_accounting_invariants() {
        let event_classes = [
            RecoveryEventClass::ShakingOnly,
            RecoveryEventClass::FaultDisplacement,
            RecoveryEventClass::LiquefactionOrLateralSpread,
            RecoveryEventClass::PortalOrSlopeFailure,
            RecoveryEventClass::WaterIngress,
            RecoveryEventClass::UtilityCommonCause,
            RecoveryEventClass::AftershockSequence,
            RecoveryEventClass::CombinedScenario,
        ];

        // This is a contract stress matrix: it exercises all eight event labels
        // against four supplied curve shapes. It is not a physical hazard simulation.
        for (event_index, event_class) in event_classes.into_iter().enumerate() {
            for curve_kind in 0..4 {
                let mut scenario = base_scenario();
                scenario.scenario_id = format!("matrix-{event_index}-{curve_kind}");
                scenario.event_class = event_class;
                scenario.service_basis = if event_index % 2 == 0 {
                    ServiceBasis::PassengerTrip
                } else {
                    ServiceBasis::FreightTonne
                };
                let demand = 80.0 + (event_index * 7 + curve_kind) as f64;
                let curve: &[(f64, f64)] = match curve_kind {
                    0 => &[(0.0, 0.0), (5.0, 0.25), (15.0, 0.75), (30.0, 0.95)],
                    1 => &[
                        (0.0, 0.1),
                        (5.0, 0.8),
                        (10.0, 0.3),
                        (25.0, 0.9),
                        (36.0, 0.95),
                    ],
                    2 => &[(0.0, 0.0), (8.0, 0.2), (24.0, 0.4)],
                    _ => &[(0.0, 1.0), (1.0, 1.0), (10.0, 1.0)],
                };
                scenario.samples = curve
                    .iter()
                    .map(|(time_hours, safe_fraction)| RecoverySample {
                        time_hours: *time_hours,
                        demand_units_per_hour: demand,
                        safe_delivered_fraction: *safe_fraction,
                    })
                    .collect();

                let evaluation = evaluate_recovery(&scenario);
                assert_eq!(
                    evaluation.disposition,
                    RecoveryDisposition::ComparableSynthetic,
                    "{}",
                    scenario.scenario_id
                );
                let metrics = evaluation.metrics.unwrap();
                let expected_demand = demand * metrics.horizon_hours;
                assert!(
                    (metrics.demanded_service_units - expected_demand).abs() < 1.0e-8,
                    "{}",
                    scenario.scenario_id
                );
                assert!(
                    (metrics.delivered_service_units + metrics.cumulative_unmet_service_units
                        - metrics.demanded_service_units)
                        .abs()
                        < 1.0e-8,
                    "{}",
                    scenario.scenario_id
                );
                assert!((0.0..=1.0).contains(&metrics.delivered_fraction_over_horizon));
                if let (Some(t50), Some(t90)) = (metrics.t50_hours, metrics.t90_hours) {
                    assert!(t50 <= t90, "{}", scenario.scenario_id);
                }
                if let (Some(t90), Some(t95)) = (metrics.t90_hours, metrics.t95_hours) {
                    assert!(t90 <= t95, "{}", scenario.scenario_id);
                }
                if curve_kind == 2 {
                    assert!(metrics.t50_hours.is_none());
                    assert!(metrics.t90_hours.is_none());
                    assert!(metrics.t95_hours.is_none());
                }
                if curve_kind == 3 {
                    assert_eq!(metrics.t50_hours, Some(0.0));
                    assert_eq!(metrics.t90_hours, Some(0.0));
                    assert_eq!(metrics.t95_hours, Some(0.0));
                }
            }
        }
    }
}
