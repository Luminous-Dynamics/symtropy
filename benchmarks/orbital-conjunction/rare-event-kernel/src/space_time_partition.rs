// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact synthetic product-partition accounting for the space-time × sphere
//! Ncdot enclosure research.
//!
//! This first milestone deliberately supports only a two-hemisphere sphere
//! partition and caller-supplied constant rational integrand enclosures.
//! Contributions are represented as exact rational coefficients of π. It is
//! a partition/accounting verifier, not a physical rate-integrand enclosure,
//! arbitrary spherical tessellation, or machine-verified interval backend.

use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

/// Exact nonnegative rational number with a reduced numerator and denominator.
///
/// The denominator is always positive. Construction and arithmetic fail
/// closed on a zero denominator or u128 overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    numerator: u128,
    denominator: u128,
}

impl Rational {
    /// Construct and reduce a nonnegative rational number.
    pub fn new(numerator: u128, denominator: u128) -> Result<Self, SpaceTimePartitionError> {
        if denominator == 0 {
            return Err(SpaceTimePartitionError::InvalidRationalDenominator);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Construct an integer-valued rational number.
    pub const fn from_integer(value: u128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    /// Return the reduced numerator.
    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    /// Return the reduced positive denominator.
    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    /// Add two rationals exactly, returning an overflow error when necessary.
    pub fn checked_add(self, other: Self) -> Result<Self, SpaceTimePartitionError> {
        let common = gcd(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_factor)
            .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_factor)
            .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
        let numerator = left
            .checked_add(right)
            .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(left_factor)
            .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }

    /// Multiply two rationals exactly, cross-cancelling before multiplication.
    pub fn checked_mul(self, other: Self) -> Result<Self, SpaceTimePartitionError> {
        let cancel_left = gcd(self.numerator, other.denominator);
        let cancel_right = gcd(other.numerator, self.denominator);
        let left_numerator = self.numerator / cancel_left;
        let right_numerator = other.numerator / cancel_right;
        let left_denominator = self.denominator / cancel_right;
        let right_denominator = other.denominator / cancel_left;
        let numerator = left_numerator
            .checked_mul(right_numerator)
            .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
        let denominator = left_denominator
            .checked_mul(right_denominator)
            .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_fractions(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

// Compare fractions without cross multiplication, which could overflow even
// when both input fractions are individually representable.
fn compare_fractions(
    mut left_numerator: u128,
    mut left_denominator: u128,
    mut right_numerator: u128,
    mut right_denominator: u128,
) -> Ordering {
    let mut reverse = false;
    loop {
        let left_integer = left_numerator / left_denominator;
        let right_integer = right_numerator / right_denominator;
        if left_integer != right_integer {
            let order = left_integer.cmp(&right_integer);
            return if reverse { order.reverse() } else { order };
        }

        let left_remainder = left_numerator % left_denominator;
        let right_remainder = right_numerator % right_denominator;
        let order = match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => {
                left_numerator = left_denominator;
                left_denominator = left_remainder;
                right_numerator = right_denominator;
                right_denominator = right_remainder;
                reverse = !reverse;
                continue;
            }
        };
        return if reverse { order.reverse() } else { order };
    }
}

/// Closed interval with exact nonnegative rational endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonnegativeRationalInterval {
    /// Inclusive lower endpoint.
    pub lower: Rational,
    /// Inclusive upper endpoint.
    pub upper: Rational,
}

impl NonnegativeRationalInterval {
    /// Construct an ordered interval of exact nonnegative rationals.
    pub fn new(lower: Rational, upper: Rational) -> Result<Self, SpaceTimePartitionError> {
        if lower > upper {
            return Err(SpaceTimePartitionError::InvalidIntegrandInterval);
        }
        Ok(Self { lower, upper })
    }

    fn is_valid(self) -> bool {
        self.lower <= self.upper
    }
}

/// Stable identifier for a temporal slab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeSlabId(pub u64);

/// Stable identity for one of the two supported hemispherical patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpherePatchId {
    /// Unit-sphere hemisphere with nonnegative z coordinate.
    NorthHemisphere,
    /// Unit-sphere hemisphere with negative z coordinate.
    SouthHemisphere,
}

/// Declared assessment horizon in integer seconds.
///
/// The first prototype uses integer seconds to avoid ambiguity in time-scale
/// conversion and to make the synthetic partition exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceTimeHorizon {
    /// Inclusive beginning of the declared horizon.
    pub start_seconds: u64,
    /// Exclusive end of the declared horizon.
    pub end_seconds: u64,
}

/// One nonempty temporal slab in the partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSlab {
    /// Stable slab identity.
    pub id: TimeSlabId,
    /// Inclusive beginning in integer seconds.
    pub start_seconds: u64,
    /// Exclusive end in integer seconds.
    pub end_seconds: u64,
}

/// One declared spherical patch and its exact fraction of the full sphere.
///
/// The area fraction 1/2 denotes a hemisphere. The verifier currently accepts
/// only the north/south two-hemisphere partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpherePatch {
    /// Stable orientation/patch identity.
    pub id: SpherePatchId,
    /// Exact fraction of the full 4π surface area.
    pub area_fraction_of_full_sphere: Rational,
}

impl SpherePatch {
    /// Construct one canonical hemisphere with exactly half the full-sphere area.
    pub fn hemisphere(id: SpherePatchId) -> Self {
        Self {
            id,
            area_fraction_of_full_sphere: Rational {
                numerator: 1,
                denominator: 2,
            },
        }
    }
}

/// Identity tuple binding cells to one event, physical model and target measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceTimeScope {
    /// SHA-256 digest of the event/revision identity.
    pub event_digest: [u8; 32],
    /// SHA-256 digest of the physical/statistical model identity.
    pub model_digest: [u8; 32],
    /// SHA-256 digest of the declared target probability measure.
    pub target_distribution_digest: [u8; 32],
}

/// Exact contribution interval expressed as rational coefficients of π.
///
/// For a constant integrand enclosure g over a cell, the represented area is
/// 4π times the area fraction. Thus the cell contribution is
/// duration_seconds × 4 × area_fraction × g, multiplied by π. The rational
/// coefficient avoids floating-point arithmetic in this synthetic prototype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceTimeCellContribution {
    /// Temporal slab whose contribution is represented.
    pub time_slab_id: TimeSlabId,
    /// Sphere patch whose contribution is represented.
    pub sphere_patch_id: SpherePatchId,
    /// Exact lower coefficient in lower_pi_coefficient × π.
    pub lower_pi_coefficient: Rational,
    /// Exact upper coefficient in upper_pi_coefficient × π.
    pub upper_pi_coefficient: Rational,
}

/// One time-slab × sphere-patch cell with a caller-supplied constant enclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceTimeRateCell {
    /// Event/model/target identity of this cell.
    pub scope: SpaceTimeScope,
    /// Stable temporal slab identity.
    pub time_slab_id: TimeSlabId,
    /// Stable sphere patch identity.
    pub sphere_patch_id: SpherePatchId,
    /// Caller-supplied constant integrand enclosure for this synthetic cell.
    pub integrand_bounds: NonnegativeRationalInterval,
    /// Contribution supplied by the producer and independently recomputed by
    /// the verifier.
    pub claimed_contribution: SpaceTimeCellContribution,
}

impl SpaceTimeRateCell {
    /// Construct a cell and its claimed exact contribution from a constant
    /// rational integrand enclosure.
    ///
    /// The builder calculates a contribution claim, but the partition verifier
    /// independently recomputes it. This does not qualify the physical
    /// validity of the caller-supplied integrand enclosure.
    pub fn from_enclosure(
        scope: SpaceTimeScope,
        slab: TimeSlab,
        patch: SpherePatch,
        integrand_bounds: NonnegativeRationalInterval,
    ) -> Result<Self, SpaceTimePartitionError> {
        if slab.end_seconds <= slab.start_seconds {
            return Err(SpaceTimePartitionError::InvalidTimeSlab);
        }
        if !integrand_bounds.is_valid() {
            return Err(SpaceTimePartitionError::InvalidIntegrandInterval);
        }
        let contribution = calculate_contribution(slab, patch, integrand_bounds)?;
        Ok(Self {
            scope,
            time_slab_id: slab.id,
            sphere_patch_id: patch.id,
            integrand_bounds,
            claimed_contribution: contribution,
        })
    }
}

/// Complete proposed product partition for one event/model/target scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceTimePartition {
    /// Event/model/target identity required on every cell.
    pub scope: SpaceTimeScope,
    /// Entire declared time horizon.
    pub horizon: SpaceTimeHorizon,
    /// Proposed non-overlapping time slabs.
    pub time_slabs: Vec<TimeSlab>,
    /// Proposed full-sphere partition.
    pub sphere_patches: Vec<SpherePatch>,
    /// Proposed product cells; order is not semantically significant.
    pub cells: Vec<SpaceTimeRateCell>,
}

/// Deterministic, independently recomputed result of a verified partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcEnvelopeReport {
    /// Sum of lower contribution coefficients; multiply by π for the lower Nc.
    pub lower_pi_coefficient: Rational,
    /// Sum of upper contribution coefficients; multiply by π for the upper Nc.
    pub upper_pi_coefficient: Rational,
    /// Number of product cells independently verified.
    pub verified_cell_count: usize,
    /// Canonically ordered per-cell contribution ledger.
    pub ledger: Vec<SpaceTimeCellContribution>,
}

/// Errors that cause the synthetic product-partition verifier to abstain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceTimePartitionError {
    /// A rational denominator was zero.
    InvalidRationalDenominator,
    /// Exact rational arithmetic exceeded u128.
    ArithmeticOverflow,
    /// The declared horizon has invalid or reversed endpoints.
    InvalidHorizon,
    /// A temporal slab has an empty or reversed interval.
    InvalidTimeSlab,
    /// Temporal slabs do not tile the horizon contiguously and exactly once.
    TemporalGapOrOverlap,
    /// Two temporal slabs reuse the same stable identity.
    DuplicateTimeSlabId,
    /// The supported full-sphere partition must contain exactly two patches.
    InvalidSpherePatchCount,
    /// A hemisphere identity appears more than once.
    DuplicateSpherePatch,
    /// The two hemisphere patches must each have exactly half the sphere area.
    InvalidSpherePatchMeasure,
    /// A product cell names an undeclared temporal slab.
    UnknownTimeSlab,
    /// A product cell names an undeclared sphere patch.
    UnknownSpherePatch,
    /// A required time-slab × hemisphere product cell is missing.
    MissingProductCell,
    /// A time-slab × hemisphere product cell appears more than once.
    DuplicateProductCell,
    /// A cell's event/model/target identity differs from its partition.
    ScopeMismatch,
    /// A cell has a reversed integrand interval.
    InvalidIntegrandInterval,
    /// A producer-supplied contribution differs from exact recomputation.
    InvalidClaimedContribution,
}

impl core::fmt::Display for SpaceTimePartitionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidRationalDenominator => "rational denominator must be positive",
            Self::ArithmeticOverflow => "exact rational arithmetic overflowed u128",
            Self::InvalidHorizon => "space-time horizon must have increasing endpoints",
            Self::InvalidTimeSlab => "time slab must have increasing endpoints",
            Self::TemporalGapOrOverlap => "time slabs do not tile the horizon exactly once",
            Self::DuplicateTimeSlabId => "time slab identifiers must be unique",
            Self::InvalidSpherePatchCount => "the prototype requires exactly two sphere patches",
            Self::DuplicateSpherePatch => "sphere patch identifiers must be unique",
            Self::InvalidSpherePatchMeasure => "each hemisphere must have exactly half the sphere area",
            Self::UnknownTimeSlab => "product cell references an unknown time slab",
            Self::UnknownSpherePatch => "product cell references an unknown sphere patch",
            Self::MissingProductCell => "a required time-slab × hemisphere cell is missing",
            Self::DuplicateProductCell => "a time-slab × hemisphere cell is duplicated",
            Self::ScopeMismatch => "cell event/model/target identity differs from its partition",
            Self::InvalidIntegrandInterval => "integrand bounds must be ordered and nonnegative",
            Self::InvalidClaimedContribution => "claimed cell contribution does not match exact recomputation",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SpaceTimePartitionError {}

/// Independently verify complete two-slab/two-hemisphere product accounting.
///
/// The verifier checks an exact contiguous time tiling, exact north/south
/// hemisphere measures, matching scope identities, exactly one product cell
/// for every slab × patch pair, and each claimed contribution. All integration
/// arithmetic is exact rational arithmetic in coefficients of π.
///
/// This verifier does not establish the physical validity of the supplied
/// integrand enclosures, construct arbitrary sphere patches, or qualify
/// trajectory/covariance uncertainty.
pub fn verify_space_time_partition(
    partition: &SpaceTimePartition,
) -> Result<NcEnvelopeReport, SpaceTimePartitionError> {
    if partition.horizon.end_seconds <= partition.horizon.start_seconds {
        return Err(SpaceTimePartitionError::InvalidHorizon);
    }
    if partition.time_slabs.is_empty() {
        return Err(SpaceTimePartitionError::TemporalGapOrOverlap);
    }

    let mut slabs = partition.time_slabs.clone();
    slabs.sort_by_key(|slab| (slab.start_seconds, slab.end_seconds, slab.id));
    let mut slab_by_id = BTreeMap::new();
    let mut expected_start = partition.horizon.start_seconds;
    for slab in &slabs {
        if slab.end_seconds <= slab.start_seconds {
            return Err(SpaceTimePartitionError::InvalidTimeSlab);
        }
        if slab.start_seconds != expected_start || slab.end_seconds > partition.horizon.end_seconds {
            return Err(SpaceTimePartitionError::TemporalGapOrOverlap);
        }
        if slab_by_id.insert(slab.id, *slab).is_some() {
            return Err(SpaceTimePartitionError::DuplicateTimeSlabId);
        }
        expected_start = slab.end_seconds;
    }
    if expected_start != partition.horizon.end_seconds {
        return Err(SpaceTimePartitionError::TemporalGapOrOverlap);
    }

    if partition.sphere_patches.len() != 2 {
        return Err(SpaceTimePartitionError::InvalidSpherePatchCount);
    }
    let half = Rational {
        numerator: 1,
        denominator: 2,
    };
    let mut patch_by_id = BTreeMap::new();
    for patch in &partition.sphere_patches {
        if patch_by_id.insert(patch.id, *patch).is_some() {
            return Err(SpaceTimePartitionError::DuplicateSpherePatch);
        }
        if patch.area_fraction_of_full_sphere != half {
            return Err(SpaceTimePartitionError::InvalidSpherePatchMeasure);
        }
    }
    if !patch_by_id.contains_key(&SpherePatchId::NorthHemisphere)
        || !patch_by_id.contains_key(&SpherePatchId::SouthHemisphere)
    {
        return Err(SpaceTimePartitionError::InvalidSpherePatchMeasure);
    }

    let mut seen = BTreeSet::new();
    let mut ledger = Vec::with_capacity(partition.cells.len());
    for cell in &partition.cells {
        if cell.scope != partition.scope {
            return Err(SpaceTimePartitionError::ScopeMismatch);
        }
        let slab = slab_by_id
            .get(&cell.time_slab_id)
            .ok_or(SpaceTimePartitionError::UnknownTimeSlab)?;
        let patch = patch_by_id
            .get(&cell.sphere_patch_id)
            .ok_or(SpaceTimePartitionError::UnknownSpherePatch)?;
        if !cell.integrand_bounds.is_valid() {
            return Err(SpaceTimePartitionError::InvalidIntegrandInterval);
        }
        if !seen.insert((cell.time_slab_id, cell.sphere_patch_id)) {
            return Err(SpaceTimePartitionError::DuplicateProductCell);
        }
        let expected = calculate_contribution(*slab, *patch, cell.integrand_bounds)?;
        if cell.claimed_contribution != expected {
            return Err(SpaceTimePartitionError::InvalidClaimedContribution);
        }
        ledger.push(expected);
    }

    let required_count = slabs
        .len()
        .checked_mul(2)
        .ok_or(SpaceTimePartitionError::ArithmeticOverflow)?;
    if seen.len() != required_count {
        return Err(SpaceTimePartitionError::MissingProductCell);
    }

    let slab_order = slabs
        .iter()
        .enumerate()
        .map(|(index, slab)| (slab.id, index))
        .collect::<BTreeMap<_, _>>();
    ledger.sort_by_key(|item| {
        (
            slab_order.get(&item.time_slab_id).copied().unwrap_or(usize::MAX),
            item.sphere_patch_id,
        )
    });

    let mut lower = Rational::from_integer(0);
    let mut upper = Rational::from_integer(0);
    for item in &ledger {
        lower = lower.checked_add(item.lower_pi_coefficient)?;
        upper = upper.checked_add(item.upper_pi_coefficient)?;
    }

    Ok(NcEnvelopeReport {
        lower_pi_coefficient: lower,
        upper_pi_coefficient: upper,
        verified_cell_count: ledger.len(),
        ledger,
    })
}

fn calculate_contribution(
    slab: TimeSlab,
    patch: SpherePatch,
    integrand_bounds: NonnegativeRationalInterval,
) -> Result<SpaceTimeCellContribution, SpaceTimePartitionError> {
    if slab.end_seconds <= slab.start_seconds {
        return Err(SpaceTimePartitionError::InvalidTimeSlab);
    }
    if !integrand_bounds.is_valid() {
        return Err(SpaceTimePartitionError::InvalidIntegrandInterval);
    }
    let duration = Rational::from_integer(u128::from(
        slab.end_seconds
            .checked_sub(slab.start_seconds)
            .ok_or(SpaceTimePartitionError::InvalidTimeSlab)?,
    ));
    let four = Rational::from_integer(4);
    let area_factor = four.checked_mul(patch.area_fraction_of_full_sphere)?;
    let scale = duration.checked_mul(area_factor)?;
    Ok(SpaceTimeCellContribution {
        time_slab_id: slab.id,
        sphere_patch_id: patch.id,
        lower_pi_coefficient: scale.checked_mul(integrand_bounds.lower)?,
        upper_pi_coefficient: scale.checked_mul(integrand_bounds.upper)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn scope() -> SpaceTimeScope {
        SpaceTimeScope {
            event_digest: [1; 32],
            model_digest: [2; 32],
            target_distribution_digest: [3; 32],
        }
    }

    fn fixture() -> SpaceTimePartition {
        let horizon = SpaceTimeHorizon {
            start_seconds: 0,
            end_seconds: 5,
        };
        let first = TimeSlab {
            id: TimeSlabId(10),
            start_seconds: 0,
            end_seconds: 2,
        };
        let second = TimeSlab {
            id: TimeSlabId(20),
            start_seconds: 2,
            end_seconds: 5,
        };
        let north = SpherePatch::hemisphere(SpherePatchId::NorthHemisphere);
        let south = SpherePatch::hemisphere(SpherePatchId::SouthHemisphere);
        let cells = vec![
            SpaceTimeRateCell::from_enclosure(
                scope(),
                first,
                north,
                NonnegativeRationalInterval::new(rational(1, 2), rational(1, 1)).unwrap(),
            )
            .unwrap(),
            SpaceTimeRateCell::from_enclosure(
                scope(),
                first,
                south,
                NonnegativeRationalInterval::new(rational(1, 1), rational(3, 2)).unwrap(),
            )
            .unwrap(),
            SpaceTimeRateCell::from_enclosure(
                scope(),
                second,
                north,
                NonnegativeRationalInterval::new(rational(1, 4), rational(1, 2)).unwrap(),
            )
            .unwrap(),
            SpaceTimeRateCell::from_enclosure(
                scope(),
                second,
                south,
                NonnegativeRationalInterval::new(rational(3, 4), rational(1, 1)).unwrap(),
            )
            .unwrap(),
        ];
        SpaceTimePartition {
            scope: scope(),
            horizon,
            time_slabs: vec![first, second],
            sphere_patches: vec![north, south],
            cells,
        }
    }

    #[test]
    fn rational_reduces_and_adds_exactly() {
        let sum = rational(1, 2).checked_add(rational(1, 3)).unwrap();
        assert_eq!(sum, rational(5, 6));
    }

    #[test]
    fn synthetic_two_by_two_returns_exact_pi_scaled_envelope() {
        let report = verify_space_time_partition(&fixture()).unwrap();
        assert_eq!(report.lower_pi_coefficient, Rational::from_integer(12));
        assert_eq!(report.upper_pi_coefficient, Rational::from_integer(19));
        assert_eq!(report.verified_cell_count, 4);
        assert_eq!(report.ledger.len(), 4);
    }

    #[test]
    fn missing_product_cell_is_rejected() {
        let mut input = fixture();
        input.cells.pop();
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::MissingProductCell)
        );
    }

    #[test]
    fn duplicate_product_cell_is_rejected() {
        let mut input = fixture();
        let duplicate = input.cells[0];
        input.cells.push(duplicate);
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::DuplicateProductCell)
        );
    }

    #[test]
    fn temporal_gap_is_rejected() {
        let mut input = fixture();
        input.time_slabs[1].start_seconds = 3;
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::TemporalGapOrOverlap)
        );
    }

    #[test]
    fn temporal_overlap_is_rejected() {
        let mut input = fixture();
        input.time_slabs[1].start_seconds = 1;
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::TemporalGapOrOverlap)
        );
    }

    #[test]
    fn incorrect_hemisphere_measure_is_rejected() {
        let mut input = fixture();
        input.sphere_patches[0].area_fraction_of_full_sphere = rational(1, 3);
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::InvalidSpherePatchMeasure)
        );
    }

    #[test]
    fn incorrect_cell_contribution_is_rejected() {
        let mut input = fixture();
        input.cells[0].claimed_contribution.upper_pi_coefficient = Rational::from_integer(99);
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::InvalidClaimedContribution)
        );
    }

    #[test]
    fn cross_scope_cell_is_rejected() {
        let mut input = fixture();
        input.cells[0].scope.event_digest[0] = 9;
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::ScopeMismatch)
        );
    }

    #[test]
    fn invalid_integrand_interval_is_rejected() {
        let mut input = fixture();
        input.cells[0].integrand_bounds.lower = rational(2, 1);
        input.cells[0].integrand_bounds.upper = rational(1, 1);
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::InvalidIntegrandInterval)
        );
    }

    #[test]
    fn duplicate_temporal_slab_id_is_rejected() {
        let mut input = fixture();
        input.time_slabs[1].id = input.time_slabs[0].id;
        assert_eq!(
            verify_space_time_partition(&input),
            Err(SpaceTimePartitionError::DuplicateTimeSlabId)
        );
    }

    #[test]
    fn product_cell_order_does_not_change_replay_report() {
        let mut original = fixture();
        let expected = verify_space_time_partition(&original).unwrap();
        original.cells.reverse();
        let replayed = verify_space_time_partition(&original).unwrap();
        assert_eq!(replayed, expected);
    }

    #[test]
    fn rational_overflow_fails_closed() {
        let maximum = Rational::from_integer(u128::MAX);
        assert_eq!(
            maximum.checked_add(Rational::from_integer(1)),
            Err(SpaceTimePartitionError::ArithmeticOverflow)
        );
    }
}
