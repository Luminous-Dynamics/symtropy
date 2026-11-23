// SPDX-License-Identifier: AGPL-3.0-or-later

//! Analytic ion-neutral drag momentum/energy consistency reference.
//!
//! For a constant relative drift over one source space-time event, the
//! integrated drag coefficient is
//!
//!     Gamma = integral(m_s * n_s * nu_sn dV dt)   [kg].
//!
//! With relative velocity du = u_charged - u_neutral, the neutral receives
//! I_n = Gamma * du and the charged phase receives -I_n. The combined kinetic
//! power of the two bulk phases is -k*|du|^2, so the integrated collisional
//! frictional heating is Gamma*|du|^2 = I_n dot du >= 0.
//!
//! This is a constant-drift manufactured case. It does not model thermal
//! energy partition, velocity-dependent collision operators, reactions,
//! charge exchange, or a general time-varying kinetic collision integral.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// One integrated explicit species-collision-drag contribution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragEvent {
    pub event_id: u64,
    /// Integral of m_s * n_s * nu_sn over the event space-time slab [kg].
    pub integrated_drag_coefficient_kg: f64,
    /// Constant charged-minus-neutral bulk drift over this event [m/s].
    pub relative_velocity_m_s: [f64; 3],
    /// Integrated impulse deposited into neutrals [N*s].
    pub neutral_impulse_ns: [f64; 3],
    /// Integrated equal-and-opposite reaction applied to charged species [N*s].
    pub charged_reaction_impulse_ns: [f64; 3],
}

/// Case-specific componentwise impulse and scalar energy tolerances.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragTolerance {
    pub impulse_absolute_ns: f64,
    pub impulse_relative: f64,
    pub energy_absolute_j: f64,
    pub energy_relative: f64,
}

/// Integrated momentum and frictional-heating report for a batch of events.
#[derive(Clone, Debug, PartialEq)]
pub struct DragDissipationReport {
    pub event_count: usize,
    pub total_neutral_impulse_ns: [f64; 3],
    pub total_charged_reaction_impulse_ns: [f64; 3],
    pub aggregate_impulse_residual_ns: [f64; 3],
    /// Sum of Gamma * |relative_velocity|^2 [J].
    pub predicted_frictional_heating_j: f64,
    /// Sum of supplied neutral impulse dot relative velocity [J].
    pub impulse_work_j: f64,
    /// impulse_work_j - predicted_frictional_heating_j.
    pub energy_identity_residual_j: f64,
    pub max_event_impulse_residual_ns: f64,
    pub max_event_energy_residual_j: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DragError {
    EmptyBatch,
    InvalidTolerance,
    DuplicateEventId(u64),
    InvalidDragCoefficient(u64),
    NonFiniteInput(u64),
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
    NegativeFrictionalHeating {
        event_id: u64,
        supplied_work_j: f64,
        allowed_negative_j: f64,
    },
    EventEnergyMismatch {
        event_id: u64,
        residual_j: f64,
        allowed_j: f64,
    },
    AggregateImpulseMismatch {
        component: usize,
        residual_ns: f64,
        allowed_ns: f64,
    },
    AggregateEnergyMismatch {
        residual_j: f64,
        allowed_j: f64,
    },
}

impl fmt::Display for DragError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBatch => write!(f, "no drag events supplied"),
            Self::InvalidTolerance => write!(f, "drag tolerances must be finite and non-negative"),
            Self::DuplicateEventId(id) => write!(f, "duplicate drag event id: {id}"),
            Self::InvalidDragCoefficient(id) => write!(
                f,
                "drag event {id} has a negative or non-finite integrated drag coefficient"
            ),
            Self::NonFiniteInput(id) => write!(f, "drag event {id} has a non-finite input"),
            Self::NonFiniteAccumulation => {
                write!(f, "drag momentum/energy accumulation became non-finite")
            }
            Self::NeutralImpulseMismatch {
                event_id,
                component,
                supplied_ns,
                expected_ns,
                allowed_ns,
            } => write!(
                f,
                "drag event {event_id} neutral impulse component {component}: {supplied_ns} N*s != expected {expected_ns} N*s within {allowed_ns}"
            ),
            Self::ChargedReactionMismatch {
                event_id,
                component,
                supplied_ns,
                expected_ns,
                allowed_ns,
            } => write!(
                f,
                "drag event {event_id} charged reaction component {component}: {supplied_ns} N*s != expected {expected_ns} N*s within {allowed_ns}"
            ),
            Self::NegativeFrictionalHeating {
                event_id,
                supplied_work_j,
                allowed_negative_j,
            } => write!(
                f,
                "drag event {event_id} creates negative frictional heating: {supplied_work_j} J, allowed negative {allowed_negative_j} J"
            ),
            Self::EventEnergyMismatch {
                event_id,
                residual_j,
                allowed_j,
            } => write!(
                f,
                "drag event {event_id} frictional-energy residual {residual_j} J exceeds allowed {allowed_j} J"
            ),
            Self::AggregateImpulseMismatch {
                component,
                residual_ns,
                allowed_ns,
            } => write!(
                f,
                "aggregate drag impulse component {component} residual {residual_ns} N*s exceeds allowed {allowed_ns} N*s"
            ),
            Self::AggregateEnergyMismatch {
                residual_j,
                allowed_j,
            } => write!(
                f,
                "aggregate drag energy residual {residual_j} J exceeds allowed {allowed_j} J"
            ),
        }
    }
}

impl Error for DragError {}

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

/// Verify the constant-drift ion-neutral drag identity for a batch of events.
///
/// Per event it checks:
/// - neutral impulse = Gamma * relative velocity;
/// - charged reaction impulse = -neutral impulse;
/// - frictional heating = Gamma * |relative velocity|^2;
/// - supplied impulse work = neutral impulse dot relative velocity >= 0.
///
/// It then independently checks aggregate momentum and energy residuals.
pub fn validate_drag_dissipation(
    events: &[DragEvent],
    tolerance: DragTolerance,
) -> Result<DragDissipationReport, DragError> {
    if events.is_empty() {
        return Err(DragError::EmptyBatch);
    }
    for value in [
        tolerance.impulse_absolute_ns,
        tolerance.impulse_relative,
        tolerance.energy_absolute_j,
        tolerance.energy_relative,
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(DragError::InvalidTolerance);
        }
    }

    let mut ordered = events.to_vec();
    ordered.sort_by_key(|event| event.event_id);
    let mut seen = BTreeSet::new();

    let mut neutral_totals = [CompensatedSum::default(); 3];
    let mut charged_totals = [CompensatedSum::default(); 3];
    let mut impulse_scales = [CompensatedSum::default(); 3];
    let mut predicted_heat = CompensatedSum::default();
    let mut impulse_work = CompensatedSum::default();
    let mut energy_scale = CompensatedSum::default();
    let mut max_event_impulse_residual_ns = 0.0_f64;
    let mut max_event_energy_residual_j = 0.0_f64;

    for event in &ordered {
        if !seen.insert(event.event_id) {
            return Err(DragError::DuplicateEventId(event.event_id));
        }
        let gamma = event.integrated_drag_coefficient_kg;
        if !gamma.is_finite() || gamma < 0.0 {
            return Err(DragError::InvalidDragCoefficient(event.event_id));
        }
        if !event
            .relative_velocity_m_s
            .iter()
            .all(|value| value.is_finite())
            || !event
                .neutral_impulse_ns
                .iter()
                .all(|value| value.is_finite())
            || !event
                .charged_reaction_impulse_ns
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(DragError::NonFiniteInput(event.event_id));
        }

        let mut expected_heat_j = 0.0_f64;
        let mut supplied_work_j = 0.0_f64;
        for component in 0..3 {
            let drift = event.relative_velocity_m_s[component];
            let expected_neutral = gamma * drift;
            let expected_charged = -expected_neutral;
            if !expected_neutral.is_finite() {
                return Err(DragError::NonFiniteAccumulation);
            }

            let neutral = event.neutral_impulse_ns[component];
            let charged = event.charged_reaction_impulse_ns[component];
            let neutral_allowed = tolerance.impulse_absolute_ns
                + tolerance.impulse_relative * expected_neutral.abs().max(neutral.abs());
            let charged_allowed = tolerance.impulse_absolute_ns
                + tolerance.impulse_relative * expected_charged.abs().max(charged.abs());
            let neutral_residual = neutral - expected_neutral;
            let charged_residual = charged - expected_charged;

            if !neutral_allowed.is_finite()
                || !charged_allowed.is_finite()
                || !neutral_residual.is_finite()
                || !charged_residual.is_finite()
            {
                return Err(DragError::NonFiniteAccumulation);
            }
            max_event_impulse_residual_ns = max_event_impulse_residual_ns
                .max(neutral_residual.abs())
                .max(charged_residual.abs());

            if neutral_residual.abs() > neutral_allowed {
                return Err(DragError::NeutralImpulseMismatch {
                    event_id: event.event_id,
                    component,
                    supplied_ns: neutral,
                    expected_ns: expected_neutral,
                    allowed_ns: neutral_allowed,
                });
            }
            if charged_residual.abs() > charged_allowed {
                return Err(DragError::ChargedReactionMismatch {
                    event_id: event.event_id,
                    component,
                    supplied_ns: charged,
                    expected_ns: expected_charged,
                    allowed_ns: charged_allowed,
                });
            }

            expected_heat_j += gamma * drift * drift;
            supplied_work_j += neutral * drift;
            neutral_totals[component].add(neutral);
            charged_totals[component].add(charged);
            impulse_scales[component].add(neutral.abs() + charged.abs());
        }

        if !expected_heat_j.is_finite() || !supplied_work_j.is_finite() {
            return Err(DragError::NonFiniteAccumulation);
        }
        let energy_allowed = tolerance.energy_absolute_j
            + tolerance.energy_relative * expected_heat_j.abs().max(supplied_work_j.abs());
        if !energy_allowed.is_finite() {
            return Err(DragError::NonFiniteAccumulation);
        }
        if supplied_work_j < -energy_allowed {
            return Err(DragError::NegativeFrictionalHeating {
                event_id: event.event_id,
                supplied_work_j,
                allowed_negative_j: energy_allowed,
            });
        }
        let energy_residual = supplied_work_j - expected_heat_j;
        max_event_energy_residual_j = max_event_energy_residual_j.max(energy_residual.abs());
        if energy_residual.abs() > energy_allowed {
            return Err(DragError::EventEnergyMismatch {
                event_id: event.event_id,
                residual_j: energy_residual,
                allowed_j: energy_allowed,
            });
        }

        predicted_heat.add(expected_heat_j);
        impulse_work.add(supplied_work_j);
        energy_scale.add(expected_heat_j.abs() + supplied_work_j.abs());
    }

    let total_neutral_impulse_ns = [
        neutral_totals[0].value(),
        neutral_totals[1].value(),
        neutral_totals[2].value(),
    ];
    let total_charged_reaction_impulse_ns = [
        charged_totals[0].value(),
        charged_totals[1].value(),
        charged_totals[2].value(),
    ];
    let mut aggregate_impulse_residual_ns = [0.0; 3];

    for component in 0..3 {
        let residual =
            total_neutral_impulse_ns[component] + total_charged_reaction_impulse_ns[component];
        let allowed = tolerance.impulse_absolute_ns
            + tolerance.impulse_relative * impulse_scales[component].value();
        if !residual.is_finite() || !allowed.is_finite() {
            return Err(DragError::NonFiniteAccumulation);
        }
        aggregate_impulse_residual_ns[component] = residual;
        if residual.abs() > allowed {
            return Err(DragError::AggregateImpulseMismatch {
                component,
                residual_ns: residual,
                allowed_ns: allowed,
            });
        }
    }

    let predicted_frictional_heating_j = predicted_heat.value();
    let impulse_work_j = impulse_work.value();
    let energy_identity_residual_j = impulse_work_j - predicted_frictional_heating_j;
    let allowed_energy =
        tolerance.energy_absolute_j + tolerance.energy_relative * energy_scale.value();
    if !predicted_frictional_heating_j.is_finite()
        || !impulse_work_j.is_finite()
        || !energy_identity_residual_j.is_finite()
        || !allowed_energy.is_finite()
    {
        return Err(DragError::NonFiniteAccumulation);
    }
    if energy_identity_residual_j.abs() > allowed_energy {
        return Err(DragError::AggregateEnergyMismatch {
            residual_j: energy_identity_residual_j,
            allowed_j: allowed_energy,
        });
    }

    Ok(DragDissipationReport {
        event_count: ordered.len(),
        total_neutral_impulse_ns,
        total_charged_reaction_impulse_ns,
        aggregate_impulse_residual_ns,
        predicted_frictional_heating_j,
        impulse_work_j,
        energy_identity_residual_j,
        max_event_impulse_residual_ns,
        max_event_energy_residual_j,
    })
}

/// Small deterministic reference fixture for the offline auditor.
pub fn reference_self_check() -> Result<(), DragError> {
    let events = [
        DragEvent {
            event_id: 1,
            integrated_drag_coefficient_kg: 2.0,
            relative_velocity_m_s: [3.0, -1.0, 2.0],
            neutral_impulse_ns: [6.0, -2.0, 4.0],
            charged_reaction_impulse_ns: [-6.0, 2.0, -4.0],
        },
        DragEvent {
            event_id: 2,
            integrated_drag_coefficient_kg: 0.5,
            relative_velocity_m_s: [2.0, 0.0, -2.0],
            neutral_impulse_ns: [1.0, 0.0, -1.0],
            charged_reaction_impulse_ns: [-1.0, 0.0, 1.0],
        },
    ];
    let report = validate_drag_dissipation(
        &events,
        DragTolerance {
            impulse_absolute_ns: 1.0e-12,
            impulse_relative: 1.0e-12,
            energy_absolute_j: 1.0e-12,
            energy_relative: 1.0e-12,
        },
    )?;
    if report.total_neutral_impulse_ns != [7.0, -2.0, 3.0]
        || report.total_charged_reaction_impulse_ns != [-7.0, 2.0, -3.0]
        || (report.predicted_frictional_heating_j - 32.0).abs() > 1.0e-12
        || (report.impulse_work_j - 32.0).abs() > 1.0e-12
    {
        return Err(DragError::AggregateEnergyMismatch {
            residual_j: report.energy_identity_residual_j,
            allowed_j: 1.0e-12,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance() -> DragTolerance {
        DragTolerance {
            impulse_absolute_ns: 1.0e-12,
            impulse_relative: 1.0e-12,
            energy_absolute_j: 1.0e-12,
            energy_relative: 1.0e-12,
        }
    }

    fn event(event_id: u64, gamma: f64, drift: [f64; 3]) -> DragEvent {
        let neutral = drift.map(|value| gamma * value);
        DragEvent {
            event_id,
            integrated_drag_coefficient_kg: gamma,
            relative_velocity_m_s: drift,
            neutral_impulse_ns: neutral,
            charged_reaction_impulse_ns: neutral.map(|value| -value),
        }
    }

    #[test]
    fn exact_constant_drift_drag_passes_momentum_and_energy_identity() {
        let report =
            validate_drag_dissipation(&[event(1, 2.0, [3.0, -1.0, 2.0])], tolerance()).unwrap();
        assert_eq!(report.total_neutral_impulse_ns, [6.0, -2.0, 4.0]);
        assert_eq!(report.total_charged_reaction_impulse_ns, [-6.0, 2.0, -4.0]);
        assert_eq!(report.predicted_frictional_heating_j, 28.0);
        assert_eq!(report.impulse_work_j, 28.0);
    }

    #[test]
    fn zero_drift_produces_zero_impulse_and_zero_frictional_heating() {
        let report = validate_drag_dissipation(&[event(1, 10.0, [0.0; 3])], tolerance()).unwrap();
        assert_eq!(report.total_neutral_impulse_ns, [0.0; 3]);
        assert_eq!(report.predicted_frictional_heating_j, 0.0);
    }

    #[test]
    fn rejects_negative_drag_coefficient() {
        let mut item = event(1, 1.0, [1.0, 0.0, 0.0]);
        item.integrated_drag_coefficient_kg = -1.0;
        assert_eq!(
            validate_drag_dissipation(&[item], tolerance()).unwrap_err(),
            DragError::InvalidDragCoefficient(1)
        );
    }

    #[test]
    fn rejects_anti_drag_impulse_that_creates_kinetic_energy() {
        let mut item = event(1, 1.0, [2.0, 0.0, 0.0]);
        item.neutral_impulse_ns = [-2.0, 0.0, 0.0];
        item.charged_reaction_impulse_ns = [2.0, 0.0, 0.0];
        let error = validate_drag_dissipation(&[item], tolerance()).unwrap_err();
        assert!(matches!(
            error,
            DragError::NeutralImpulseMismatch {
                event_id: 1,
                component: 0,
                ..
            }
        ));
    }

    #[test]
    fn rejects_negative_frictional_work_even_when_impulse_residual_is_within_its_tolerance() {
        let mut item = event(1, 0.0, [1.0, 0.0, 0.0]);
        item.neutral_impulse_ns = [-0.25, 0.0, 0.0];
        item.charged_reaction_impulse_ns = [0.25, 0.0, 0.0];
        let relaxed_impulse = DragTolerance {
            impulse_relative: 1.0,
            ..tolerance()
        };
        assert!(matches!(
            validate_drag_dissipation(&[item], relaxed_impulse).unwrap_err(),
            DragError::NegativeFrictionalHeating { event_id: 1, .. }
        ));
    }

    #[test]
    fn rejects_equal_opposite_but_wrong_drag_magnitude() {
        let mut item = event(1, 1.0, [2.0, 0.0, 0.0]);
        item.neutral_impulse_ns = [1.0, 0.0, 0.0];
        item.charged_reaction_impulse_ns = [-1.0, 0.0, 0.0];
        assert!(matches!(
            validate_drag_dissipation(&[item], tolerance()).unwrap_err(),
            DragError::NeutralImpulseMismatch {
                event_id: 1,
                component: 0,
                ..
            }
        ));
    }

    #[test]
    fn rejects_event_ids_duplicated_in_batch() {
        let item = event(1, 1.0, [1.0, 0.0, 0.0]);
        assert_eq!(
            validate_drag_dissipation(&[item, item], tolerance()).unwrap_err(),
            DragError::DuplicateEventId(1)
        );
    }

    #[test]
    fn rejects_non_finite_drift_and_invalid_tolerance() {
        let mut item = event(1, 1.0, [1.0, 0.0, 0.0]);
        item.relative_velocity_m_s[0] = f64::NAN;
        assert_eq!(
            validate_drag_dissipation(&[item], tolerance()).unwrap_err(),
            DragError::NonFiniteInput(1)
        );
        let invalid = DragTolerance {
            impulse_absolute_ns: -1.0,
            ..tolerance()
        };
        assert_eq!(
            validate_drag_dissipation(&[event(1, 1.0, [1.0, 0.0, 0.0])], invalid).unwrap_err(),
            DragError::InvalidTolerance
        );
    }

    #[test]
    fn result_is_deterministic_under_event_permutation() {
        let first = event(1, 2.0, [1.0, 2.0, 0.0]);
        let second = event(2, 0.5, [-2.0, 1.0, 3.0]);
        let a = validate_drag_dissipation(&[first, second], tolerance()).unwrap();
        let b = validate_drag_dissipation(&[second, first], tolerance()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
