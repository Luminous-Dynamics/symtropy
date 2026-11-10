// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Event-resolved one-dimensional unilateral impact step map.
//!
//! Position is a nonnegative gap above an immovable floor. Acceleration is
//! constant during a step. A single impact is resolved at its analytic event
//! time; restitution is applied to the incoming velocity. A perfectly plastic
//! impact (e=0) transitions to supported rest for the remainder of the step.
//!
//! More than one impact in a step is rejected as a typed unsupported case.
//! The returned derivative is branch-local and is withheld at event boundaries.

use std::error::Error;
use std::fmt;

/// State above a unilateral floor, with position interpreted as gap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactState1D {
    pub gap_m: f64,
    pub velocity_m_s: f64,
}

/// Dimensional tolerances for event detection and active-set classification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactTolerances {
    pub event_time_s: f64,
    pub gap_m: f64,
    pub normal_velocity_m_s: f64,
}

impl Default for ImpactTolerances {
    fn default() -> Self {
        Self {
            event_time_s: 1.0e-10,
            gap_m: 1.0e-12,
            normal_velocity_m_s: 1.0e-12,
        }
    }
}

/// Event-resolved impact step configuration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactModel1D {
    mass_kg: f64,
    acceleration_m_s2: f64,
    restitution: f64,
}

/// Derivative classification for the hybrid step map.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImpactDerivativeClass {
    /// The step remains strictly in free flight.
    SmoothFreeFlight,
    /// One event ordering is fixed and the event is strictly inside the step.
    BranchLocalSingleImpact,
    /// The initial state or impact event lies on a mode boundary.
    EventBoundary,
    /// Supported rest is a separate active-set branch; no cross-mode derivative
    /// is asserted by this seed.
    ActiveSupportUnavailable,
}

/// Discrete hybrid branch taken during the step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImpactBranch {
    FreeFlight,
    RestingSupport,
    PlasticImpactThenSupport,
    PlasticImpactThenFreeFlight,
    SingleRestitutionImpact,
}

/// Jacobian of the final [gap, velocity] with respect to the initial state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactStepJacobian {
    pub dgap_dgap: f64,
    pub dgap_dvelocity: f64,
    pub dvelocity_dgap: f64,
    pub dvelocity_dvelocity: f64,
}

/// Result of one event-resolved impact step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImpactStepOutcome {
    pub state: ImpactState1D,
    pub branch: ImpactBranch,
    pub derivative_class: ImpactDerivativeClass,
    pub event_time_s: Option<f64>,
    pub impact_impulse_ns: f64,
    pub support_impulse_ns: f64,
    /// Event-gap times impact-impulse plus final-gap times support-impulse.
    /// Units are metre-newton-seconds; zero is exact complementarity.
    pub complementarity_residual: f64,
    pub step_jacobian: Option<ImpactStepJacobian>,
}

/// Typed failures for the unilateral-impact map.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImpactError {
    InvalidMass,
    InvalidRestitution,
    InvalidState,
    InvalidTimestep,
    InvalidTolerance,
    MultipleImpactsInOneStep,
    GrazingOrUnresolvedEvent,
    UnsupportedOutwardAccelerationAtContact,
    NonFiniteResult,
}

impl fmt::Display for ImpactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMass => write!(f, "mass must be finite and positive"),
            Self::InvalidRestitution => write!(f, "restitution must be in [0, 1]"),
            Self::InvalidState => write!(f, "gap must be finite/nonnegative and velocity finite"),
            Self::InvalidTimestep => write!(f, "timestep must be finite and positive"),
            Self::InvalidTolerance => write!(f, "event tolerance must be finite and positive"),
            Self::MultipleImpactsInOneStep => write!(f, "multiple impacts in one step are unsupported"),
            Self::GrazingOrUnresolvedEvent => write!(f, "step crosses the contact surface without a resolvable inward event"),
            Self::UnsupportedOutwardAccelerationAtContact => {
                write!(f, "active support requires nonpositive normal acceleration")
            }
            Self::NonFiniteResult => write!(f, "impact step produced a non-finite result"),
        }
    }
}

impl Error for ImpactError {}

impl ImpactModel1D {
    pub fn try_new(
        mass_kg: f64,
        acceleration_m_s2: f64,
        restitution: f64,
    ) -> Result<Self, ImpactError> {
        if !mass_kg.is_finite() || mass_kg <= 0.0 {
            return Err(ImpactError::InvalidMass);
        }
        if !acceleration_m_s2.is_finite() {
            return Err(ImpactError::NonFiniteResult);
        }
        if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
            return Err(ImpactError::InvalidRestitution);
        }
        Ok(Self {
            mass_kg,
            acceleration_m_s2,
            restitution,
        })
    }

    pub fn restitution(&self) -> f64 {
        self.restitution
    }

    /// Advance by exact free flight or resolve at most one impact event.
    ///
    /// The event impulse is m(1+e)|v_minus|. For e=0, the body remains
    /// supported at the floor after impact; the additional support impulse
    /// balances constant inward acceleration for the rest of the step.
    pub fn step(
        &self,
        initial: ImpactState1D,
        dt_s: f64,
        tolerances: ImpactTolerances,
    ) -> Result<ImpactStepOutcome, ImpactError> {
        if !initial.gap_m.is_finite()
            || initial.gap_m < 0.0
            || !initial.velocity_m_s.is_finite()
        {
            return Err(ImpactError::InvalidState);
        }
        if !dt_s.is_finite() || dt_s <= 0.0 {
            return Err(ImpactError::InvalidTimestep);
        }
        if !tolerances.event_time_s.is_finite()
            || tolerances.event_time_s <= 0.0
            || !tolerances.gap_m.is_finite()
            || tolerances.gap_m <= 0.0
            || !tolerances.normal_velocity_m_s.is_finite()
            || tolerances.normal_velocity_m_s <= 0.0
        {
            return Err(ImpactError::InvalidTolerance);
        }

        // A body at rest on the floor with inward acceleration is already in
        // the support branch; do not manufacture a zero-time impact.
        if initial.gap_m == 0.0
            && initial.velocity_m_s == 0.0
            && self.acceleration_m_s2 < 0.0
        {
            let support_impulse = -self.mass_kg * self.acceleration_m_s2 * dt_s;
            return Ok(ImpactStepOutcome {
                state: initial,
                branch: ImpactBranch::RestingSupport,
                derivative_class: ImpactDerivativeClass::ActiveSupportUnavailable,
                event_time_s: None,
                impact_impulse_ns: 0.0,
                support_impulse_ns: support_impulse,
                complementarity_residual: 0.0,
                step_jacobian: None,
            });
        }

        let free_gap = initial.gap_m
            + initial.velocity_m_s * dt_s
            + 0.5 * self.acceleration_m_s2 * dt_s * dt_s;
        let free_velocity =
            initial.velocity_m_s + self.acceleration_m_s2 * dt_s;

        let Some(event_time) = self.first_inward_event(initial, dt_s, tolerances) else {
            if !free_gap.is_finite() || !free_velocity.is_finite() {
                return Err(ImpactError::NonFiniteResult);
            }
            if free_gap < -tolerances.gap_m {
                return Err(ImpactError::GrazingOrUnresolvedEvent);
            }
            let boundary = free_gap.abs() <= tolerances.gap_m;
            let state = ImpactState1D {
                gap_m: free_gap.max(0.0),
                velocity_m_s: free_velocity,
            };
            return Ok(ImpactStepOutcome {
                state,
                branch: ImpactBranch::FreeFlight,
                derivative_class: if boundary {
                    ImpactDerivativeClass::EventBoundary
                } else {
                    ImpactDerivativeClass::SmoothFreeFlight
                },
                event_time_s: None,
                impact_impulse_ns: 0.0,
                support_impulse_ns: 0.0,
                complementarity_residual: 0.0,
                step_jacobian: if boundary {
                    None
                } else {
                    Some(ImpactStepJacobian {
                        dgap_dgap: 1.0,
                        dgap_dvelocity: dt_s,
                        dvelocity_dgap: 0.0,
                        dvelocity_dvelocity: 1.0,
                    })
                },
            });
        };

        let incoming_velocity = initial.velocity_m_s
            + self.acceleration_m_s2 * event_time;
        if !incoming_velocity.is_finite() || incoming_velocity >= 0.0 {
            return Err(ImpactError::NonFiniteResult);
        }

        let impact_impulse = self.mass_kg
            * (1.0 + self.restitution)
            * (-incoming_velocity);
        let remaining = (dt_s - event_time).max(0.0);

        if self.restitution == 0.0 {
            let boundary = event_time <= tolerances.event_time_s
                || remaining <= tolerances.event_time_s
                || incoming_velocity.abs() <= tolerances.normal_velocity_m_s;
            let (state, branch, support_impulse, jacobian) =
                if self.acceleration_m_s2 <= 0.0 {
                    (
                        ImpactState1D {
                            gap_m: 0.0,
                            velocity_m_s: 0.0,
                        },
                        ImpactBranch::PlasticImpactThenSupport,
                        -self.mass_kg * self.acceleration_m_s2 * remaining,
                        Some(ImpactStepJacobian {
                            dgap_dgap: 0.0,
                            dgap_dvelocity: 0.0,
                            dvelocity_dgap: 0.0,
                            dvelocity_dvelocity: 0.0,
                        }),
                    )
                } else {
                    // No inward support force is needed. The body leaves the
                    // contact surface under outward/zero acceleration.
                    let final_gap = 0.5 * self.acceleration_m_s2 * remaining * remaining;
                    let final_velocity = self.acceleration_m_s2 * remaining;
                    (
                        ImpactState1D {
                            gap_m: final_gap,
                            velocity_m_s: final_velocity,
                        },
                        ImpactBranch::PlasticImpactThenFreeFlight,
                        0.0,
                        Some(ImpactStepJacobian {
                            dgap_dgap: -self.acceleration_m_s2
                                * remaining
                                * (1.0 / (-incoming_velocity)),
                            dgap_dvelocity: -self.acceleration_m_s2
                                * remaining
                                * ((-1.0 - initial.velocity_m_s / (-incoming_velocity))
                                    / self.acceleration_m_s2),
                            dvelocity_dgap: -self.acceleration_m_s2 * (1.0 / (-incoming_velocity)),
                            dvelocity_dvelocity: -self.acceleration_m_s2
                                * ((-1.0 - initial.velocity_m_s / (-incoming_velocity))
                                    / self.acceleration_m_s2),
                        }),
                    )
                };

            let event_gap = initial.gap_m
                + initial.velocity_m_s * event_time
                + 0.5 * self.acceleration_m_s2 * event_time * event_time;
            let complementarity_residual =
                event_gap.abs() * impact_impulse + state.gap_m * support_impulse;
            let outcome = ImpactStepOutcome {
                state,
                branch,
                derivative_class: if boundary {
                    ImpactDerivativeClass::EventBoundary
                } else {
                    ImpactDerivativeClass::BranchLocalSingleImpact
                },
                event_time_s: Some(event_time),
                impact_impulse_ns: impact_impulse,
                support_impulse_ns: support_impulse,
                complementarity_residual,
                step_jacobian: if boundary { None } else { jacobian },
            };
            if !outcome.impact_impulse_ns.is_finite()
                || !outcome.support_impulse_ns.is_finite()
                || !outcome.state.gap_m.is_finite()
                || !outcome.state.velocity_m_s.is_finite()
                || !outcome.complementarity_residual.is_finite()
            {
                return Err(ImpactError::NonFiniteResult);
            }
            return Ok(outcome);
        }

        let outgoing_velocity = -self.restitution * incoming_velocity;
        let final_gap = outgoing_velocity * remaining
            + 0.5 * self.acceleration_m_s2 * remaining * remaining;
        let final_velocity =
            outgoing_velocity + self.acceleration_m_s2 * remaining;

        if final_gap < -tolerances.gap_m
            || (final_gap.abs() <= tolerances.gap_m
                && final_velocity < -tolerances.normal_velocity_m_s)
        {
            return Err(ImpactError::MultipleImpactsInOneStep);
        }
        if !final_gap.is_finite() || !final_velocity.is_finite() {
            return Err(ImpactError::NonFiniteResult);
        }

        let boundary = event_time <= tolerances.event_time_s
            || remaining <= tolerances.event_time_s
            || final_gap.abs() <= tolerances.gap_m
            || incoming_velocity.abs() <= tolerances.normal_velocity_m_s;
        let jacobian = if boundary {
            None
        } else {
            Some(self.single_impact_jacobian(
                initial,
                dt_s,
                event_time,
                incoming_velocity,
            )?)
        };

        let event_gap = initial.gap_m
            + initial.velocity_m_s * event_time
            + 0.5 * self.acceleration_m_s2 * event_time * event_time;
        let complementarity_residual = event_gap.abs() * impact_impulse;
        if !impact_impulse.is_finite() || !complementarity_residual.is_finite() {
            return Err(ImpactError::NonFiniteResult);
        }

        Ok(ImpactStepOutcome {
            state: ImpactState1D {
                gap_m: final_gap.max(0.0),
                velocity_m_s: final_velocity,
            },
            branch: ImpactBranch::SingleRestitutionImpact,
            derivative_class: if boundary {
                ImpactDerivativeClass::EventBoundary
            } else {
                ImpactDerivativeClass::BranchLocalSingleImpact
            },
            event_time_s: Some(event_time),
            impact_impulse_ns: impact_impulse,
            support_impulse_ns: 0.0,
            complementarity_residual,
            step_jacobian: jacobian,
        })
    }

    fn first_inward_event(
        &self,
        state: ImpactState1D,
        dt_s: f64,
        tolerances: ImpactTolerances,
    ) -> Option<f64> {
        let acceleration = self.acceleration_m_s2;
        if state.gap_m == 0.0
            && state.velocity_m_s < -tolerances.normal_velocity_m_s
        {
            return Some(0.0);
        }
        if acceleration.abs() <= f64::MIN_POSITIVE {
            if state.velocity_m_s >= -tolerances.normal_velocity_m_s {
                return None;
            }
            let time = -state.gap_m / state.velocity_m_s;
            return if time >= -tolerances.event_time_s
                && time <= dt_s + tolerances.event_time_s
            {
                Some(time.clamp(0.0, dt_s))
            } else {
                None
            };
        }
        let discriminant = state.velocity_m_s * state.velocity_m_s
            - 2.0 * acceleration * state.gap_m;
        if !discriminant.is_finite() || discriminant < 0.0 {
            return None;
        }
        let root = discriminant.sqrt();
        let roots = [
            (-state.velocity_m_s - root) / acceleration,
            (-state.velocity_m_s + root) / acceleration,
        ];
        roots
            .iter()
            .copied()
            .filter(|time| {
                *time >= -tolerances.event_time_s
                    && *time <= dt_s + tolerances.event_time_s
            })
            .map(|time| time.clamp(0.0, dt_s))
            .filter(|time| {
                state.velocity_m_s + acceleration * *time
                    < -tolerances.normal_velocity_m_s
            })
            .min_by(f64::total_cmp)
    }

    fn single_impact_jacobian(
        &self,
        initial: ImpactState1D,
        dt_s: f64,
        event_time: f64,
        incoming_velocity: f64,
    ) -> Result<ImpactStepJacobian, ImpactError> {
        let speed = -incoming_velocity;
        if !speed.is_finite() || speed <= f64::MIN_POSITIVE {
            return Err(ImpactError::NonFiniteResult);
        }
        let acceleration = self.acceleration_m_s2;
        let restitution = self.restitution;
        let event_time_dgap = 1.0 / speed;
        let event_time_dvelocity = if acceleration.abs() <= f64::MIN_POSITIVE {
            event_time / speed
        } else {
            (-1.0 - initial.velocity_m_s / speed) / acceleration
        };
        let incoming_dgap = acceleration / speed;
        let incoming_dvelocity = -initial.velocity_m_s / speed;
        let outgoing_dgap = -restitution * incoming_dgap;
        let outgoing_dvelocity = -restitution * incoming_dvelocity;
        let remaining = dt_s - event_time;
        let remaining_dgap = -event_time_dgap;
        let remaining_dvelocity = -event_time_dvelocity;
        let final_velocity =
            -restitution * incoming_velocity + acceleration * remaining;
        let jacobian = ImpactStepJacobian {
            dgap_dgap: outgoing_dgap * remaining + final_velocity * remaining_dgap,
            dgap_dvelocity: outgoing_dvelocity * remaining
                + final_velocity * remaining_dvelocity,
            dvelocity_dgap: outgoing_dgap + acceleration * remaining_dgap,
            dvelocity_dvelocity: outgoing_dvelocity + acceleration * remaining_dvelocity,
        };
        if [
            jacobian.dgap_dgap,
            jacobian.dgap_dvelocity,
            jacobian.dvelocity_dgap,
            jacobian.dvelocity_dvelocity,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            Ok(jacobian)
        } else {
            Err(ImpactError::NonFiniteResult)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left:.16e} ~= {right:.16e} within {tolerance:e}"
        );
    }

    #[test]
    fn plastic_impact_resolves_event_and_supported_rest() {
        let model = ImpactModel1D::try_new(2.0, -9.81, 0.0).unwrap();
        let result = model
            .step(
                ImpactState1D {
                    gap_m: 1.0,
                    velocity_m_s: 0.0,
                },
                1.0,
                ImpactTolerances::default(),
            )
            .unwrap();
        close(result.event_time_s.unwrap(), (2.0_f64 / 9.81).sqrt(), 1.0e-12);
        close(result.state.gap_m, 0.0, 1.0e-15);
        close(result.state.velocity_m_s, 0.0, 1.0e-15);
        close(result.impact_impulse_ns, 2.0 * (2.0 * 9.81_f64).sqrt(), 1.0e-12);
        close(result.support_impulse_ns, 2.0 * 9.81 * (1.0 - result.event_time_s.unwrap()), 1.0e-12);
        assert_eq!(result.branch, ImpactBranch::PlasticImpactThenSupport);
        assert_eq!(
            result.derivative_class,
            ImpactDerivativeClass::BranchLocalSingleImpact
        );
        assert_eq!(
            result.step_jacobian,
            Some(ImpactStepJacobian {
                dgap_dgap: 0.0,
                dgap_dvelocity: 0.0,
                dvelocity_dgap: 0.0,
                dvelocity_dvelocity: 0.0,
            })
        );
    }

    #[test]
    fn free_flight_is_smooth_and_has_analytic_jacobian() {
        let model = ImpactModel1D::try_new(1.0, -9.81, 0.0).unwrap();
        let result = model
            .step(
                ImpactState1D {
                    gap_m: 10.0,
                    velocity_m_s: 0.0,
                },
                0.1,
                ImpactTolerances::default(),
            )
            .unwrap();
        assert_eq!(result.branch, ImpactBranch::FreeFlight);
        assert_eq!(result.derivative_class, ImpactDerivativeClass::SmoothFreeFlight);
        close(result.state.gap_m, 10.0 - 0.5 * 9.81 * 0.01, 1.0e-12);
        close(result.state.velocity_m_s, -0.981, 1.0e-12);
        assert_eq!(
            result.step_jacobian,
            Some(ImpactStepJacobian {
                dgap_dgap: 1.0,
                dgap_dvelocity: 0.1,
                dvelocity_dgap: 0.0,
                dvelocity_dvelocity: 1.0,
            })
        );
    }

    #[test]
    fn impact_complementarity_product_residual_is_small() {
        let model = ImpactModel1D::try_new(1.0, -9.81, 0.4).unwrap();
        let result = model.step(
            ImpactState1D {
                gap_m: 0.75,
                velocity_m_s: -0.1,
            },
            0.5,
            ImpactTolerances::default(),
        ).unwrap();
        assert!(result.complementarity_residual <= 1.0e-12);
    }

    #[test]
    fn restitution_impact_has_correct_impulse_and_event_time() {
        let model = ImpactModel1D::try_new(1.5, -9.81, 0.5).unwrap();
        let result = model
            .step(
                ImpactState1D {
                    gap_m: 1.0,
                    velocity_m_s: 0.0,
                },
                0.6,
                ImpactTolerances::default(),
            )
            .unwrap();
        let event_time = (2.0_f64 / 9.81).sqrt();
        let incoming_speed = (2.0_f64 * 9.81).sqrt();
        close(result.event_time_s.unwrap(), event_time, 1.0e-12);
        close(
            result.impact_impulse_ns,
            1.5 * 1.5 * incoming_speed,
            1.0e-12,
        );
        assert_eq!(result.branch, ImpactBranch::SingleRestitutionImpact);
        assert_eq!(result.derivative_class, ImpactDerivativeClass::BranchLocalSingleImpact);
        assert!(result.state.gap_m > 0.0);
        assert!(result.state.velocity_m_s > 0.0);
        assert!(result.step_jacobian.is_some());
    }

    #[test]
    fn zero_acceleration_restitution_jacobian_matches_finite_difference() {
        let model = ImpactModel1D::try_new(1.0, 0.0, 0.4).unwrap();
        let initial = ImpactState1D {
            gap_m: 0.5,
            velocity_m_s: -1.0,
        };
        let outcome = model.step(initial, 1.0, ImpactTolerances::default()).unwrap();
        let jacobian = outcome.step_jacobian.unwrap();
        close(outcome.event_time_s.unwrap(), 0.5, 1.0e-12);
        close(jacobian.dgap_dgap, -0.4, 1.0e-12);
        close(jacobian.dgap_dvelocity, -0.4, 1.0e-12);
        close(jacobian.dvelocity_dgap, 0.0, 1.0e-12);
        close(jacobian.dvelocity_dvelocity, -0.4, 1.0e-12);
    }

    #[test]
    fn impact_jacobian_matches_finite_difference_inside_one_branch() {
        let model = ImpactModel1D::try_new(1.0, -9.81, 0.3).unwrap();
        let initial = ImpactState1D {
            gap_m: 1.2,
            velocity_m_s: -0.2,
        };
        let dt = 0.7;
        let outcome = model.step(initial, dt, ImpactTolerances::default()).unwrap();
        let analytic = outcome.step_jacobian.unwrap();
        assert_eq!(
            outcome.derivative_class,
            ImpactDerivativeClass::BranchLocalSingleImpact
        );
        let epsilon = 1.0e-6;
        let plus_gap = model
            .step(
                ImpactState1D {
                    gap_m: initial.gap_m + epsilon,
                    ..initial
                },
                dt,
                ImpactTolerances::default(),
            )
            .unwrap()
            .state;
        let minus_gap = model
            .step(
                ImpactState1D {
                    gap_m: initial.gap_m - epsilon,
                    ..initial
                },
                dt,
                ImpactTolerances::default(),
            )
            .unwrap()
            .state;
        let plus_velocity = model
            .step(
                ImpactState1D {
                    velocity_m_s: initial.velocity_m_s + epsilon,
                    ..initial
                },
                dt,
                ImpactTolerances::default(),
            )
            .unwrap()
            .state;
        let minus_velocity = model
            .step(
                ImpactState1D {
                    velocity_m_s: initial.velocity_m_s - epsilon,
                    ..initial
                },
                dt,
                ImpactTolerances::default(),
            )
            .unwrap()
            .state;

        close(
            analytic.dgap_dgap,
            (plus_gap.gap_m - minus_gap.gap_m) / (2.0 * epsilon),
            2.0e-7,
        );
        close(
            analytic.dvelocity_dgap,
            (plus_gap.velocity_m_s - minus_gap.velocity_m_s) / (2.0 * epsilon),
            2.0e-7,
        );
        close(
            analytic.dgap_dvelocity,
            (plus_velocity.gap_m - minus_velocity.gap_m) / (2.0 * epsilon),
            2.0e-7,
        );
        close(
            analytic.dvelocity_dvelocity,
            (plus_velocity.velocity_m_s - minus_velocity.velocity_m_s) / (2.0 * epsilon),
            2.0e-7,
        );
    }

    #[test]
    fn zero_acceleration_constant_velocity_impact_is_resolved() {
        let model = ImpactModel1D::try_new(1.0, 0.0, 0.0).unwrap();
        let result = model.step(
            ImpactState1D {
                gap_m: 0.5,
                velocity_m_s: -1.0,
            },
            1.0,
            ImpactTolerances::default(),
        ).unwrap();
        close(result.event_time_s.unwrap(), 0.5, 1.0e-12);
        assert_eq!(result.state.gap_m, 0.0);
        assert_eq!(result.state.velocity_m_s, 0.0);
        assert_eq!(result.branch, ImpactBranch::PlasticImpactThenSupport);
        assert_eq!(result.support_impulse_ns, 0.0);
    }

    #[test]
    fn multiple_impacts_inside_one_step_fail_closed() {
        let model = ImpactModel1D::try_new(1.0, -9.81, 0.8).unwrap();
        let result = model.step(
            ImpactState1D {
                gap_m: 0.1,
                velocity_m_s: 0.0,
            },
            1.0,
            ImpactTolerances::default(),
        );
        assert_eq!(result, Err(ImpactError::MultipleImpactsInOneStep));
    }

    #[test]
    fn event_at_step_endpoint_is_classified_as_boundary() {
        let model = ImpactModel1D::try_new(1.0, -2.0, 0.5).unwrap();
        let result = model
            .step(
                ImpactState1D {
                    gap_m: 1.0,
                    velocity_m_s: 0.0,
                },
                1.0,
                ImpactTolerances {
                    event_time_s: 1.0e-9,
                    ..ImpactTolerances::default()
                },
            )
            .unwrap();
        assert_eq!(result.derivative_class, ImpactDerivativeClass::EventBoundary);
        assert!(result.step_jacobian.is_none());
    }

    #[test]
    fn plastic_impact_releases_under_outward_acceleration() {
        let model = ImpactModel1D::try_new(1.0, 1.0, 0.0).unwrap();
        let initial = ImpactState1D {
            gap_m: 0.2,
            velocity_m_s: -2.0,
        };
        let outcome = model.step(initial, 0.5, ImpactTolerances::default()).unwrap();
        assert_eq!(outcome.branch, ImpactBranch::PlasticImpactThenFreeFlight);
        assert_eq!(outcome.derivative_class, ImpactDerivativeClass::BranchLocalSingleImpact);
        assert!(outcome.event_time_s.unwrap() > 0.0);
        assert!(outcome.state.gap_m > 0.0);
        assert!(outcome.state.velocity_m_s > 0.0);
        assert_eq!(outcome.support_impulse_ns, 0.0);
        assert!(outcome.step_jacobian.is_some());
    }

    #[test]
    fn active_resting_contact_reports_support_impulse_and_withholds_derivative() {
        let model = ImpactModel1D::try_new(3.0, -9.81, 0.0).unwrap();
        let outcome = model
            .step(
                ImpactState1D {
                    gap_m: 0.0,
                    velocity_m_s: 0.0,
                },
                0.4,
                ImpactTolerances::default(),
            )
            .unwrap();
        assert_eq!(outcome.branch, ImpactBranch::RestingSupport);
        assert_eq!(
            outcome.derivative_class,
            ImpactDerivativeClass::ActiveSupportUnavailable
        );
        close(outcome.support_impulse_ns, 3.0 * 9.81 * 0.4, 1.0e-12);
        assert!(outcome.step_jacobian.is_none());
    }

    #[test]
    fn invalid_dimensional_tolerances_fail_closed() {
        let model = ImpactModel1D::try_new(1.0, -9.81, 0.0).unwrap();
        assert_eq!(
            model.step(
                ImpactState1D {
                    gap_m: 1.0,
                    velocity_m_s: 0.0,
                },
                0.5,
                ImpactTolerances {
                    event_time_s: 0.0,
                    ..ImpactTolerances::default()
                },
            ),
            Err(ImpactError::InvalidTolerance)
        );
        assert_eq!(
            model.step(
                ImpactState1D {
                    gap_m: 1.0,
                    velocity_m_s: 0.0,
                },
                0.5,
                ImpactTolerances {
                    normal_velocity_m_s: f64::NAN,
                    ..ImpactTolerances::default()
                },
            ),
            Err(ImpactError::InvalidTolerance)
        );
    }

    #[test]
    fn invalid_states_restitution_and_timestep_fail_closed() {
        assert_eq!(
            ImpactModel1D::try_new(0.0, -9.81, 0.0),
            Err(ImpactError::InvalidMass)
        );
        assert_eq!(
            ImpactModel1D::try_new(1.0, -9.81, 1.1),
            Err(ImpactError::InvalidRestitution)
        );
        let model = ImpactModel1D::try_new(1.0, -9.81, 0.0).unwrap();
        assert_eq!(
            model.step(
                ImpactState1D {
                    gap_m: -1.0,
                    velocity_m_s: 0.0
                },
                0.1,
                ImpactTolerances::default()
            ),
            Err(ImpactError::InvalidState)
        );
        assert_eq!(
            model.step(
                ImpactState1D {
                    gap_m: 1.0,
                    velocity_m_s: 0.0
                },
                0.0,
                ImpactTolerances::default()
            ),
            Err(ImpactError::InvalidTimestep)
        );
    }
}
