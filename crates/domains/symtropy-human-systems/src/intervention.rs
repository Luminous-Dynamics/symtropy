// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed, synthetic intervention event-chain fixtures.
//!
//! This module deliberately stops before causal effect estimation. It makes
//! assignment, delivery, physical exposure and sensor observation separate
//! records so that later integration with the shared Symthaea causal DGP
//! cannot silently equate one stage with another.
//!
//! All values and units are arbitrary software-test fixtures. They are not
//! calibrated human exposure levels, physiological parameters or clinical data.

/// Canonical non-clinical intervention event-chain scenarios.
pub const SCENARIO_COUNT: usize = 5;

/// Stable scenario identities for the first intervention-event fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterventionScenarioId {
    /// Action is delivered, exposure occurs and the observation arrives promptly.
    DeliveredAndMeasured,
    /// Assignment is made, but delivery fails.
    AssignedButNotDelivered,
    /// Delivery succeeds, but the synthetic environment blocks exposure.
    DeliveredButExposureBlocked,
    /// Exposure occurs, but the exposure sensor reports a missing value.
    ExposureOccurredMeasurementMissing,
    /// Exposure occurs, but its measurement arrives after the decision cutoff.
    ExposureOccurredMeasurementDelayed,
}

impl InterventionScenarioId {
    /// Canonical stable ordering.
    pub const ALL: [Self; SCENARIO_COUNT] = [
        Self::DeliveredAndMeasured,
        Self::AssignedButNotDelivered,
        Self::DeliveredButExposureBlocked,
        Self::ExposureOccurredMeasurementMissing,
        Self::ExposureOccurredMeasurementDelayed,
    ];

    /// Stable machine-readable ID.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeliveredAndMeasured => "HIE-00",
            Self::AssignedButNotDelivered => "HIE-01",
            Self::DeliveredButExposureBlocked => "HIE-02",
            Self::ExposureOccurredMeasurementMissing => "HIE-03",
            Self::ExposureOccurredMeasurementDelayed => "HIE-04",
        }
    }
}

/// Non-clinical action category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignedActionKind {
    /// Offer or schedule a rest opportunity.
    RestOpportunity,
    /// Change the order or demand of a synthetic task.
    TaskDemandChange,
    /// Command an environmental change; not a photobiological dose.
    EnvironmentalCommand,
    /// Change an activity or movement opportunity.
    ActivityOpportunity,
    /// Request a sensor measurement.
    SensorRequest,
}

/// A candidate-visible action assignment.
///
/// Assignment says what was requested or allocated. It is not evidence that
/// the action was delivered or that a physical exposure occurred.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionAssignment {
    /// Stable assignment identity.
    pub assignment_id: u64,
    /// Requested action category.
    pub action: AssignedActionKind,
    /// When the assignment was made, in simulation seconds.
    pub assigned_at_seconds: f64,
    /// Requested duration, in synthetic seconds.
    pub requested_duration_seconds: f64,
}

/// Reason a delivery attempt failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryFailureReason {
    /// The actuator or service was unavailable.
    ServiceUnavailable,
    /// An explicit authority/permission gate rejected the action.
    NotAuthorized,
    /// Required resource or capacity was unavailable.
    ResourceUnavailable,
}

/// Result of the actual delivery attempt, held in the scorer view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrueDeliveryOutcome {
    /// The assigned action was delivered to the simulated environment.
    Delivered,
    /// Delivery did not occur.
    Failed(DeliveryFailureReason),
}

/// Delivery evidence made available to the candidate.
///
/// This is a report received from a delivery subsystem. It is not proof that
/// the intended physical exposure reached the synthetic person or world.
#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryReceipt {
    /// Assignment this report concerns.
    pub assignment_id: u64,
    /// Reported delivery outcome.
    pub reported_outcome: TrueDeliveryOutcome,
    /// Time the report became available to the candidate.
    pub received_at_seconds: f64,
    /// Source identity for the reporting subsystem.
    pub reporting_source: &'static str,
}

/// Synthetic exposure channel. These values describe model inputs, not
/// measured biological quantities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticExposureChannel {
    /// A rest opportunity was taken in the synthetic world.
    RestOpportunityTaken,
    /// A task-demand command was physically applied.
    TaskDemandApplied,
    /// A generic environment command was applied.
    EnvironmentCommandApplied,
    /// An activity opportunity was physically available.
    ActivityOpportunityAvailable,
    /// A sensor acquisition was physically triggered.
    SensorAcquisitionTriggered,
}

/// Reason a delivered command did not produce the intended exposure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposureBlockReason {
    /// The synthetic person/actor was not present.
    ActorAbsent,
    /// A geometry or occlusion constraint blocked the command.
    GeometryBlocked,
    /// The environment rejected the command after delivery.
    EnvironmentRejected,
}

/// Scorer-only record of whether a physical exposure occurred.
///
/// Magnitude is arbitrary and intentionally carries no medical or
/// physiological interpretation.
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicalExposureTruth {
    /// A physical exposure/action occurred in the synthetic world.
    Occurred {
        /// Stable exposure identity.
        exposure_id: u64,
        /// Synthetic exposure channel.
        channel: SyntheticExposureChannel,
        /// Physical start time in simulation seconds.
        started_at_seconds: f64,
        /// Actual duration in synthetic seconds.
        duration_seconds: f64,
        /// Arbitrary fixture magnitude.
        magnitude: f64,
        /// Explicitly arbitrary unit.
        unit: &'static str,
    },
    /// The command was delivered but produced no physical exposure.
    Blocked(ExposureBlockReason),
    /// Delivery failed, so no exposure was possible.
    NotDelivered,
}

/// Candidate-visible sensor reading state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposureMeasurementStatus {
    /// A value was measured.
    Observed,
    /// A capture was attempted but no value was available.
    Missing,
    /// The value was invalid under the sensor contract.
    Invalid,
}

/// An estimator-facing exposure measurement.
///
/// The record identifies a sensor event but does not expose the scorer's
/// physical-exposure identity or latent exposure magnitude.
#[derive(Debug, Clone, PartialEq)]
pub struct ExposureObservation {
    /// Opaque sensor-event identity.
    pub observation_id: u64,
    /// Assignment the sensor event is nominally associated with.
    pub assignment_id: u64,
    /// Sensor capture time.
    pub captured_at_seconds: f64,
    /// Time this measurement became available to the candidate.
    pub available_at_seconds: f64,
    /// Reported measurement status.
    pub status: ExposureMeasurementStatus,
    /// Optional arbitrary-unit sensor value.
    pub value: Option<f64>,
    /// Measurement unit.
    pub unit: &'static str,
}

/// Candidate-facing evidence available at one decision cutoff.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateInterventionView {
    /// Scenario family, not a hidden scenario seed or latent state.
    pub scenario_id: InterventionScenarioId,
    /// Decision cutoff, in simulation seconds.
    pub decision_cutoff_seconds: f64,
    /// All assignments made by the cutoff.
    pub assignments: Vec<ActionAssignment>,
    /// Delivery reports received by the cutoff.
    pub delivery_receipts: Vec<DeliveryReceipt>,
    /// Exposure observations available by the cutoff.
    pub exposure_observations: Vec<ExposureObservation>,
}

/// Scorer-only event chain for a single assignment.
///
/// No potential outcome, causal effect or human-response prediction is
/// generated by this module. This is only a truth record for event semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct OracleInterventionTrace {
    /// Original assignment.
    pub assignment: ActionAssignment,
    /// Actual delivery result and times.
    pub delivery_outcome: TrueDeliveryOutcome,
    /// When the delivery attempt occurred.
    pub attempted_at_seconds: f64,
    /// When delivery completed, if successful.
    pub completed_at_seconds: Option<f64>,
    /// True physical exposure, hidden from the candidate.
    pub physical_exposure: PhysicalExposureTruth,
}

/// Scorer-only truth view.
///
/// Keep this view out of the candidate process. Separate Rust fields do not
/// provide security isolation; confirmatory runs require a process boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct OracleInterventionView {
    /// Scenario family.
    pub scenario_id: InterventionScenarioId,
    /// Full event-chain truth.
    pub traces: Vec<OracleInterventionTrace>,
}

/// One synthetic intervention-event scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct InterventionEventScenario {
    /// Candidate-facing evidence.
    pub candidate: CandidateInterventionView,
    /// Scorer-only event truth.
    pub oracle: OracleInterventionView,
    /// Delivery receipts that may become visible after the current cutoff.
    future_delivery_receipts: Vec<DeliveryReceipt>,
    /// Exposure sensor events that may become visible after the current cutoff.
    future_exposure_observations: Vec<ExposureObservation>,
}

impl InterventionEventScenario {
    /// Reconstruct a candidate view at a new information cutoff.
    ///
    /// This method only releases receipts and measurements whose availability
    /// timestamp is at or before the supplied cutoff. It never modifies oracle
    /// truth or retroactively changes an earlier candidate snapshot.
    pub fn candidate_view_at(
        &self,
        decision_cutoff_seconds: f64,
    ) -> Result<CandidateInterventionView, InterventionEventError> {
        if !decision_cutoff_seconds.is_finite() || decision_cutoff_seconds < 0.0 {
            return Err(InterventionEventError::InvalidDecisionCutoff);
        }
        let mut delivery_receipts: Vec<_> = self
            .candidate
            .delivery_receipts
            .iter()
            .chain(self.future_delivery_receipts.iter())
            .filter(|receipt| receipt.received_at_seconds <= decision_cutoff_seconds)
            .cloned()
            .collect();
        delivery_receipts.sort_by(|left, right| {
            left.received_at_seconds.total_cmp(&right.received_at_seconds)
        });

        let mut exposure_observations: Vec<_> = self
            .candidate
            .exposure_observations
            .iter()
            .chain(self.future_exposure_observations.iter())
            .filter(|observation| {
                observation.available_at_seconds <= decision_cutoff_seconds
            })
            .cloned()
            .collect();
        exposure_observations.sort_by(|left, right| {
            left.available_at_seconds
                .total_cmp(&right.available_at_seconds)
        });

        Ok(CandidateInterventionView {
            scenario_id: self.candidate.scenario_id,
            decision_cutoff_seconds,
            assignments: self
                .candidate
                .assignments
                .iter()
                .filter(|assignment| assignment.assigned_at_seconds <= decision_cutoff_seconds)
                .cloned()
                .collect(),
            delivery_receipts,
            exposure_observations,
        })
    }
}

/// Fail-closed errors for the event-chain fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionEventError {
    /// Candidate decision time was negative or non-finite.
    InvalidDecisionCutoff,
}

impl std::fmt::Display for InterventionEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDecisionCutoff => {
                write!(formatter, "decision cutoff must be finite and non-negative")
            }
        }
    }
}

impl std::error::Error for InterventionEventError {}

/// Generate one deterministic, arbitrary-unit intervention event-chain fixture.
///
/// The initial_cutoff_seconds determines what the candidate initially sees.
/// The returned scenario also permits later views through candidate_view_at.
pub fn generate_intervention_scenario(
    scenario_id: InterventionScenarioId,
    initial_cutoff_seconds: f64,
) -> Result<InterventionEventScenario, InterventionEventError> {
    if !initial_cutoff_seconds.is_finite() || initial_cutoff_seconds < 0.0 {
        return Err(InterventionEventError::InvalidDecisionCutoff);
    }

    let assignment = ActionAssignment {
        assignment_id: 100,
        action: match scenario_id {
            InterventionScenarioId::DeliveredAndMeasured
            | InterventionScenarioId::AssignedButNotDelivered => {
                AssignedActionKind::RestOpportunity
            }
            InterventionScenarioId::DeliveredButExposureBlocked
            | InterventionScenarioId::ExposureOccurredMeasurementMissing
            | InterventionScenarioId::ExposureOccurredMeasurementDelayed => {
                AssignedActionKind::EnvironmentalCommand
            }
        },
        assigned_at_seconds: 0.0,
        requested_duration_seconds: 1_800.0,
    };

    let (delivery_outcome, completed_at_seconds, physical_exposure, report, observation) =
        match scenario_id {
            InterventionScenarioId::DeliveredAndMeasured => (
                TrueDeliveryOutcome::Delivered,
                Some(5.0),
                PhysicalExposureTruth::Occurred {
                    exposure_id: 500,
                    channel: SyntheticExposureChannel::RestOpportunityTaken,
                    started_at_seconds: 10.0,
                    duration_seconds: 900.0,
                    magnitude: 1.0,
                    unit: "arbitrary_unit",
                },
                Some(DeliveryReceipt {
                    assignment_id: assignment.assignment_id,
                    reported_outcome: TrueDeliveryOutcome::Delivered,
                    received_at_seconds: 8.0,
                    reporting_source: "synthetic_delivery_controller",
                }),
                Some(ExposureObservation {
                    observation_id: 900,
                    assignment_id: assignment.assignment_id,
                    captured_at_seconds: 15.0,
                    available_at_seconds: 30.0,
                    status: ExposureMeasurementStatus::Observed,
                    value: Some(1.0),
                    unit: "arbitrary_unit",
                }),
            ),
            InterventionScenarioId::AssignedButNotDelivered => (
                TrueDeliveryOutcome::Failed(DeliveryFailureReason::ServiceUnavailable),
                None,
                PhysicalExposureTruth::NotDelivered,
                Some(DeliveryReceipt {
                    assignment_id: assignment.assignment_id,
                    reported_outcome: TrueDeliveryOutcome::Failed(
                        DeliveryFailureReason::ServiceUnavailable,
                    ),
                    received_at_seconds: 15.0,
                    reporting_source: "synthetic_delivery_controller",
                }),
                None,
            ),
            InterventionScenarioId::DeliveredButExposureBlocked => (
                TrueDeliveryOutcome::Delivered,
                Some(5.0),
                PhysicalExposureTruth::Blocked(ExposureBlockReason::GeometryBlocked),
                Some(DeliveryReceipt {
                    assignment_id: assignment.assignment_id,
                    reported_outcome: TrueDeliveryOutcome::Delivered,
                    received_at_seconds: 8.0,
                    reporting_source: "synthetic_delivery_controller",
                }),
                Some(ExposureObservation {
                    observation_id: 901,
                    assignment_id: assignment.assignment_id,
                    captured_at_seconds: 20.0,
                    available_at_seconds: 25.0,
                    status: ExposureMeasurementStatus::Observed,
                    value: Some(0.0),
                    unit: "arbitrary_unit",
                }),
            ),
            InterventionScenarioId::ExposureOccurredMeasurementMissing => (
                TrueDeliveryOutcome::Delivered,
                Some(5.0),
                PhysicalExposureTruth::Occurred {
                    exposure_id: 502,
                    channel: SyntheticExposureChannel::EnvironmentCommandApplied,
                    started_at_seconds: 12.0,
                    duration_seconds: 300.0,
                    magnitude: 0.75,
                    unit: "arbitrary_unit",
                },
                Some(DeliveryReceipt {
                    assignment_id: assignment.assignment_id,
                    reported_outcome: TrueDeliveryOutcome::Delivered,
                    received_at_seconds: 8.0,
                    reporting_source: "synthetic_delivery_controller",
                }),
                Some(ExposureObservation {
                    observation_id: 902,
                    assignment_id: assignment.assignment_id,
                    captured_at_seconds: 18.0,
                    available_at_seconds: 40.0,
                    status: ExposureMeasurementStatus::Missing,
                    value: None,
                    unit: "arbitrary_unit",
                }),
            ),
            InterventionScenarioId::ExposureOccurredMeasurementDelayed => (
                TrueDeliveryOutcome::Delivered,
                Some(5.0),
                PhysicalExposureTruth::Occurred {
                    exposure_id: 503,
                    channel: SyntheticExposureChannel::EnvironmentCommandApplied,
                    started_at_seconds: 12.0,
                    duration_seconds: 300.0,
                    magnitude: 0.75,
                    unit: "arbitrary_unit",
                },
                Some(DeliveryReceipt {
                    assignment_id: assignment.assignment_id,
                    reported_outcome: TrueDeliveryOutcome::Delivered,
                    received_at_seconds: 8.0,
                    reporting_source: "synthetic_delivery_controller",
                }),
                Some(ExposureObservation {
                    observation_id: 903,
                    assignment_id: assignment.assignment_id,
                    captured_at_seconds: 18.0,
                    available_at_seconds: 90.0,
                    status: ExposureMeasurementStatus::Observed,
                    value: Some(0.75),
                    unit: "arbitrary_unit",
                }),
            ),
        };

    let trace = OracleInterventionTrace {
        assignment: assignment.clone(),
        delivery_outcome,
        attempted_at_seconds: 2.0,
        completed_at_seconds,
        physical_exposure,
    };

    let future_delivery_receipts: Vec<_> = report
        .iter()
        .filter(|receipt| receipt.received_at_seconds > initial_cutoff_seconds)
        .cloned()
        .collect();
    let visible_delivery_receipts: Vec<_> = report
        .iter()
        .filter(|receipt| receipt.received_at_seconds <= initial_cutoff_seconds)
        .cloned()
        .collect();
    let future_exposure_observations: Vec<_> = observation
        .iter()
        .filter(|item| item.available_at_seconds > initial_cutoff_seconds)
        .cloned()
        .collect();
    let visible_exposure_observations: Vec<_> = observation
        .iter()
        .filter(|item| item.available_at_seconds <= initial_cutoff_seconds)
        .cloned()
        .collect();

    Ok(InterventionEventScenario {
        candidate: CandidateInterventionView {
            scenario_id,
            decision_cutoff_seconds: initial_cutoff_seconds,
            assignments: if assignment.assigned_at_seconds <= initial_cutoff_seconds {
                vec![assignment]
            } else {
                Vec::new()
            },
            delivery_receipts: visible_delivery_receipts,
            exposure_observations: visible_exposure_observations,
        },
        oracle: OracleInterventionView {
            scenario_id,
            traces: vec![trace],
        },
        future_delivery_receipts,
        future_exposure_observations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate(scenario_id: InterventionScenarioId, cutoff: f64) -> InterventionEventScenario {
        generate_intervention_scenario(scenario_id, cutoff).unwrap()
    }

    #[test]
    fn scenario_ids_are_unique_and_stable() {
        let ids = InterventionScenarioId::ALL.map(InterventionScenarioId::as_str);
        let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
        assert_eq!(ids.len(), SCENARIO_COUNT);
        assert_eq!(unique.len(), SCENARIO_COUNT);
        assert_eq!(ids[0], "HIE-00");
        assert_eq!(ids[4], "HIE-04");
    }

    #[test]
    fn assignment_is_not_delivery() {
        let scenario = generate(InterventionScenarioId::AssignedButNotDelivered, 60.0);
        assert_eq!(scenario.candidate.assignments.len(), 1);
        assert_eq!(
            scenario.oracle.traces[0].delivery_outcome,
            TrueDeliveryOutcome::Failed(DeliveryFailureReason::ServiceUnavailable)
        );
        assert_eq!(
            scenario.oracle.traces[0].physical_exposure,
            PhysicalExposureTruth::NotDelivered
        );
        assert_eq!(
            scenario.candidate.delivery_receipts[0].reported_outcome,
            TrueDeliveryOutcome::Failed(DeliveryFailureReason::ServiceUnavailable)
        );
        assert!(scenario.candidate.exposure_observations.is_empty());
    }

    #[test]
    fn delivery_is_not_physical_exposure() {
        let scenario = generate(InterventionScenarioId::DeliveredButExposureBlocked, 60.0);
        assert_eq!(
            scenario.oracle.traces[0].delivery_outcome,
            TrueDeliveryOutcome::Delivered
        );
        assert_eq!(
            scenario.oracle.traces[0].physical_exposure,
            PhysicalExposureTruth::Blocked(ExposureBlockReason::GeometryBlocked)
        );
        assert_eq!(
            scenario.candidate.delivery_receipts[0].reported_outcome,
            TrueDeliveryOutcome::Delivered
        );
        assert_eq!(
            scenario.candidate.exposure_observations[0].value,
            Some(0.0)
        );
    }

    #[test]
    fn exposure_can_occur_when_its_measurement_is_missing() {
        let scenario = generate(
            InterventionScenarioId::ExposureOccurredMeasurementMissing,
            60.0,
        );
        assert!(matches!(
            scenario.oracle.traces[0].physical_exposure,
            PhysicalExposureTruth::Occurred { .. }
        ));
        let observation = &scenario.candidate.exposure_observations[0];
        assert_eq!(observation.status, ExposureMeasurementStatus::Missing);
        assert_eq!(observation.value, None);
    }

    #[test]
    fn delayed_measurement_is_hidden_until_its_availability_time() {
        let scenario = generate(
            InterventionScenarioId::ExposureOccurredMeasurementDelayed,
            60.0,
        );
        assert!(matches!(
            scenario.oracle.traces[0].physical_exposure,
            PhysicalExposureTruth::Occurred { .. }
        ));
        assert!(scenario.candidate.exposure_observations.is_empty());

        let later = scenario.candidate_view_at(100.0).unwrap();
        assert_eq!(later.exposure_observations.len(), 1);
        assert_eq!(later.exposure_observations[0].available_at_seconds, 90.0);
    }

    #[test]
    fn later_receipt_does_not_rewrite_earlier_information_snapshot() {
        let scenario = generate(InterventionScenarioId::AssignedButNotDelivered, 10.0);
        assert!(scenario.candidate.delivery_receipts.is_empty());
        let later = scenario.candidate_view_at(20.0).unwrap();
        assert_eq!(later.delivery_receipts.len(), 1);
        assert!(scenario.candidate.delivery_receipts.is_empty());
    }

    #[test]
    fn delivered_measured_scenario_has_distinct_event_times() {
        let scenario = generate(InterventionScenarioId::DeliveredAndMeasured, 60.0);
        let trace = &scenario.oracle.traces[0];
        let observation = &scenario.candidate.exposure_observations[0];
        assert_eq!(trace.assignment.assigned_at_seconds, 0.0);
        assert_eq!(trace.attempted_at_seconds, 2.0);
        assert_eq!(trace.completed_at_seconds, Some(5.0));
        assert_eq!(observation.captured_at_seconds, 15.0);
        assert_eq!(observation.available_at_seconds, 30.0);
    }

    #[test]
    fn fixed_scenario_replays_deterministically() {
        for scenario_id in InterventionScenarioId::ALL {
            assert_eq!(
                generate_intervention_scenario(scenario_id, 50.0),
                generate_intervention_scenario(scenario_id, 50.0)
            );
        }
    }

    #[test]
    fn invalid_cutoffs_fail_before_a_candidate_view_is_returned() {
        assert_eq!(
            generate_intervention_scenario(InterventionScenarioId::DeliveredAndMeasured, f64::NAN),
            Err(InterventionEventError::InvalidDecisionCutoff)
        );
        assert_eq!(
            generate_intervention_scenario(InterventionScenarioId::DeliveredAndMeasured, -1.0),
            Err(InterventionEventError::InvalidDecisionCutoff)
        );
    }

    #[test]
    fn all_candidate_timestamps_are_finite_and_not_in_the_future_of_the_cutoff() {
        for scenario_id in InterventionScenarioId::ALL {
            let scenario = generate(scenario_id, 60.0);
            assert!(scenario.candidate.assignments.iter().all(|item| {
                item.assigned_at_seconds.is_finite()
                    && item.assigned_at_seconds <= scenario.candidate.decision_cutoff_seconds
            }));
            assert!(scenario.candidate.delivery_receipts.iter().all(|item| {
                item.received_at_seconds.is_finite()
                    && item.received_at_seconds <= scenario.candidate.decision_cutoff_seconds
            }));
            assert!(scenario.candidate.exposure_observations.iter().all(|item| {
                item.available_at_seconds.is_finite()
                    && item.available_at_seconds <= scenario.candidate.decision_cutoff_seconds
            }));
        }
    }
}
