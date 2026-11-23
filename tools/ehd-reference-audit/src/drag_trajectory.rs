// SPDX-License-Identifier: AGPL-3.0-or-later

//! Space-time quadrature receipt for time-varying ion-neutral drag.
//!
//! Each sample carries the non-negative integrated drag measure
//!
//!     dGamma_q = quadrature_weight * (integral_cell m_s*n_s*nu_sn dV) * dt
//!
//! in kg, together with its time fraction and local relative drift. The same
//! samples and dGamma_q values generate both:
//!
//!     I_neutral = sum_q dGamma_q * (u_charged - u_neutral)_q
//!     Q_friction = sum_q dGamma_q * |u_charged - u_neutral|_q^2 >= 0.
//!
//! Thus impulse and dissipative heating use the same space-time quadrature.
//! This module validates the resulting receipt; it does not choose a quadrature
//! rule, estimate quadrature error, solve collision kinetics, or partition
//! thermal energy between species.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// One space-time quadrature contribution. The integrated drag weight already
/// includes the positive quadrature weight, spatial measure, time measure, and
/// local m*n*nu coefficient. It is therefore non-negative and has units kg.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragQuadratureSample {
    pub sample_id: u64,
    /// Normalized time coordinate in the event interval [0, 1].
    pub time_fraction: f64,
    /// Positive quadrature-weighted integral of m*n*nu over space-time [kg].
    pub integrated_drag_weight_kg: f64,
    /// Local charged-minus-neutral velocity at this quadrature sample [m/s].
    pub relative_velocity_m_s: [f64; 3],
}

/// A time-varying drag event with independently reported momentum and heat.
#[derive(Clone, Debug, PartialEq)]
pub struct DragTrajectoryEvent {
    pub event_id: u64,
    /// Identity of the temporal/spatial quadrature rule used by the solver.
    pub quadrature_rule_id: u64,
    /// Charged and neutral coordinate frames for the paired impulse vectors.
    pub charged_frame_id: u64,
    pub neutral_frame_id: u64,
    pub samples: Vec<DragQuadratureSample>,
    /// Integrated impulse delivered to neutral species [N*s].
    pub neutral_impulse_ns: [f64; 3],
    /// Integrated equal-and-opposite charged-side reaction [N*s].
    pub charged_reaction_impulse_ns: [f64; 3],
    /// Frictional heating reported by the collision/energy operator [J].
    pub reported_frictional_heating_j: f64,
}

/// Case-specific componentwise impulse and scalar energy tolerances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragTrajectoryTolerance {
    pub impulse_absolute_ns: f64,
    pub impulse_relative: f64,
    pub energy_absolute_j: f64,
    pub energy_relative: f64,
}

/// Validated time-varying momentum/energy receipt.
#[derive(Clone, Debug, PartialEq)]
pub struct DragTrajectoryReceipt {
    pub event_count: usize,
    pub sample_count: usize,
    /// Per-event quadrature rule identities retained for downstream evidence.
    pub quadrature_rule_ids: BTreeMap<u64, u64>,
    /// Shared coordinate-frame identity retained for each event.
    pub coordinate_frame_ids: BTreeMap<u64, u64>,
    pub total_integrated_drag_weight_kg: f64,
    pub total_neutral_impulse_ns: [f64; 3],
    pub total_charged_reaction_impulse_ns: [f64; 3],
    pub aggregate_impulse_residual_ns: [f64; 3],
    pub predicted_frictional_heating_j: f64,
    pub reported_frictional_heating_j: f64,
    pub aggregate_energy_residual_j: f64,
    pub max_event_impulse_residual_ns: f64,
    pub max_event_energy_residual_j: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DragTrajectoryError {
    EmptyBatch,
    EmptyEvent(u64),
    InvalidTolerance,
    DuplicateEventId(u64),
    DuplicateSampleId {
        event_id: u64,
        sample_id: u64,
    },
    CoordinateFrameMismatch {
        event_id: u64,
        charged_frame_id: u64,
        neutral_frame_id: u64,
    },
    InvalidTimeFraction {
        event_id: u64,
        sample_id: u64,
    },
    InvalidIntegratedDragWeight {
        event_id: u64,
        sample_id: u64,
    },
    NonFiniteVelocity {
        event_id: u64,
        sample_id: u64,
    },
    NonFiniteReportedValue(u64),
    NonFiniteAccumulation,
    NeutralImpulseMismatch {
        event_id: u64,
        component: usize,
        supplied_ns: f64,
        expected_ns: f64,
        allowed_ns: f64,
    },
    ChargedReactionMismatch {
        event_id: u64,
        component: usize,
        supplied_ns: f64,
        expected_ns: f64,
        allowed_ns: f64,
    },
    NegativeReportedHeating {
        event_id: u64,
        supplied_j: f64,
        allowed_negative_j: f64,
    },
    HeatingMismatch {
        event_id: u64,
        supplied_j: f64,
        expected_j: f64,
        allowed_j: f64,
    },
    AggregateImpulseMismatch {
        component: usize,
        residual_ns: f64,
        allowed_ns: f64,
    },
    AggregateHeatingMismatch {
        residual_j: f64,
        allowed_j: f64,
    },
}

impl fmt::Display for DragTrajectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBatch => write!(f, "no time-varying drag events supplied"),
            Self::EmptyEvent(id) => {
                write!(f, "drag event {id} has no space-time quadrature samples")
            }
            Self::InvalidTolerance => write!(
                f,
                "drag trajectory tolerances must be finite and non-negative"
            ),
            Self::DuplicateEventId(id) => write!(f, "duplicate drag trajectory event id: {id}"),
            Self::DuplicateSampleId {
                event_id,
                sample_id,
            } => write!(
                f,
                "drag event {event_id} has duplicate quadrature sample id {sample_id}"
            ),
            Self::CoordinateFrameMismatch {
                event_id,
                charged_frame_id,
                neutral_frame_id,
            } => write!(
                f,
                "drag event {event_id} mixes charged frame {charged_frame_id} and neutral frame {neutral_frame_id}"
            ),
            Self::InvalidTimeFraction {
                event_id,
                sample_id,
            } => write!(
                f,
                "drag event {event_id} sample {sample_id} has time fraction outside [0, 1] or non-finite"
            ),
            Self::InvalidIntegratedDragWeight {
                event_id,
                sample_id,
            } => write!(
                f,
                "drag event {event_id} sample {sample_id} has negative or non-finite integrated drag weight"
            ),
            Self::NonFiniteVelocity {
                event_id,
                sample_id,
            } => write!(
                f,
                "drag event {event_id} sample {sample_id} has a non-finite relative velocity"
            ),
            Self::NonFiniteReportedValue(id) => write!(
                f,
                "drag event {id} has a non-finite reported impulse or frictional heating"
            ),
            Self::NonFiniteAccumulation => write!(
                f,
                "time-varying drag momentum/energy accumulation became non-finite"
            ),
            Self::NeutralImpulseMismatch {
                event_id,
                component,
                supplied_ns,
                expected_ns,
                allowed_ns,
            } => write!(
                f,
                "drag event {event_id} neutral impulse component {component}: supplied {supplied_ns} N*s, expected {expected_ns} N*s, allowed residual {allowed_ns} N*s"
            ),
            Self::ChargedReactionMismatch {
                event_id,
                component,
                supplied_ns,
                expected_ns,
                allowed_ns,
            } => write!(
                f,
                "drag event {event_id} charged reaction component {component}: supplied {supplied_ns} N*s, expected {expected_ns} N*s, allowed residual {allowed_ns} N*s"
            ),
            Self::NegativeReportedHeating {
                event_id,
                supplied_j,
                allowed_negative_j,
            } => write!(
                f,
                "drag event {event_id} reports negative frictional heating {supplied_j} J, allowed negative tolerance {allowed_negative_j} J"
            ),
            Self::HeatingMismatch {
                event_id,
                supplied_j,
                expected_j,
                allowed_j,
            } => write!(
                f,
                "drag event {event_id} heating {supplied_j} J differs from quadrature prediction {expected_j} J by more than {allowed_j} J"
            ),
            Self::AggregateImpulseMismatch {
                component,
                residual_ns,
                allowed_ns,
            } => write!(
                f,
                "aggregate drag impulse component {component} residual {residual_ns} N*s exceeds {allowed_ns} N*s"
            ),
            Self::AggregateHeatingMismatch {
                residual_j,
                allowed_j,
            } => write!(
                f,
                "aggregate drag heating residual {residual_j} J exceeds {allowed_j} J"
            ),
        }
    }
}

impl Error for DragTrajectoryError {}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum
    }
}

/// Validate a batch of time-varying drag receipts.
///
/// Each event's impulse and heating are independently reconstructed from the
/// same positive space-time quadrature samples. The function checks the
/// event-local values before checking aggregate momentum and energy closure.
/// Quadrature accuracy itself must be established by order/refinement studies
/// in the calling solver adapter.
pub fn validate_drag_trajectory(
    events: &[DragTrajectoryEvent],
    tolerance: DragTrajectoryTolerance,
) -> Result<DragTrajectoryReceipt, DragTrajectoryError> {
    if events.is_empty() {
        return Err(DragTrajectoryError::EmptyBatch);
    }
    for value in [
        tolerance.impulse_absolute_ns,
        tolerance.impulse_relative,
        tolerance.energy_absolute_j,
        tolerance.energy_relative,
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(DragTrajectoryError::InvalidTolerance);
        }
    }

    let mut ordered_events = events.to_vec();
    ordered_events.sort_by_key(|event| event.event_id);
    let mut seen_event_ids = BTreeSet::new();
    let mut quadrature_rule_ids = BTreeMap::new();
    let mut coordinate_frame_ids = BTreeMap::new();

    let mut total_neutral = [CompensatedSum::default(); 3];
    let mut total_charged = [CompensatedSum::default(); 3];
    let mut total_impulse_scale = [CompensatedSum::default(); 3];
    let mut total_drag_weight = CompensatedSum::default();
    let mut total_predicted_heat = CompensatedSum::default();
    let mut total_reported_heat = CompensatedSum::default();
    let mut total_energy_scale = CompensatedSum::default();
    let mut sample_count = 0_usize;
    let mut max_event_impulse_residual_ns = 0.0_f64;
    let mut max_event_energy_residual_j = 0.0_f64;

    for event in &ordered_events {
        if !seen_event_ids.insert(event.event_id) {
            return Err(DragTrajectoryError::DuplicateEventId(event.event_id));
        }
        if event.samples.is_empty() {
            return Err(DragTrajectoryError::EmptyEvent(event.event_id));
        }
        if event.charged_frame_id != event.neutral_frame_id {
            return Err(DragTrajectoryError::CoordinateFrameMismatch {
                event_id: event.event_id,
                charged_frame_id: event.charged_frame_id,
                neutral_frame_id: event.neutral_frame_id,
            });
        }
        quadrature_rule_ids.insert(event.event_id, event.quadrature_rule_id);
        coordinate_frame_ids.insert(event.event_id, event.charged_frame_id);
        if !event.reported_frictional_heating_j.is_finite()
            || !event
                .neutral_impulse_ns
                .iter()
                .all(|value| value.is_finite())
            || !event
                .charged_reaction_impulse_ns
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(DragTrajectoryError::NonFiniteReportedValue(event.event_id));
        }

        let mut samples = event.samples.clone();
        samples.sort_by(|left, right| {
            left.time_fraction
                .total_cmp(&right.time_fraction)
                .then(left.sample_id.cmp(&right.sample_id))
        });
        let mut seen_sample_ids = BTreeSet::new();

        let mut expected_neutral = [CompensatedSum::default(); 3];
        let mut expected_drag_heat = CompensatedSum::default();
        let mut event_drag_weight = CompensatedSum::default();
        for sample in &samples {
            if !seen_sample_ids.insert(sample.sample_id) {
                return Err(DragTrajectoryError::DuplicateSampleId {
                    event_id: event.event_id,
                    sample_id: sample.sample_id,
                });
            }
            if !sample.time_fraction.is_finite() || !(0.0..=1.0).contains(&sample.time_fraction) {
                return Err(DragTrajectoryError::InvalidTimeFraction {
                    event_id: event.event_id,
                    sample_id: sample.sample_id,
                });
            }
            if !sample.integrated_drag_weight_kg.is_finite()
                || sample.integrated_drag_weight_kg < 0.0
            {
                return Err(DragTrajectoryError::InvalidIntegratedDragWeight {
                    event_id: event.event_id,
                    sample_id: sample.sample_id,
                });
            }
            if !sample
                .relative_velocity_m_s
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(DragTrajectoryError::NonFiniteVelocity {
                    event_id: event.event_id,
                    sample_id: sample.sample_id,
                });
            }

            let drag_weight = sample.integrated_drag_weight_kg;
            let mut speed_squared = 0.0_f64;
            for component in 0..3 {
                let velocity = sample.relative_velocity_m_s[component];
                expected_neutral[component].add(drag_weight * velocity);
                speed_squared += velocity * velocity;
            }
            let heating_contribution = drag_weight * speed_squared;
            if !speed_squared.is_finite() || !heating_contribution.is_finite() {
                return Err(DragTrajectoryError::NonFiniteAccumulation);
            }
            expected_drag_heat.add(heating_contribution);
            event_drag_weight.add(drag_weight);
        }

        let expected_neutral = [
            expected_neutral[0].value(),
            expected_neutral[1].value(),
            expected_neutral[2].value(),
        ];
        let expected_drag_heat = expected_drag_heat.value();
        let event_drag_weight = event_drag_weight.value();
        if !expected_drag_heat.is_finite() || !event_drag_weight.is_finite() {
            return Err(DragTrajectoryError::NonFiniteAccumulation);
        }

        for component in 0..3 {
            let expected_charged = -expected_neutral[component];
            let supplied_neutral = event.neutral_impulse_ns[component];
            let supplied_charged = event.charged_reaction_impulse_ns[component];
            let neutral_allowed = tolerance.impulse_absolute_ns
                + tolerance.impulse_relative
                    * expected_neutral[component]
                        .abs()
                        .max(supplied_neutral.abs());
            let charged_allowed = tolerance.impulse_absolute_ns
                + tolerance.impulse_relative * expected_charged.abs().max(supplied_charged.abs());
            let neutral_residual = supplied_neutral - expected_neutral[component];
            let charged_residual = supplied_charged - expected_charged;

            if !neutral_allowed.is_finite()
                || !charged_allowed.is_finite()
                || !neutral_residual.is_finite()
                || !charged_residual.is_finite()
            {
                return Err(DragTrajectoryError::NonFiniteAccumulation);
            }
            max_event_impulse_residual_ns = max_event_impulse_residual_ns
                .max(neutral_residual.abs())
                .max(charged_residual.abs());

            if neutral_residual.abs() > neutral_allowed {
                return Err(DragTrajectoryError::NeutralImpulseMismatch {
                    event_id: event.event_id,
                    component,
                    supplied_ns: supplied_neutral,
                    expected_ns: expected_neutral[component],
                    allowed_ns: neutral_allowed,
                });
            }
            if charged_residual.abs() > charged_allowed {
                return Err(DragTrajectoryError::ChargedReactionMismatch {
                    event_id: event.event_id,
                    component,
                    supplied_ns: supplied_charged,
                    expected_ns: expected_charged,
                    allowed_ns: charged_allowed,
                });
            }

            total_neutral[component].add(supplied_neutral);
            total_charged[component].add(supplied_charged);
            total_impulse_scale[component].add(supplied_neutral.abs() + supplied_charged.abs());
        }

        let supplied_heat = event.reported_frictional_heating_j;
        let energy_allowed = tolerance.energy_absolute_j
            + tolerance.energy_relative * expected_drag_heat.abs().max(supplied_heat.abs());
        if !energy_allowed.is_finite() {
            return Err(DragTrajectoryError::NonFiniteAccumulation);
        }
        if supplied_heat < -energy_allowed {
            return Err(DragTrajectoryError::NegativeReportedHeating {
                event_id: event.event_id,
                supplied_j: supplied_heat,
                allowed_negative_j: energy_allowed,
            });
        }
        let energy_residual = supplied_heat - expected_drag_heat;
        if !energy_residual.is_finite() {
            return Err(DragTrajectoryError::NonFiniteAccumulation);
        }
        max_event_energy_residual_j = max_event_energy_residual_j.max(energy_residual.abs());
        if energy_residual.abs() > energy_allowed {
            return Err(DragTrajectoryError::HeatingMismatch {
                event_id: event.event_id,
                supplied_j: supplied_heat,
                expected_j: expected_drag_heat,
                allowed_j: energy_allowed,
            });
        }

        total_drag_weight.add(event_drag_weight);
        total_predicted_heat.add(expected_drag_heat);
        total_reported_heat.add(supplied_heat);
        total_energy_scale.add(expected_drag_heat.abs() + supplied_heat.abs());
        sample_count += samples.len();
    }

    let total_neutral_impulse_ns = [
        total_neutral[0].value(),
        total_neutral[1].value(),
        total_neutral[2].value(),
    ];
    let total_charged_reaction_impulse_ns = [
        total_charged[0].value(),
        total_charged[1].value(),
        total_charged[2].value(),
    ];
    let mut aggregate_impulse_residual_ns = [0.0_f64; 3];

    for component in 0..3 {
        let residual =
            total_neutral_impulse_ns[component] + total_charged_reaction_impulse_ns[component];
        let allowed = tolerance.impulse_absolute_ns
            + tolerance.impulse_relative * total_impulse_scale[component].value();
        if !residual.is_finite() || !allowed.is_finite() {
            return Err(DragTrajectoryError::NonFiniteAccumulation);
        }
        aggregate_impulse_residual_ns[component] = residual;
        if residual.abs() > allowed {
            return Err(DragTrajectoryError::AggregateImpulseMismatch {
                component,
                residual_ns: residual,
                allowed_ns: allowed,
            });
        }
    }

    let predicted_frictional_heating_j = total_predicted_heat.value();
    let reported_frictional_heating_j = total_reported_heat.value();
    let aggregate_energy_residual_j =
        reported_frictional_heating_j - predicted_frictional_heating_j;
    let aggregate_energy_allowed =
        tolerance.energy_absolute_j + tolerance.energy_relative * total_energy_scale.value();
    if !predicted_frictional_heating_j.is_finite()
        || !reported_frictional_heating_j.is_finite()
        || !aggregate_energy_residual_j.is_finite()
        || !aggregate_energy_allowed.is_finite()
    {
        return Err(DragTrajectoryError::NonFiniteAccumulation);
    }
    if aggregate_energy_residual_j.abs() > aggregate_energy_allowed {
        return Err(DragTrajectoryError::AggregateHeatingMismatch {
            residual_j: aggregate_energy_residual_j,
            allowed_j: aggregate_energy_allowed,
        });
    }

    Ok(DragTrajectoryReceipt {
        event_count: ordered_events.len(),
        sample_count,
        quadrature_rule_ids,
        coordinate_frame_ids,
        total_integrated_drag_weight_kg: total_drag_weight.value(),
        total_neutral_impulse_ns,
        total_charged_reaction_impulse_ns,
        aggregate_impulse_residual_ns,
        predicted_frictional_heating_j,
        reported_frictional_heating_j,
        aggregate_energy_residual_j,
        max_event_impulse_residual_ns,
        max_event_energy_residual_j,
    })
}

/// Three-point Simpson manufactured solution for K(t)=2 kg/s and
/// du(t)=[1+2t, -1+t, 0.5] m/s over one second.
pub fn reference_self_check() -> Result<(), DragTrajectoryError> {
    let event = DragTrajectoryEvent {
        event_id: 1,
        quadrature_rule_id: 3,
        charged_frame_id: 7,
        neutral_frame_id: 7,
        samples: vec![
            DragQuadratureSample {
                sample_id: 1,
                time_fraction: 0.0,
                integrated_drag_weight_kg: 1.0 / 3.0,
                relative_velocity_m_s: [1.0, -1.0, 0.5],
            },
            DragQuadratureSample {
                sample_id: 2,
                time_fraction: 0.5,
                integrated_drag_weight_kg: 4.0 / 3.0,
                relative_velocity_m_s: [2.0, -0.5, 0.5],
            },
            DragQuadratureSample {
                sample_id: 3,
                time_fraction: 1.0,
                integrated_drag_weight_kg: 1.0 / 3.0,
                relative_velocity_m_s: [3.0, 0.0, 0.5],
            },
        ],
        neutral_impulse_ns: [4.0, -1.0, 1.0],
        charged_reaction_impulse_ns: [-4.0, 1.0, -1.0],
        reported_frictional_heating_j: 59.0 / 6.0,
    };
    let receipt = validate_drag_trajectory(
        &[event],
        DragTrajectoryTolerance {
            impulse_absolute_ns: 1.0e-12,
            impulse_relative: 1.0e-12,
            energy_absolute_j: 1.0e-12,
            energy_relative: 1.0e-12,
        },
    )?;
    if receipt.total_neutral_impulse_ns != [4.0, -1.0, 1.0]
        || receipt.total_charged_reaction_impulse_ns != [-4.0, 1.0, -1.0]
        || (receipt.predicted_frictional_heating_j - 59.0 / 6.0).abs() > 1.0e-12
    {
        return Err(DragTrajectoryError::AggregateHeatingMismatch {
            residual_j: receipt.aggregate_energy_residual_j,
            allowed_j: 1.0e-12,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance() -> DragTrajectoryTolerance {
        DragTrajectoryTolerance {
            impulse_absolute_ns: 1.0e-12,
            impulse_relative: 1.0e-12,
            energy_absolute_j: 1.0e-12,
            energy_relative: 1.0e-12,
        }
    }

    fn sample(
        sample_id: u64,
        time_fraction: f64,
        integrated_drag_weight_kg: f64,
        relative_velocity_m_s: [f64; 3],
    ) -> DragQuadratureSample {
        DragQuadratureSample {
            sample_id,
            time_fraction,
            integrated_drag_weight_kg,
            relative_velocity_m_s,
        }
    }

    fn simpson_event() -> DragTrajectoryEvent {
        DragTrajectoryEvent {
            event_id: 1,
            quadrature_rule_id: 3,
            charged_frame_id: 7,
            neutral_frame_id: 7,
            samples: vec![
                sample(1, 0.0, 1.0 / 3.0, [1.0, -1.0, 0.5]),
                sample(2, 0.5, 4.0 / 3.0, [2.0, -0.5, 0.5]),
                sample(3, 1.0, 1.0 / 3.0, [3.0, 0.0, 0.5]),
            ],
            neutral_impulse_ns: [4.0, -1.0, 1.0],
            charged_reaction_impulse_ns: [-4.0, 1.0, -1.0],
            reported_frictional_heating_j: 59.0 / 6.0,
        }
    }

    #[test]
    fn variable_drift_simpson_receipt_matches_impulse_and_frictional_heating() {
        let receipt = validate_drag_trajectory(&[simpson_event()], tolerance()).unwrap();
        assert_eq!(receipt.total_neutral_impulse_ns, [4.0, -1.0, 1.0]);
        assert!((receipt.predicted_frictional_heating_j - 59.0 / 6.0).abs() < 1.0e-12);
        assert_eq!(receipt.aggregate_impulse_residual_ns, [0.0; 3]);
    }

    #[test]
    fn constant_drift_reduces_to_the_integrated_drag_identity() {
        let event = DragTrajectoryEvent {
            event_id: 2,
            quadrature_rule_id: 1,
            charged_frame_id: 7,
            neutral_frame_id: 7,
            samples: vec![
                sample(1, 0.0, 0.25, [3.0, -1.0, 2.0]),
                sample(2, 1.0, 0.75, [3.0, -1.0, 2.0]),
            ],
            neutral_impulse_ns: [3.0, -1.0, 2.0],
            charged_reaction_impulse_ns: [-3.0, 1.0, -2.0],
            reported_frictional_heating_j: 14.0,
        };
        let receipt = validate_drag_trajectory(&[event], tolerance()).unwrap();
        assert_eq!(receipt.total_integrated_drag_weight_kg, 1.0);
        assert_eq!(receipt.predicted_frictional_heating_j, 14.0);
    }

    #[test]
    fn rejects_negative_integrated_drag_weight() {
        let mut event = simpson_event();
        event.samples[1].integrated_drag_weight_kg = -1.0;
        assert!(matches!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::InvalidIntegratedDragWeight {
                event_id: 1,
                sample_id: 2
            }
        ));
    }

    #[test]
    fn rejects_incorrect_impulse_even_when_heating_matches() {
        let mut event = simpson_event();
        event.neutral_impulse_ns[0] += 0.1;
        assert!(matches!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::NeutralImpulseMismatch {
                event_id: 1,
                component: 0,
                ..
            }
        ));
    }

    #[test]
    fn rejects_incorrect_heating_even_when_impulse_matches() {
        let mut event = simpson_event();
        event.reported_frictional_heating_j = 0.0;
        assert!(matches!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::HeatingMismatch { event_id: 1, .. }
        ));
    }

    #[test]
    fn rejects_negative_reported_heating() {
        let mut event = simpson_event();
        event.reported_frictional_heating_j = -0.1;
        assert!(matches!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::NegativeReportedHeating { event_id: 1, .. }
        ));
    }

    #[test]
    fn rejects_a_global_momentum_error_that_cancels_between_events() {
        let mut first = simpson_event();
        first.event_id = 1;
        first.neutral_impulse_ns[0] += 0.1;
        let mut second = simpson_event();
        second.event_id = 2;
        second.neutral_impulse_ns[0] -= 0.1;
        assert!(matches!(
            validate_drag_trajectory(&[first, second], tolerance()).unwrap_err(),
            DragTrajectoryError::NeutralImpulseMismatch {
                event_id: 1,
                component: 0,
                ..
            }
        ));
    }

    #[test]
    fn rejects_duplicate_event_and_sample_ids() {
        let event = simpson_event();
        assert_eq!(
            validate_drag_trajectory(&[event.clone(), event.clone()], tolerance()).unwrap_err(),
            DragTrajectoryError::DuplicateEventId(1)
        );
        let mut event = simpson_event();
        event.samples[2].sample_id = event.samples[1].sample_id;
        assert_eq!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::DuplicateSampleId {
                event_id: 1,
                sample_id: 2
            }
        );
    }

    #[test]
    fn rejects_implicit_coordinate_frame_mismatch() {
        let mut event = simpson_event();
        event.neutral_frame_id = 8;
        assert_eq!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::CoordinateFrameMismatch {
                event_id: 1,
                charged_frame_id: 7,
                neutral_frame_id: 8
            }
        );
    }

    #[test]
    fn rejects_out_of_range_time_and_non_finite_velocity() {
        let mut event = simpson_event();
        event.samples[0].time_fraction = 1.1;
        assert!(matches!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::InvalidTimeFraction { .. }
        ));
        let mut event = simpson_event();
        event.samples[0].relative_velocity_m_s[0] = f64::NAN;
        assert!(matches!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::NonFiniteVelocity { .. }
        ));
    }

    #[test]
    fn receipts_are_deterministic_under_event_and_sample_permutation() {
        let event = simpson_event();
        let mut reversed = event.clone();
        reversed.samples.reverse();
        let forward = validate_drag_trajectory(&[event], tolerance()).unwrap();
        let reverse = validate_drag_trajectory(&[reversed], tolerance()).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn rejects_empty_batches_events_and_invalid_tolerances() {
        assert_eq!(
            validate_drag_trajectory(&[], tolerance()).unwrap_err(),
            DragTrajectoryError::EmptyBatch
        );
        let mut event = simpson_event();
        event.samples.clear();
        assert_eq!(
            validate_drag_trajectory(&[event], tolerance()).unwrap_err(),
            DragTrajectoryError::EmptyEvent(1)
        );
        let invalid = DragTrajectoryTolerance {
            energy_relative: f64::NAN,
            ..tolerance()
        };
        assert_eq!(
            validate_drag_trajectory(&[simpson_event()], invalid).unwrap_err(),
            DragTrajectoryError::InvalidTolerance
        );
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
