// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent equation-level reference for BMX passive inter-segment exchange.
//!
//! Source: PNNL-CompBio/BoltzmannMFX,
//! commit ad0bea280a9fb655e77e32db1f2931c2bea1fb28,
//! src/chemistry/bmx_chem_K.H, evaluateExchange, lines 1960-2077.
//! The external unit declaration for mt remains unverified; length/time is
//! inferred dimensionally. This is a clean-room synthetic reference, not a
//! biological calibration or copied BMX source.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BmxSegmentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmxChemicalSpecies {
    A,
    B,
    C,
}

impl BmxChemicalSpecies {
    const ALL: [Self; 3] = [Self::A, Self::B, Self::C];

    const fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
            Self::C => 2,
        }
    }
}

/// Exchange-relevant state for one fungal or non-fungal particle.
/// Synthetic geometry units: metres and cubic metres.
#[derive(Debug, Clone, PartialEq)]
pub struct BmxExchangeSegment {
    pub id: BmxSegmentId,
    pub is_fungus: bool,
    pub radius_m: f64,
    pub volume_m3: f64,
    pub concentrations: [f64; 3],
    pub connected_segment_ids: Vec<BmxSegmentId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmxExchangePair {
    /// A positive amount leaves first and enters second.
    pub first: BmxSegmentId,
    pub second: BmxSegmentId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BmxExchangeParameters {
    /// Species-specific mass-transfer coefficients, inferred units m/time.
    pub mt_m_per_time: [f64; 3],
    /// Same time unit used by the coefficients.
    pub dt_time_units: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmxSkipReason {
    NonFungalPair,
    MissingReciprocalBond,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BmxPairExchange {
    pub first: BmxSegmentId,
    pub second: BmxSegmentId,
    pub signed_amount_by_species: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct BmxSkippedPair {
    pub first: BmxSegmentId,
    pub second: BmxSegmentId,
    pub reason: BmxSkipReason,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BmxExchangeReport {
    pub pair_exchanges: Vec<BmxPairExchange>,
    pub skipped_pairs: Vec<BmxSkippedPair>,
    pub total_amount_before_by_species: [f64; 3],
    pub total_amount_after_by_species: [f64; 3],
    pub conservation_residual_by_species: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub enum BmxExchangeError {
    InvalidTimeStep,
    InvalidCoefficient(BmxChemicalSpecies),
    InvalidRadius(BmxSegmentId),
    InvalidVolume(BmxSegmentId),
    InvalidConcentration(BmxSegmentId, BmxChemicalSpecies),
    NonFiniteTotal(BmxChemicalSpecies),
    DuplicateSegment(BmxSegmentId),
    DuplicateBond(BmxSegmentId, BmxSegmentId),
    MissingSegment(BmxSegmentId),
    SameSegmentPair(BmxSegmentId),
    DuplicatePair(BmxSegmentId, BmxSegmentId),
    NonFiniteTransfer(BmxSegmentId, BmxSegmentId, BmxChemicalSpecies),
    NegativeFinalConcentration(BmxSegmentId, BmxChemicalSpecies),
}

impl fmt::Display for BmxExchangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeStep => {
                write!(f, "BMX reference time step must be finite and positive")
            }
            Self::InvalidCoefficient(species) => {
                write!(
                    f,
                    "BMX coefficient for {:?} must be finite and non-negative",
                    species
                )
            }
            Self::InvalidRadius(id) => {
                write!(f, "segment {:?} radius must be finite and positive", id)
            }
            Self::InvalidVolume(id) => {
                write!(f, "segment {:?} volume must be finite and positive", id)
            }
            Self::InvalidConcentration(id, species) => {
                write!(
                    f,
                    "segment {:?} concentration for {:?} must be finite and non-negative",
                    id, species
                )
            }
            Self::NonFiniteTotal(species) => {
                write!(f, "total amount for {:?} is not finite", species)
            }
            Self::DuplicateSegment(id) => write!(f, "duplicate segment {:?}", id),
            Self::DuplicateBond(a, b) => write!(f, "duplicate bond {:?} -> {:?}", a, b),
            Self::MissingSegment(id) => write!(f, "missing segment {:?}", id),
            Self::SameSegmentPair(id) => {
                write!(f, "exchange pair cannot connect segment {:?} to itself", id)
            }
            Self::DuplicatePair(a, b) => {
                write!(f, "duplicate unordered exchange pair {:?}/{:?}", a, b)
            }
            Self::NonFiniteTransfer(a, b, species) => {
                write!(
                    f,
                    "non-finite transfer {:?} -> {:?} for {:?}",
                    a, b, species
                )
            }
            Self::NegativeFinalConcentration(id, species) => {
                write!(
                    f,
                    "negative final concentration at {:?} for {:?}",
                    id, species
                )
            }
        }
    }
}

impl std::error::Error for BmxExchangeError {}

/// Reproduce the source's pairwise passive exchange and donor-share limiter.
///
/// A = 0.5 * (pi*r1^2 + pi*r2^2)
/// dM = (c1-c2) * A * mt * dt
///
/// Positive dM is capped by c1*V1/degree1; negative dM is capped by
/// -c2*V2/degree2. The operation accumulates increments from base
/// concentrations, then applies paired amount debit/credit updates.
///
/// Each unordered pair must appear at most once in pairs. Missing reciprocal
/// bonds and non-fungal pairs are skipped. All validation and settlement are
/// atomic: an error leaves the caller's segment slice unchanged.
pub fn apply_bmx_reference_exchange(
    segments: &mut [BmxExchangeSegment],
    pairs: &[BmxExchangePair],
    parameters: BmxExchangeParameters,
) -> Result<BmxExchangeReport, BmxExchangeError> {
    if !parameters.dt_time_units.is_finite() || parameters.dt_time_units <= 0.0 {
        return Err(BmxExchangeError::InvalidTimeStep);
    }
    for species in BmxChemicalSpecies::ALL {
        let coefficient = parameters.mt_m_per_time[species.index()];
        if !coefficient.is_finite() || coefficient < 0.0 {
            return Err(BmxExchangeError::InvalidCoefficient(species));
        }
    }

    let mut index_by_id = BTreeMap::new();
    let mut total_before = [0.0_f64; 3];
    for (index, segment) in segments.iter().enumerate() {
        if !segment.radius_m.is_finite() || segment.radius_m <= 0.0 {
            return Err(BmxExchangeError::InvalidRadius(segment.id));
        }
        if !segment.volume_m3.is_finite() || segment.volume_m3 <= 0.0 {
            return Err(BmxExchangeError::InvalidVolume(segment.id));
        }
        if index_by_id.insert(segment.id, index).is_some() {
            return Err(BmxExchangeError::DuplicateSegment(segment.id));
        }
        let mut seen_bonds = BTreeSet::new();
        for other in &segment.connected_segment_ids {
            if *other == segment.id || !seen_bonds.insert(*other) {
                return Err(BmxExchangeError::DuplicateBond(segment.id, *other));
            }
        }
        for species in BmxChemicalSpecies::ALL {
            let slot = species.index();
            let concentration = segment.concentrations[slot];
            if !concentration.is_finite() || concentration < 0.0 {
                return Err(BmxExchangeError::InvalidConcentration(segment.id, species));
            }
            total_before[slot] += concentration * segment.volume_m3;
            if !total_before[slot].is_finite() {
                return Err(BmxExchangeError::NonFiniteTotal(species));
            }
        }
    }

    let mut seen_pairs = BTreeSet::new();
    let mut concentration_delta = vec![[0.0_f64; 3]; segments.len()];
    let mut pair_exchanges = Vec::new();
    let mut skipped_pairs = Vec::new();

    for pair in pairs {
        if pair.first == pair.second {
            return Err(BmxExchangeError::SameSegmentPair(pair.first));
        }
        let first_key = pair.first.min(pair.second);
        let second_key = pair.first.max(pair.second);
        if !seen_pairs.insert((first_key, second_key)) {
            return Err(BmxExchangeError::DuplicatePair(first_key, second_key));
        }

        let first_index = *index_by_id
            .get(&pair.first)
            .ok_or(BmxExchangeError::MissingSegment(pair.first))?;
        let second_index = *index_by_id
            .get(&pair.second)
            .ok_or(BmxExchangeError::MissingSegment(pair.second))?;
        let first = &segments[first_index];
        let second = &segments[second_index];

        if !first.is_fungus || !second.is_fungus {
            skipped_pairs.push(BmxSkippedPair {
                first: pair.first,
                second: pair.second,
                reason: BmxSkipReason::NonFungalPair,
            });
            continue;
        }
        if !first.connected_segment_ids.contains(&pair.second)
            || !second.connected_segment_ids.contains(&pair.first)
        {
            skipped_pairs.push(BmxSkippedPair {
                first: pair.first,
                second: pair.second,
                reason: BmxSkipReason::MissingReciprocalBond,
            });
            continue;
        }

        let first_degree = first.connected_segment_ids.len() as f64;
        let second_degree = second.connected_segment_ids.len() as f64;
        let area_m2 = 0.5
            * (std::f64::consts::PI * first.radius_m * first.radius_m
                + std::f64::consts::PI * second.radius_m * second.radius_m);
        let mut signed_amount = [0.0_f64; 3];

        for species in BmxChemicalSpecies::ALL {
            let slot = species.index();
            let raw = (first.concentrations[slot] - second.concentrations[slot])
                * area_m2
                * parameters.mt_m_per_time[slot]
                * parameters.dt_time_units;
            if !raw.is_finite() {
                return Err(BmxExchangeError::NonFiniteTransfer(
                    pair.first,
                    pair.second,
                    species,
                ));
            }

            let mut applied = raw;
            if applied / first.volume_m3 > first.concentrations[slot] / first_degree {
                applied = first.concentrations[slot] * first.volume_m3 / first_degree;
            }
            if -applied / second.volume_m3 > second.concentrations[slot] / second_degree {
                applied = -second.concentrations[slot] * second.volume_m3 / second_degree;
            }
            signed_amount[slot] = applied;
            concentration_delta[first_index][slot] -= applied / first.volume_m3;
            concentration_delta[second_index][slot] += applied / second.volume_m3;

            if !concentration_delta[first_index][slot].is_finite()
                || !concentration_delta[second_index][slot].is_finite()
            {
                return Err(BmxExchangeError::NonFiniteTransfer(
                    pair.first,
                    pair.second,
                    species,
                ));
            }
        }

        pair_exchanges.push(BmxPairExchange {
            first: pair.first,
            second: pair.second,
            signed_amount_by_species: signed_amount,
        });
    }

    let mut next = segments.to_vec();
    let mut total_after = [0.0_f64; 3];
    for (index, segment) in segments.iter().enumerate() {
        for species in BmxChemicalSpecies::ALL {
            let slot = species.index();
            let final_concentration =
                segment.concentrations[slot] + concentration_delta[index][slot];
            if !final_concentration.is_finite() || final_concentration < 0.0 {
                return Err(BmxExchangeError::NegativeFinalConcentration(
                    segment.id, species,
                ));
            }
            next[index].concentrations[slot] = final_concentration;
            total_after[slot] += final_concentration * segment.volume_m3;
            if !total_after[slot].is_finite() {
                return Err(BmxExchangeError::NonFiniteTotal(species));
            }
        }
    }

    let residual = [
        total_after[0] - total_before[0],
        total_after[1] - total_before[1],
        total_after[2] - total_before[2],
    ];
    segments.clone_from_slice(&next);

    Ok(BmxExchangeReport {
        pair_exchanges,
        skipped_pairs,
        total_amount_before_by_species: total_before,
        total_amount_after_by_species: total_after,
        conservation_residual_by_species: residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(
        id: u64,
        radius_m: f64,
        length_m: f64,
        concentration_a: f64,
        connections: &[u64],
    ) -> BmxExchangeSegment {
        BmxExchangeSegment {
            id: BmxSegmentId(id),
            is_fungus: true,
            radius_m,
            volume_m3: std::f64::consts::PI * radius_m * radius_m * length_m,
            concentrations: [concentration_a, 0.0, 0.0],
            connected_segment_ids: connections.iter().copied().map(BmxSegmentId).collect(),
        }
    }

    fn parameters(mt: f64) -> BmxExchangeParameters {
        BmxExchangeParameters {
            mt_m_per_time: [mt, 0.0, 0.0],
            dt_time_units: 0.25,
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left:.16e}, right={right:.16e}"
        );
    }

    #[test]
    fn unequal_radii_matches_mean_endcap_area_equation() {
        let mut state = vec![
            segment(1, 0.00025, 0.003, 100.0, &[2]),
            segment(2, 0.00015, 0.002, 20.0, &[1]),
        ];
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[BmxExchangePair {
                first: BmxSegmentId(1),
                second: BmxSegmentId(2),
            }],
            parameters(0.00002),
        )
        .unwrap();

        close(
            report.pair_exchanges[0].signed_amount_by_species[0],
            5.340707511102649e-11,
            1e-25,
        );
        close(state[0].concentrations[0], 99.90933333333334, 1e-10);
        close(state[1].concentrations[0], 20.377777777777776, 1e-10);
        close(report.conservation_residual_by_species[0], 0.0, 1e-24);
    }

    #[test]
    fn two_neighbor_source_degree_limiter_matches_bmx() {
        let radius = 0.00025;
        let mut state = vec![
            segment(1, radius, 0.003, 100.0, &[2, 3]),
            segment(2, radius, 0.003, 20.0, &[1]),
            segment(3, radius, 0.003, 20.0, &[1]),
        ];
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[
                BmxExchangePair {
                    first: BmxSegmentId(1),
                    second: BmxSegmentId(2),
                },
                BmxExchangePair {
                    first: BmxSegmentId(1),
                    second: BmxSegmentId(3),
                },
            ],
            parameters(1.0),
        )
        .unwrap();

        let volume = std::f64::consts::PI * radius * radius * 0.003;
        let cap_per_edge = 100.0 * volume / 2.0;
        close(
            report.pair_exchanges[0].signed_amount_by_species[0],
            cap_per_edge,
            1e-23,
        );
        close(
            report.pair_exchanges[1].signed_amount_by_species[0],
            cap_per_edge,
            1e-23,
        );
        close(state[0].concentrations[0], 0.0, 1e-10);
        close(state[1].concentrations[0], 70.0, 1e-10);
        close(state[2].concentrations[0], 70.0, 1e-10);
        close(report.conservation_residual_by_species[0], 0.0, 1e-22);
    }

    #[test]
    fn reverse_gradient_caps_the_second_segment_as_donor() {
        let radius = 0.00025;
        let mut state = vec![
            segment(1, radius, 0.003, 20.0, &[2]),
            segment(2, radius, 0.003, 100.0, &[1, 3]),
            segment(3, radius, 0.003, 20.0, &[2]),
        ];
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[
                BmxExchangePair {
                    first: BmxSegmentId(1),
                    second: BmxSegmentId(2),
                },
                BmxExchangePair {
                    first: BmxSegmentId(3),
                    second: BmxSegmentId(2),
                },
            ],
            parameters(1.0),
        )
        .unwrap();

        close(
            report.pair_exchanges[0].signed_amount_by_species[0],
            -2.9452431127404313e-8,
            1e-22,
        );
        close(state[1].concentrations[0], 0.0, 1e-10);
        close(state[0].concentrations[0], 70.0, 1e-10);
        close(state[2].concentrations[0], 70.0, 1e-10);
    }

    #[test]
    fn one_sided_bond_is_skipped_without_exchange() {
        let mut state = vec![
            segment(1, 0.00025, 0.003, 100.0, &[2]),
            segment(2, 0.00025, 0.003, 0.0, &[]),
        ];
        let before = state.clone();
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[BmxExchangePair {
                first: BmxSegmentId(1),
                second: BmxSegmentId(2),
            }],
            parameters(0.00002),
        )
        .unwrap();
        assert_eq!(
            report.skipped_pairs[0].reason,
            BmxSkipReason::MissingReciprocalBond
        );
        assert!(report.pair_exchanges.is_empty());
        assert_eq!(state, before);
    }

    #[test]
    fn non_fungal_pair_is_skipped_without_exchange() {
        let mut state = vec![
            segment(1, 0.00025, 0.003, 100.0, &[2]),
            segment(2, 0.00025, 0.003, 0.0, &[1]),
        ];
        state[1].is_fungus = false;
        let before = state.clone();
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[BmxExchangePair {
                first: BmxSegmentId(1),
                second: BmxSegmentId(2),
            }],
            parameters(0.00002),
        )
        .unwrap();
        assert_eq!(report.skipped_pairs[0].reason, BmxSkipReason::NonFungalPair);
        assert_eq!(state, before);
    }

    #[test]
    fn equal_concentration_produces_zero_exchange() {
        let mut state = vec![
            segment(1, 0.00025, 0.003, 20.0, &[2]),
            segment(2, 0.00025, 0.003, 20.0, &[1]),
        ];
        let before = state.clone();
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[BmxExchangePair {
                first: BmxSegmentId(1),
                second: BmxSegmentId(2),
            }],
            parameters(0.00002),
        )
        .unwrap();
        close(
            report.pair_exchanges[0].signed_amount_by_species[0],
            0.0,
            0.0,
        );
        assert_eq!(state, before);
    }

    #[test]
    fn duplicate_unordered_pair_is_rejected() {
        let mut state = vec![
            segment(1, 0.00025, 0.003, 100.0, &[2]),
            segment(2, 0.00025, 0.003, 0.0, &[1]),
        ];
        let before = state.clone();
        assert_eq!(
            apply_bmx_reference_exchange(
                &mut state,
                &[
                    BmxExchangePair {
                        first: BmxSegmentId(1),
                        second: BmxSegmentId(2)
                    },
                    BmxExchangePair {
                        first: BmxSegmentId(2),
                        second: BmxSegmentId(1)
                    },
                ],
                parameters(0.00002),
            ),
            Err(BmxExchangeError::DuplicatePair(
                BmxSegmentId(1),
                BmxSegmentId(2)
            ))
        );
        assert_eq!(state, before);
    }

    #[test]
    fn species_coefficients_are_applied_independently() {
        let mut state = vec![
            segment(1, 0.00025, 0.003, 100.0, &[2]),
            segment(2, 0.00025, 0.003, 0.0, &[1]),
        ];
        state[0].concentrations = [100.0, 100.0, 100.0];
        state[1].concentrations = [0.0, 0.0, 0.0];
        let report = apply_bmx_reference_exchange(
            &mut state,
            &[BmxExchangePair {
                first: BmxSegmentId(1),
                second: BmxSegmentId(2),
            }],
            BmxExchangeParameters {
                mt_m_per_time: [0.00001, 0.00002, 0.00004],
                dt_time_units: 0.25,
            },
        )
        .unwrap();
        assert!(
            report.pair_exchanges[0].signed_amount_by_species[0]
                < report.pair_exchanges[0].signed_amount_by_species[1]
        );
        assert!(
            report.pair_exchanges[0].signed_amount_by_species[1]
                < report.pair_exchanges[0].signed_amount_by_species[2]
        );
    }
}
