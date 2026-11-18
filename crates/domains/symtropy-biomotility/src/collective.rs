// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Event-level collision mechanosensing and collective-order statistics.
//!
//! This module models the sensory response to an already identified cell-cell
//! contact. It does not generate contacts, solve rod collisions, infer species,
//! or derive contact incidence from density. Contact generation and collective
//! mechanics remain separate qualification layers.

use std::error::Error;
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;

use super::{EventEvidenceStatus, SeededRng};

const MIN_CONTACT_ANGLE_RAD: f64 = PI / 6.0;
const ANGLE_GATE_WIDTH_RAD: f64 = FRAC_PI_2 - MIN_CONTACT_ANGLE_RAD;
const LN_2: f64 = std::f64::consts::LN_2;
const MAX_CONSTITUTIVE_REVERSALS: usize = 1_000_000;

/// One already-detected cell-cell contact, with angle and duration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionContact {
    /// Canonical contact angle in radians, from parallel (0) to perpendicular (pi/2).
    pub contact_angle_rad: f64,
    /// Duration for which the mechanosensory contact is maintained.
    pub contact_duration_s: f64,
}

impl CollisionContact {
    pub fn validate(&self) -> Result<(), CollisionResponseError> {
        if !self.contact_angle_rad.is_finite()
            || !(0.0..=FRAC_PI_2).contains(&self.contact_angle_rad)
        {
            return Err(CollisionResponseError::InvalidContactAngle);
        }
        if !self.contact_duration_s.is_finite() || self.contact_duration_s < 0.0 {
            return Err(CollisionResponseError::InvalidContactDuration);
        }
        Ok(())
    }
}

/// Characteristic collision-response time for a declared phenotype / condition.
///
/// The duration response is an exponential saturation, scaled by a
/// probability-linear angular gate. At a perpendicular contact (pi/2),
/// contact duration equal to characteristic_time_s gives exactly 0.5
/// reversal probability. The continuous duration interpolation is a model
/// assumption constrained by the source anchor, not a fully fitted law.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionReversalLaw {
    pub characteristic_time_s: f64,
}

impl CollisionReversalLaw {
    pub fn validate(&self) -> Result<(), CollisionResponseError> {
        if !self.characteristic_time_s.is_finite() || self.characteristic_time_s <= 0.0 {
            return Err(CollisionResponseError::InvalidCharacteristicTime);
        }
        Ok(())
    }

    /// Probability of reversal for one measured/modelled contact.
    ///
    /// The angular gate scales probability linearly from zero at/below pi/6
    /// to one at pi/2. The duration response is 1 - exp(-ln(2) * t / tau),
    /// so at pi/2 and t=tau the probability is exactly 0.5.
    pub fn reversal_probability(
        &self,
        contact: CollisionContact,
    ) -> Result<f64, CollisionResponseError> {
        self.validate()?;
        contact.validate()?;

        let angle_gate = ((contact.contact_angle_rad - MIN_CONTACT_ANGLE_RAD)
            / ANGLE_GATE_WIDTH_RAD)
            .clamp(0.0, 1.0);
        if angle_gate == 0.0 || contact.contact_duration_s == 0.0 {
            return Ok(0.0);
        }

        let duration_response =
            -(-LN_2 * (contact.contact_duration_s / self.characteristic_time_s)).exp_m1();
        Ok((angle_gate * duration_response).clamp(0.0, 1.0))
    }

    /// Draw one reversal outcome using a deterministic local seed.
    pub fn sample_with_seed(
        &self,
        contact: CollisionContact,
        seed: u64,
    ) -> Result<bool, CollisionResponseError> {
        let probability = self.reversal_probability(contact)?;
        let mut rng = SeededRng::new(seed);
        Ok(rng.next_unit_f64() < probability)
    }

    /// Draw outcomes for a fixed sequence of contacts using one seeded stream.
    pub fn sample_many(
        &self,
        contacts: &[CollisionContact],
        seed: u64,
    ) -> Result<Vec<bool>, CollisionResponseError> {
        self.validate()?;
        for contact in contacts {
            contact.validate()?;
        }

        let mut rng = SeededRng::new(seed);
        contacts
            .iter()
            .map(|contact| {
                let probability = self.reversal_probability(*contact)?;
                Ok(rng.next_unit_f64() < probability)
            })
            .collect()
    }
}

/// Contact-response phenotype. Constitutive reversal is deliberately
/// modelled as a separate time process below, never as a contact response.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReversalPhenotype {
    ContactSensitive(CollisionReversalLaw),
    NonReversing,
}

impl ReversalPhenotype {
    pub fn contact_reversal_probability(
        &self,
        contact: CollisionContact,
    ) -> Result<f64, CollisionResponseError> {
        match self {
            Self::ContactSensitive(law) => law.reversal_probability(contact),
            Self::NonReversing => {
                contact.validate()?;
                Ok(0.0)
            }
        }
    }

    pub fn sample_contact_with_seed(
        &self,
        contact: CollisionContact,
        seed: u64,
    ) -> Result<bool, CollisionResponseError> {
        let probability = self.contact_reversal_probability(contact)?;
        let mut rng = SeededRng::new(seed);
        Ok(rng.next_unit_f64() < probability)
    }
}

/// Independent, constitutive reversal process with a constant event rate.
///
/// The generated event times form a Poisson process and do not depend on the
/// number, angle or duration of cell-cell contacts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstitutiveReversalLaw {
    pub rate_hz: f64,
}

impl ConstitutiveReversalLaw {
    pub fn validate(&self) -> Result<(), CollisionResponseError> {
        if !self.rate_hz.is_finite() || self.rate_hz < 0.0 {
            return Err(CollisionResponseError::InvalidConstitutiveRate);
        }
        Ok(())
    }

    pub fn event_times_with_seed(
        &self,
        observation_time_s: f64,
        seed: u64,
    ) -> Result<Vec<f64>, CollisionResponseError> {
        self.validate()?;
        if !observation_time_s.is_finite() || observation_time_s < 0.0 {
            return Err(CollisionResponseError::InvalidObservationTime);
        }
        if self.rate_hz == 0.0 || observation_time_s == 0.0 {
            return Ok(Vec::new());
        }

        let mut rng = SeededRng::new(seed);
        let mut time_s = 0.0;
        let mut event_times = Vec::new();
        loop {
            let unit = rng
                .next_unit_f64()
                .clamp(f64::EPSILON, 1.0 - f64::EPSILON);
            let inter_event_s = -(-unit).ln_1p() / self.rate_hz;
            if !inter_event_s.is_finite() || inter_event_s <= 0.0 {
                return Err(CollisionResponseError::InvalidConstitutiveRate);
            }
            time_s += inter_event_s;
            if time_s > observation_time_s {
                break;
            }
            event_times.push(time_s);
            if event_times.len() >= MAX_CONSTITUTIVE_REVERSALS {
                return Err(CollisionResponseError::ConstitutiveEventLimitExceeded);
            }
        }
        Ok(event_times)
    }
}

/// One contact outcome plus the evidence status of the response label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionResponseObservation {
    pub contact: CollisionContact,
    pub reversal_observed: bool,
    pub evidence_status: EventEvidenceStatus,
}

/// Separate contact incidence from per-contact response and overall reversal rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionResponseSummary {
    pub total_contact_count: usize,
    pub evidence_qualified_contact_count: usize,
    pub excluded_contact_count: usize,
    pub reversal_count: usize,
    pub non_reversal_count: usize,
    /// Sum of observed cell-time across all cells, measured in cell-seconds.
    pub cell_time_exposure_s: f64,
    /// All detected contacts divided by total cell-time exposure.
    pub contact_incidence_rate_per_cell_second: Option<f64>,
    /// Evidence-qualified contact-response observations divided by cell-time.
    pub evidence_qualified_contact_incidence_rate_per_cell_second: Option<f64>,
    /// Fraction of evidence-qualified contacts that produced a reversal.
    pub per_contact_reversal_fraction: Option<f64>,
    /// Evidence-qualified reversals divided by total cell-time exposure.
    pub reversal_rate_per_cell_second: Option<f64>,
}

impl CollisionResponseSummary {
    /// Summarize contacts using total tracked cell-time, not wall-clock time.
    ///
    /// `cell_time_exposure_s` is the sum of valid observation durations across
    /// all cells in the population. This normalizes incidence across population
    /// sizes. A single recording duration is not a valid denominator when the
    /// number of observed cells changes.
    pub fn from_observations(
        observations: &[CollisionResponseObservation],
        cell_time_exposure_s: f64,
    ) -> Result<Self, CollisionResponseError> {
        if !cell_time_exposure_s.is_finite() || cell_time_exposure_s <= 0.0 {
            return Err(CollisionResponseError::InvalidCellTimeExposure);
        }
        for observation in observations {
            observation.contact.validate()?;
        }

        let mut evidence_qualified_contact_count = 0;
        let mut excluded_contact_count = 0;
        let mut reversal_count = 0;
        let mut non_reversal_count = 0;

        for observation in observations {
            match observation.evidence_status {
                EventEvidenceStatus::DirectlyObserved | EventEvidenceStatus::ModelLatent => {
                    evidence_qualified_contact_count += 1;
                    if observation.reversal_observed {
                        reversal_count += 1;
                    } else {
                        non_reversal_count += 1;
                    }
                }
                EventEvidenceStatus::AlgorithmInferred | EventEvidenceStatus::Unresolved => {
                    excluded_contact_count += 1;
                }
            }
        }

        let contact_incidence_rate_per_cell_second =
            Some(observations.len() as f64 / cell_time_exposure_s);
        let evidence_qualified_contact_incidence_rate_per_cell_second =
            (evidence_qualified_contact_count > 0)
                .then(|| evidence_qualified_contact_count as f64 / cell_time_exposure_s);
        let per_contact_reversal_fraction = (evidence_qualified_contact_count > 0)
            .then(|| reversal_count as f64 / evidence_qualified_contact_count as f64);
        let reversal_rate_per_cell_second = if evidence_qualified_contact_count > 0
            || observations.is_empty()
        {
            Some(reversal_count as f64 / cell_time_exposure_s)
        } else {
            None
        };

        Ok(Self {
            total_contact_count: observations.len(),
            evidence_qualified_contact_count,
            excluded_contact_count,
            reversal_count,
            non_reversal_count,
            cell_time_exposure_s,
            contact_incidence_rate_per_cell_second,
            evidence_qualified_contact_incidence_rate_per_cell_second,
            per_contact_reversal_fraction,
            reversal_rate_per_cell_second,
        })
    }
}

/// Polar and nematic order parameters for 2D orientation samples.
///
/// Polar order distinguishes head-tail direction. Nematic order treats
/// theta and theta+pi as equivalent, as appropriate for unoriented rod axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientationOrder {
    pub sample_count: usize,
    pub polar_order: Option<f64>,
    pub polar_mean_heading_rad: Option<f64>,
    pub nematic_order: Option<f64>,
    pub nematic_axis_rad: Option<f64>,
}

/// Compute population polar and nematic order without assuming any interaction law.
pub fn orientation_order(
    headings_rad: &[f64],
) -> Result<OrientationOrder, CollisionResponseError> {
    if headings_rad.iter().any(|heading| !heading.is_finite()) {
        return Err(CollisionResponseError::NonFiniteHeading);
    }
    if headings_rad.is_empty() {
        return Ok(OrientationOrder {
            sample_count: 0,
            polar_order: None,
            polar_mean_heading_rad: None,
            nematic_order: None,
            nematic_axis_rad: None,
        });
    }

    let n = headings_rad.len() as f64;
    let (polar_x, polar_y, nematic_x, nematic_y) = headings_rad.iter().fold(
        (0.0, 0.0, 0.0, 0.0),
        |(px, py, nx, ny), heading| {
            (
                px + heading.cos(),
                py + heading.sin(),
                nx + (2.0 * heading).cos(),
                ny + (2.0 * heading).sin(),
            )
        },
    );

    let polar_x = polar_x / n;
    let polar_y = polar_y / n;
    let nematic_x = nematic_x / n;
    let nematic_y = nematic_y / n;
    let polar_order = polar_x.hypot(polar_y).clamp(0.0, 1.0);
    let nematic_order = nematic_x.hypot(nematic_y).clamp(0.0, 1.0);

    Ok(OrientationOrder {
        sample_count: headings_rad.len(),
        polar_order: Some(polar_order),
        polar_mean_heading_rad: (polar_order > f64::EPSILON)
            .then(|| polar_y.atan2(polar_x)),
        nematic_order: Some(nematic_order),
        nematic_axis_rad: (nematic_order > f64::EPSILON)
            .then(|| 0.5 * nematic_y.atan2(nematic_x)),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionResponseError {
    InvalidCharacteristicTime,
    InvalidContactAngle,
    InvalidContactDuration,
    InvalidCellTimeExposure,
    InvalidConstitutiveRate,
    ConstitutiveEventLimitExceeded,
    NonFiniteHeading,
}

impl fmt::Display for CollisionResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCharacteristicTime => {
                write!(formatter, "collision characteristic time must be finite and positive")
            }
            Self::InvalidContactAngle => {
                write!(formatter, "contact angle must be finite and in [0, pi/2]")
            }
            Self::InvalidContactDuration => {
                write!(formatter, "contact duration must be finite and non-negative")
            }
            Self::InvalidCellTimeExposure => {
                write!(formatter, "cell-time exposure must be finite and positive")
            }
            Self::InvalidConstitutiveRate => {
                write!(formatter, "constitutive reversal rate must be finite and non-negative")
            }
            Self::ConstitutiveEventLimitExceeded => {
                write!(formatter, "constitutive reversal sample exceeded its event budget")
            }
            Self::NonFiniteHeading => {
                write!(formatter, "orientation headings must be finite")
            }
        }
    }
}

impl Error for CollisionResponseError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn law() -> CollisionReversalLaw {
        CollisionReversalLaw {
            characteristic_time_s: 2.0,
        }
    }

    #[test]
    fn perpendicular_contact_over_characteristic_time_has_half_reversal_probability() {
        let contact = CollisionContact {
            contact_angle_rad: FRAC_PI_2,
            contact_duration_s: 2.0,
        };
        assert!((law().reversal_probability(contact).unwrap() - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn shallow_contact_at_or_below_pi_over_six_has_zero_reversal_probability() {
        for angle in [0.0, MIN_CONTACT_ANGLE_RAD - 1.0e-9, MIN_CONTACT_ANGLE_RAD] {
            let contact = CollisionContact {
                contact_angle_rad: angle,
                contact_duration_s: 100.0,
            };
            assert_eq!(law().reversal_probability(contact), Ok(0.0));
        }
    }

    #[test]
    fn angular_gate_is_linear_between_pi_over_six_and_pi_over_two() {
        let duration = 2.0;
        let at_cutoff = law()
            .reversal_probability(CollisionContact {
                contact_angle_rad: MIN_CONTACT_ANGLE_RAD,
                contact_duration_s: duration,
            })
            .unwrap();
        let at_midpoint = law()
            .reversal_probability(CollisionContact {
                contact_angle_rad: PI / 3.0,
                contact_duration_s: duration,
            })
            .unwrap();
        let at_perpendicular = law()
            .reversal_probability(CollisionContact {
                contact_angle_rad: FRAC_PI_2,
                contact_duration_s: duration,
            })
            .unwrap();
        assert_eq!(at_cutoff, 0.0);
        assert!((at_midpoint - 0.25).abs() < 1.0e-12);
        assert!((at_perpendicular - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn duration_response_is_monotonic_and_converges_to_one() {
        let probabilities: Vec<_> = [0.0, 0.5, 1.0, 2.0, 10.0, 100.0]
            .iter()
            .map(|duration| {
                law()
                    .reversal_probability(CollisionContact {
                        contact_angle_rad: FRAC_PI_2,
                        contact_duration_s: *duration,
                    })
                    .unwrap()
            })
            .collect();
        assert!(probabilities.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!((probabilities[3] - 0.5).abs() < 1.0e-12);
        assert!(probabilities[5] > 0.999999);
    }

    #[test]
    fn seeded_contact_response_replays_exactly() {
        let contacts = [
            CollisionContact {
                contact_angle_rad: FRAC_PI_2,
                contact_duration_s: 2.0,
            },
            CollisionContact {
                contact_angle_rad: PI / 4.0,
                contact_duration_s: 1.0,
            },
            CollisionContact {
                contact_angle_rad: MIN_CONTACT_ANGLE_RAD,
                contact_duration_s: 10.0,
            },
        ];
        assert_eq!(
            law().sample_many(&contacts, 42),
            law().sample_many(&contacts, 42)
        );
    }

    #[test]
    fn summary_separates_contact_incidence_from_per_contact_response() {
        let observations = [
            CollisionResponseObservation {
                contact: CollisionContact {
                    contact_angle_rad: FRAC_PI_2,
                    contact_duration_s: 2.0,
                },
                reversal_observed: true,
                evidence_status: EventEvidenceStatus::DirectlyObserved,
            },
            CollisionResponseObservation {
                contact: CollisionContact {
                    contact_angle_rad: FRAC_PI_2,
                    contact_duration_s: 2.0,
                },
                reversal_observed: false,
                evidence_status: EventEvidenceStatus::ModelLatent,
            },
            CollisionResponseObservation {
                contact: CollisionContact {
                    contact_angle_rad: FRAC_PI_2,
                    contact_duration_s: 2.0,
                },
                reversal_observed: true,
                evidence_status: EventEvidenceStatus::AlgorithmInferred,
            },
        ];
        let summary =
            CollisionResponseSummary::from_observations(&observations, 4.0).unwrap();
        assert_eq!(summary.total_contact_count, 3);
        assert_eq!(summary.evidence_qualified_contact_count, 2);
        assert_eq!(summary.excluded_contact_count, 1);
        assert_eq!(summary.reversal_count, 1);
        assert_eq!(summary.non_reversal_count, 1);
        assert_eq!(summary.contact_incidence_rate_per_cell_second, Some(0.375));
        assert_eq!(summary.per_contact_reversal_fraction, Some(0.5));
        assert_eq!(summary.reversal_rate_per_cell_second, Some(0.125));
    }

    #[test]
    fn unresolved_contact_outcomes_do_not_become_non_reversals() {
        let observations = [CollisionResponseObservation {
            contact: CollisionContact {
                contact_angle_rad: FRAC_PI_2,
                contact_duration_s: 2.0,
            },
            reversal_observed: false,
            evidence_status: EventEvidenceStatus::Unresolved,
        }];
        let summary =
            CollisionResponseSummary::from_observations(&observations, 2.0).unwrap();
        assert_eq!(summary.total_contact_count, 1);
        assert_eq!(summary.excluded_contact_count, 1);
        assert_eq!(summary.evidence_qualified_contact_count, 0);
        assert_eq!(summary.evidence_qualified_contact_incidence_rate_per_cell_second, None);
        assert_eq!(summary.per_contact_reversal_fraction, None);
        assert_eq!(summary.reversal_rate_per_cell_second, None);
    }

    #[test]
    fn orientation_order_distinguishes_polar_and_nematic_alignment() {
        let parallel_and_antiparallel = [0.0, PI];
        let nematic = orientation_order(&parallel_and_antiparallel).unwrap();
        assert!(nematic.polar_order.unwrap() < 1.0e-12);
        assert!((nematic.nematic_order.unwrap() - 1.0).abs() < 1.0e-12);

        let parallel = [0.0, 0.0];
        let polar = orientation_order(&parallel).unwrap();
        assert!((polar.polar_order.unwrap() - 1.0).abs() < 1.0e-12);
        assert!((polar.nematic_order.unwrap() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn orientation_order_is_invariant_to_pi_rotation_and_rejects_non_finite_values() {
        let headings = [0.1, 0.3, 1.2, 2.9];
        let shifted: Vec<_> = headings.iter().map(|heading| heading + PI).collect();
        let first = orientation_order(&headings).unwrap();
        let second = orientation_order(&shifted).unwrap();
        assert!((first.nematic_order.unwrap() - second.nematic_order.unwrap()).abs() < 1.0e-12);
        assert!(orientation_order(&[0.0, f64::NAN]).is_err());
    }

    #[test]
    fn empty_orientation_set_returns_explicit_unknowns() {
        let order = orientation_order(&[]).unwrap();
        assert_eq!(order.sample_count, 0);
        assert_eq!(order.polar_order, None);
        assert_eq!(order.nematic_order, None);
    }

    #[test]
    fn non_reversing_phenotype_ignores_contact_exposure() {
        let phenotype = ReversalPhenotype::NonReversing;
        for contact in [
            CollisionContact {
                contact_angle_rad: FRAC_PI_2,
                contact_duration_s: 0.0,
            },
            CollisionContact {
                contact_angle_rad: FRAC_PI_2,
                contact_duration_s: 10_000.0,
            },
            CollisionContact {
                contact_angle_rad: MIN_CONTACT_ANGLE_RAD,
                contact_duration_s: 10_000.0,
            },
        ] {
            assert_eq!(phenotype.contact_reversal_probability(contact), Ok(0.0));
            assert!(!phenotype.sample_contact_with_seed(contact, 8).unwrap());
        }
    }

    #[test]
    fn constitutive_reversals_are_reproducible_and_independent_of_contacts() {
        let law = ConstitutiveReversalLaw { rate_hz: 2.0 };
        let first = law.event_times_with_seed(10.0, 11).unwrap();
        let replay = law.event_times_with_seed(10.0, 11).unwrap();
        assert_eq!(first, replay);
        assert!(first.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(first.iter().all(|time| *time > 0.0 && *time <= 10.0));
        assert_eq!(
            ConstitutiveReversalLaw { rate_hz: 0.0 }
                .event_times_with_seed(10.0, 11)
                .unwrap(),
            Vec::<f64>::new()
        );
    }

    #[test]
    fn contact_incidence_changes_population_reversal_rate_without_changing_contact_law() {
        let contact = CollisionContact {
            contact_angle_rad: FRAC_PI_2,
            contact_duration_s: 2.0,
        };
        let one = [CollisionResponseObservation {
            contact,
            reversal_observed: true,
            evidence_status: EventEvidenceStatus::DirectlyObserved,
        }];
        let four = [
            one[0],
            one[0],
            one[0],
            one[0],
        ];
        let one_summary = CollisionResponseSummary::from_observations(&one, 4.0).unwrap();
        let four_summary = CollisionResponseSummary::from_observations(&four, 4.0).unwrap();

        assert_eq!(law().reversal_probability(contact).unwrap(), 0.5);
        assert_eq!(one_summary.per_contact_reversal_fraction, Some(1.0));
        assert_eq!(four_summary.per_contact_reversal_fraction, Some(1.0));
        assert_eq!(one_summary.contact_incidence_rate_per_cell_second, Some(0.25));
        assert_eq!(four_summary.contact_incidence_rate_per_cell_second, Some(1.0));
        assert_eq!(one_summary.reversal_rate_per_cell_second, Some(0.25));
        assert_eq!(four_summary.reversal_rate_per_cell_second, Some(1.0));
    }

    #[test]
    fn invalid_characteristic_time_and_contact_fields_fail_closed() {
        assert_eq!(
            CollisionReversalLaw {
                characteristic_time_s: 0.0
            }
            .validate(),
            Err(CollisionResponseError::InvalidCharacteristicTime)
        );
        assert_eq!(
            law().reversal_probability(CollisionContact {
                contact_angle_rad: f64::NAN,
                contact_duration_s: 1.0,
            }),
            Err(CollisionResponseError::InvalidContactAngle)
        );
        assert_eq!(
            law().reversal_probability(CollisionContact {
                contact_angle_rad: FRAC_PI_2,
                contact_duration_s: -1.0,
            }),
            Err(CollisionResponseError::InvalidContactDuration)
        );
    }
}
