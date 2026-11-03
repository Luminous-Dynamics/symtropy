//! Analytical stick/slip boundary oracle for one-dimensional Coulomb contact.
//!
//! This is a branch-semantics reference, not a time integrator or a general
//! multi-body friction solver. It distinguishes a continuous kink
//! (mu_static == mu_kinetic) from a discontinuous static-to-kinetic switch
//! (mu_static > mu_kinetic). No branch Jacobians are averaged.

/// Schema identity included in deterministic model bytes.
pub const COULOMB_STICK_SLIP_SCHEMA: &str = "SYMTROPY_COULOMB_STICK_SLIP.v0.1";

/// Sign of tangential sliding at an incipient or established slip branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlipDirection {
    Positive,
    Negative,
}

impl SlipDirection {
    fn sign(self) -> f64 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
        }
    }
}

/// Parameters for a point mass on a horizontal plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoulombContact1D {
    /// Point mass in kilograms.
    pub mass_kg: f64,
    /// Unilateral normal load in newtons; strictly positive in this fixture.
    pub normal_force_n: f64,
    /// Static friction coefficient.
    pub mu_static: f64,
    /// Kinetic friction coefficient.
    pub mu_kinetic: f64,
    /// Caller-declared absolute force uncertainty around the static threshold.
    /// It is part of the model identity; it is never silently defaulted.
    pub force_tolerance_n: f64,
}

/// One physically interpretable branch response at a fixed applied force.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BranchResponse {
    /// Tangential reaction exerted by the plane, in newtons.
    pub friction_force_n: f64,
    /// Tangential acceleration on this branch, in metres per second squared.
    pub acceleration_m_s2: f64,
}

/// Directional derivative information for an exact static/kinetic boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoundarySensitivity {
    /// When static and kinetic coefficients are equal, acceleration is
    /// continuous but kinked. Both one-sided d(acceleration)/d(force) values
    /// are returned in (m/s^2)/N.
    ContinuousKink {
        stick_side_da_dforce: f64,
        slip_side_da_dforce: f64,
    },
    /// When static friction exceeds kinetic friction, acceleration jumps at
    /// incipient slip. There is no finite classical/Clarke derivative across
    /// this discontinuity; the signed jump is reported instead.
    Discontinuous {
        signed_acceleration_jump_m_s2: f64,
    },
}

/// Classification returned by the algebraic friction oracle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContactClassification {
    /// Static friction exactly balances the applied tangential force.
    Stick(BranchResponse),
    /// Established positive or negative sliding under kinetic friction.
    Slip {
        direction: SlipDirection,
        response: BranchResponse,
    },
    /// Zero static and kinetic friction: the frictionless limit.
    Frictionless {
        response: BranchResponse,
    },
    /// Exact threshold. Both static and incipient-slip branch responses are
    /// retained, with a mathematically typed derivative status.
    Boundary {
        direction: SlipDirection,
        stick: BranchResponse,
        incipient_slip: BranchResponse,
        sensitivity: BoundarySensitivity,
    },
    /// The force lies inside the caller-declared uncertainty band around the
    /// threshold but is not exactly equal to it. The oracle abstains rather
    /// than inventing a branch or silently applying a tolerance as physics.
    NearBoundary {
        direction: SlipDirection,
        signed_excess_force_n: f64,
        force_tolerance_n: f64,
    },
}

/// Input-domain or numerical failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoulombContactError {
    NonFiniteMass,
    NonPositiveMass,
    NonFiniteNormalForce,
    NonPositiveNormalForce,
    NonFiniteStaticCoefficient,
    NegativeStaticCoefficient,
    NonFiniteKineticCoefficient,
    NegativeKineticCoefficient,
    KineticExceedsStatic,
    NonFiniteForceTolerance,
    NegativeForceTolerance,
    NonFiniteAppliedForce,
    NumericalOverflow,
}

impl CoulombContact1D {
    /// Construct and validate a friction model.
    ///
    /// A zero static coefficient is permitted only with zero kinetic
    /// coefficient, representing the frictionless limit.
    pub fn new(
        mass_kg: f64,
        normal_force_n: f64,
        mu_static: f64,
        mu_kinetic: f64,
        force_tolerance_n: f64,
    ) -> Result<Self, CoulombContactError> {
        if !mass_kg.is_finite() {
            return Err(CoulombContactError::NonFiniteMass);
        }
        if mass_kg <= 0.0 {
            return Err(CoulombContactError::NonPositiveMass);
        }
        if !normal_force_n.is_finite() {
            return Err(CoulombContactError::NonFiniteNormalForce);
        }
        if normal_force_n <= 0.0 {
            return Err(CoulombContactError::NonPositiveNormalForce);
        }
        if !mu_static.is_finite() {
            return Err(CoulombContactError::NonFiniteStaticCoefficient);
        }
        if mu_static < 0.0 {
            return Err(CoulombContactError::NegativeStaticCoefficient);
        }
        if !mu_kinetic.is_finite() {
            return Err(CoulombContactError::NonFiniteKineticCoefficient);
        }
        if mu_kinetic < 0.0 {
            return Err(CoulombContactError::NegativeKineticCoefficient);
        }
        if mu_kinetic > mu_static {
            return Err(CoulombContactError::KineticExceedsStatic);
        }
        if !force_tolerance_n.is_finite() {
            return Err(CoulombContactError::NonFiniteForceTolerance);
        }
        if force_tolerance_n < 0.0 {
            return Err(CoulombContactError::NegativeForceTolerance);
        }

        Ok(Self {
            mass_kg,
            normal_force_n,
            // Normalize signed zero so semantically equivalent configurations
            // have byte-identical identities.
            mu_static: if mu_static == 0.0 { 0.0 } else { mu_static },
            mu_kinetic: if mu_kinetic == 0.0 { 0.0 } else { mu_kinetic },
            force_tolerance_n: if force_tolerance_n == 0.0 {
                0.0
            } else {
                force_tolerance_n
            },
        })
    }

    /// Return deterministic model-identity bytes.
    ///
    /// Every physical parameter and the explicit classification tolerance is
    /// included. The caller may hash these bytes with its chosen digest.
    pub fn canonical_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(COULOMB_STICK_SLIP_SCHEMA.len() + 40);
        bytes.extend_from_slice(COULOMB_STICK_SLIP_SCHEMA.as_bytes());
        bytes.push(0);
        for value in [
            self.mass_kg,
            self.normal_force_n,
            self.mu_static,
            self.mu_kinetic,
            self.force_tolerance_n,
        ] {
            bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        bytes
    }

    /// Classify the static/kinetic branch at a fixed applied tangential force.
    ///
    /// Strictly inside the static cone, only the stick response is returned.
    /// Strictly outside it, only the matching slip response is returned.
    /// At exact equality, both branches are preserved. A nonzero force offset
    /// inside the declared numerical tolerance produces NearBoundary and
    /// abstains; it is not promoted to exact mathematical equality.
    pub fn classify(
        self,
        applied_force_n: f64,
    ) -> Result<ContactClassification, CoulombContactError> {
        if !applied_force_n.is_finite() {
            return Err(CoulombContactError::NonFiniteAppliedForce);
        }

        if self.mu_static == 0.0 {
            let acceleration = checked(applied_force_n / self.mass_kg)?;
            return Ok(ContactClassification::Frictionless {
                response: BranchResponse {
                    friction_force_n: 0.0,
                    acceleration_m_s2: acceleration,
                },
            });
        }

        let static_limit = checked(self.mu_static * self.normal_force_n)?;
        let absolute_force = applied_force_n.abs();
        let signed_excess = checked(absolute_force - static_limit)?;
        let direction = if applied_force_n.is_sign_negative() {
            SlipDirection::Negative
        } else {
            SlipDirection::Positive
        };

        if signed_excess == 0.0 {
            let stick = BranchResponse {
                friction_force_n: -applied_force_n,
                acceleration_m_s2: 0.0,
            };
            let kinetic_friction = checked(
                direction.sign() * self.mu_kinetic * self.normal_force_n,
            )?;
            let incipient_acceleration =
                checked((applied_force_n - kinetic_friction) / self.mass_kg)?;
            let incipient_slip = BranchResponse {
                friction_force_n: -kinetic_friction,
                acceleration_m_s2: incipient_acceleration,
            };

            let sensitivity = if self.mu_static == self.mu_kinetic {
                BoundarySensitivity::ContinuousKink {
                    stick_side_da_dforce: 0.0,
                    slip_side_da_dforce: 1.0 / self.mass_kg,
                }
            } else {
                let jump = checked(
                    direction.sign()
                        * (self.mu_static - self.mu_kinetic)
                        * self.normal_force_n
                        / self.mass_kg,
                )?;
                BoundarySensitivity::Discontinuous {
                    signed_acceleration_jump_m_s2: jump,
                }
            };

            return Ok(ContactClassification::Boundary {
                direction,
                stick,
                incipient_slip,
                sensitivity,
            });
        }

        if signed_excess.abs() <= self.force_tolerance_n {
            return Ok(ContactClassification::NearBoundary {
                direction,
                signed_excess_force_n: signed_excess,
                force_tolerance_n: self.force_tolerance_n,
            });
        }

        if signed_excess < 0.0 {
            return Ok(ContactClassification::Stick(BranchResponse {
                friction_force_n: -applied_force_n,
                acceleration_m_s2: 0.0,
            }));
        }

        let kinetic_friction =
            checked(direction.sign() * self.mu_kinetic * self.normal_force_n)?;
        let acceleration = checked((applied_force_n - kinetic_friction) / self.mass_kg)?;
        Ok(ContactClassification::Slip {
            direction,
            response: BranchResponse {
                friction_force_n: -kinetic_friction,
                acceleration_m_s2: acceleration,
            },
        })
    }
}

fn checked(value: f64) -> Result<f64, CoulombContactError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CoulombContactError::NumericalOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(mu_static: f64, mu_kinetic: f64, tolerance: f64) -> CoulombContact1D {
        CoulombContact1D::new(2.0, 10.0, mu_static, mu_kinetic, tolerance)
            .expect("valid Coulomb fixture")
    }

    #[test]
    fn strict_interior_of_static_cone_is_stick() {
        let result = model(0.4, 0.2, 0.0)
            .classify(3.0)
            .expect("finite force");
        assert_eq!(
            result,
            ContactClassification::Stick(BranchResponse {
                friction_force_n: -3.0,
                acceleration_m_s2: 0.0,
            })
        );
    }

    #[test]
    fn positive_and_negative_overload_select_opposite_slip_branches() {
        let contact = model(0.4, 0.2, 0.0);
        let positive = contact.classify(5.0).expect("positive overload");
        let negative = contact.classify(-5.0).expect("negative overload");

        assert!(matches!(
            positive,
            ContactClassification::Slip {
                direction: SlipDirection::Positive,
                response: BranchResponse {
                    friction_force_n: -2.0,
                    acceleration_m_s2: 1.5,
                },
            }
        ));
        assert!(matches!(
            negative,
            ContactClassification::Slip {
                direction: SlipDirection::Negative,
                response: BranchResponse {
                    friction_force_n: 2.0,
                    acceleration_m_s2: -1.5,
                },
            }
        ));
    }

    #[test]
    fn equal_coefficients_preserve_two_one_sided_derivatives_at_boundary() {
        let result = model(0.4, 0.4, 0.0)
            .classify(4.0)
            .expect("exact boundary");
        assert_eq!(
            result,
            ContactClassification::Boundary {
                direction: SlipDirection::Positive,
                stick: BranchResponse {
                    friction_force_n: -4.0,
                    acceleration_m_s2: 0.0,
                },
                incipient_slip: BranchResponse {
                    friction_force_n: -4.0,
                    acceleration_m_s2: 0.0,
                },
                sensitivity: BoundarySensitivity::ContinuousKink {
                    stick_side_da_dforce: 0.0,
                    slip_side_da_dforce: 0.5,
                },
            }
        );
    }

    #[test]
    fn static_kinetic_gap_is_a_discontinuity_not_an_averaged_gradient() {
        let result = model(0.4, 0.2, 0.0)
            .classify(4.0)
            .expect("exact boundary");
        assert_eq!(
            result,
            ContactClassification::Boundary {
                direction: SlipDirection::Positive,
                stick: BranchResponse {
                    friction_force_n: -4.0,
                    acceleration_m_s2: 0.0,
                },
                incipient_slip: BranchResponse {
                    friction_force_n: -2.0,
                    acceleration_m_s2: 1.0,
                },
                sensitivity: BoundarySensitivity::Discontinuous {
                    signed_acceleration_jump_m_s2: 1.0,
                },
            }
        );
    }

    #[test]
    fn negative_boundary_preserves_signed_jump() {
        let result = model(0.4, 0.2, 0.0)
            .classify(-4.0)
            .expect("exact negative boundary");
        assert!(matches!(
            result,
            ContactClassification::Boundary {
                direction: SlipDirection::Negative,
                sensitivity: BoundarySensitivity::Discontinuous {
                    signed_acceleration_jump_m_s2: -1.0,
                },
                ..
            }
        ));
    }

    #[test]
    fn tolerance_band_abstains_instead_of_fabricating_exact_boundary() {
        let result = model(0.4, 0.2, 0.1)
            .classify(4.05)
            .expect("near-boundary force");
        assert_eq!(
            result,
            ContactClassification::NearBoundary {
                direction: SlipDirection::Positive,
                signed_excess_force_n: 0.04999999999999982,
                force_tolerance_n: 0.1,
            }
        );
    }

    #[test]
    fn tolerance_is_part_of_canonical_model_identity() {
        let tight = model(0.4, 0.2, 1.0e-6);
        let loose = model(0.4, 0.2, 1.0e-3);
        assert_ne!(tight.canonical_bytes(), loose.canonical_bytes());
    }

    #[test]
    fn signed_zero_coefficients_have_canonical_identity() {
        let positive_zero = model(0.0, 0.0, 0.0);
        let negative_zero = CoulombContact1D::new(2.0, 10.0, -0.0, -0.0, -0.0)
            .expect("negative zero is mathematically zero");
        assert_eq!(positive_zero.canonical_bytes(), negative_zero.canonical_bytes());
    }

    #[test]
    fn frictionless_limit_has_no_fictitious_static_branch() {
        let result = model(0.0, 0.0, 0.0)
            .classify(6.0)
            .expect("frictionless force");
        assert_eq!(
            result,
            ContactClassification::Frictionless {
                response: BranchResponse {
                    friction_force_n: 0.0,
                    acceleration_m_s2: 3.0,
                },
            }
        );
    }

    #[test]
    fn rejects_invalid_parameter_domains_and_non_finite_force() {
        assert_eq!(
            CoulombContact1D::new(0.0, 10.0, 0.4, 0.2, 0.0),
            Err(CoulombContactError::NonPositiveMass)
        );
        assert_eq!(
            CoulombContact1D::new(2.0, 0.0, 0.4, 0.2, 0.0),
            Err(CoulombContactError::NonPositiveNormalForce)
        );
        assert_eq!(
            CoulombContact1D::new(2.0, 10.0, 0.2, 0.4, 0.0),
            Err(CoulombContactError::KineticExceedsStatic)
        );
        assert_eq!(
            CoulombContact1D::new(2.0, 10.0, 0.4, 0.2, -0.1),
            Err(CoulombContactError::NegativeForceTolerance)
        );
        assert_eq!(
            model(0.4, 0.2, 0.0).classify(f64::NAN),
            Err(CoulombContactError::NonFiniteAppliedForce)
        );
    }
}
