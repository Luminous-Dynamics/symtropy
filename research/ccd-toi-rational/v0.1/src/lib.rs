// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Exact rational TOI-claim evaluation for the supported u128 subdomain.
//!
//! This crate evaluates reported rational point estimates and rational time
//! intervals against independently supplied exact rational roots. It does not
//! compute collision geometry, derive roots, or attest a producer's proof.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum QueryKind {
    EdgeEdge,
    VertexFace,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QueryKey {
    dataset: String,
    scene: String,
    time_step: u64,
    query_kind: QueryKind,
    query_index: usize,
}

impl QueryKey {
    pub fn new(
        dataset: impl Into<String>,
        scene: impl Into<String>,
        time_step: u64,
        query_kind: QueryKind,
        query_index: usize,
    ) -> Self {
        Self {
            dataset: dataset.into(),
            scene: scene.into(),
            time_step,
            query_kind,
            query_index,
        }
    }

    #[must_use]
    pub fn dataset(&self) -> &str {
        &self.dataset
    }

    #[must_use]
    pub fn scene(&self) -> &str {
        &self.scene
    }

    #[must_use]
    pub fn time_step(&self) -> u64 {
        self.time_step
    }

    #[must_use]
    pub fn query_kind(&self) -> QueryKind {
        self.query_kind
    }

    #[must_use]
    pub fn query_index(&self) -> usize {
        self.query_index
    }

    fn is_valid(&self) -> bool {
        !self.dataset.trim().is_empty() && !self.scene.trim().is_empty()
    }
}

/// Non-negative exact rational. Values are reduced at construction.
/// The v0.1 arithmetic domain is u128; overflow is always an error.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RationalTime {
    numerator: u128,
    denominator: u128,
}

impl RationalTime {
    pub fn new(numerator: u128, denominator: u128) -> Result<Self, RationalError> {
        if denominator == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub fn parse(numerator: &str, denominator: &str) -> Result<Self, RationalError> {
        let numerator = numerator
            .parse::<u128>()
            .map_err(|_| RationalError::InvalidNumerator)?;
        let denominator = denominator
            .parse::<u128>()
            .map_err(|_| RationalError::InvalidDenominator)?;
        Self::new(numerator, denominator)
    }

    #[must_use]
    pub fn numerator(self) -> u128 {
        self.numerator
    }

    #[must_use]
    pub fn denominator(self) -> u128 {
        self.denominator
    }

    #[must_use]
    pub fn is_in_unit_interval(self) -> bool {
        self.cmp_exact(Self::ONE) != Ordering::Greater
    }

    #[must_use]
    pub fn cmp_exact(self, other: Self) -> Ordering {
        compare_fractions(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }

    pub fn subtract(self, other: Self) -> Result<Self, RationalError> {
        if self.cmp_exact(other) == Ordering::Less {
            return Err(RationalError::NegativeResult);
        }
        let common = gcd(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_factor)
            .ok_or(RationalError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_factor)
            .ok_or(RationalError::ArithmeticOverflow)?;
        let numerator = left
            .checked_sub(right)
            .ok_or(RationalError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(left_factor)
            .ok_or(RationalError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }

    pub const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Continued-fraction comparison avoids cross-product overflow.
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
            let ordering = left_integer.cmp(&right_integer);
            return if reverse {
                ordering.reverse()
            } else {
                ordering
            };
        }

        let left_remainder = left_numerator % left_denominator;
        let right_remainder = right_numerator % right_denominator;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reverse {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if reverse {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {
                left_numerator = left_denominator;
                left_denominator = left_remainder;
                right_numerator = right_denominator;
                right_denominator = right_remainder;
                reverse = !reverse;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RationalError {
    ZeroDenominator,
    InvalidNumerator,
    InvalidDenominator,
    NegativeResult,
    ArithmeticOverflow,
}

impl fmt::Display for RationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for RationalError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToiInterval {
    lower: RationalTime,
    upper: RationalTime,
}

impl ToiInterval {
    pub fn new(lower: RationalTime, upper: RationalTime) -> Result<Self, ToiError> {
        if !lower.is_in_unit_interval() || !upper.is_in_unit_interval() {
            return Err(ToiError::IntervalOutsideUnitDomain);
        }
        if lower.cmp_exact(upper) == Ordering::Greater {
            return Err(ToiError::InvertedInterval);
        }
        Ok(Self { lower, upper })
    }

    #[must_use]
    pub fn lower(self) -> RationalTime {
        self.lower
    }

    #[must_use]
    pub fn upper(self) -> RationalTime {
        self.upper
    }

    pub fn width(self) -> Result<RationalTime, RationalError> {
        self.upper.subtract(self.lower)
    }

    #[must_use]
    pub fn contains(self, root: RationalTime) -> bool {
        self.lower.cmp_exact(root) != Ordering::Greater
            && root.cmp_exact(self.upper) != Ordering::Greater
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToiClaim {
    /// The algorithm does not expose a TOI for this query.
    Unavailable,
    /// Exact rational point claim; this is not an interval certificate.
    PointEstimate(RationalTime),
    /// Reported rational enclosure. It is checked against the independent root.
    ReportedInterval(ToiInterval),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToiEvaluation {
    key: QueryKey,
    /// None means the independent oracle says there is no contact in [0,1].
    oracle_root: Option<RationalTime>,
    claim: ToiClaim,
}

impl ToiEvaluation {
    #[must_use]
    pub fn new(key: QueryKey, oracle_root: Option<RationalTime>, claim: ToiClaim) -> Self {
        Self {
            key,
            oracle_root,
            claim,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToiError {
    EmptyDatasetOrScene,
    DuplicateQueryIdentity,
    OracleRootOutsideUnitDomain,
    PointEstimateOutsideUnitDomain,
    IntervalOutsideUnitDomain,
    InvertedInterval,
    UnexpectedToiClaimForNoCollision,
    IntervalWidthArithmeticOverflow,
}

impl fmt::Display for ToiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ToiError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ToiCounts {
    pub total_queries: u64,
    pub oracle_positive_queries: u64,
    pub oracle_negative_queries: u64,
    pub unavailable_positive_claims: u64,
    pub point_estimate_claims: u64,
    pub point_exact_matches: u64,
    pub point_estimate_misses: u64,
    pub interval_claims: u64,
    pub interval_root_enclosures: u64,
    pub interval_under_enclosures: u64,
}

impl ToiCounts {
    #[must_use]
    pub fn claim_coverage_basis_points(self) -> Option<u64> {
        rate_basis_points(
            self.oracle_positive_queries - self.unavailable_positive_claims,
            self.oracle_positive_queries,
        )
    }

    #[must_use]
    pub fn point_exact_match_basis_points(self) -> Option<u64> {
        rate_basis_points(self.point_exact_matches, self.oracle_positive_queries)
    }

    #[must_use]
    pub fn interval_enclosure_basis_points(self) -> Option<u64> {
        rate_basis_points(self.interval_root_enclosures, self.oracle_positive_queries)
    }

    #[must_use]
    pub fn interval_conditional_enclosure_basis_points(self) -> Option<u64> {
        rate_basis_points(self.interval_root_enclosures, self.interval_claims)
    }

    #[must_use]
    pub fn toi_claims_qualified(self) -> bool {
        self.oracle_positive_queries > 0
            && self.unavailable_positive_claims == 0
            && self.point_estimate_misses == 0
            && self.interval_under_enclosures == 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToiScore {
    pub counts: ToiCounts,
    interval_widths: Vec<RationalTime>,
}

impl ToiScore {
    #[must_use]
    pub fn interval_width_percentile(&self, percentile: u8) -> Option<RationalTime> {
        if self.interval_widths.is_empty() || !(1..=100).contains(&percentile) {
            return None;
        }
        let mut widths = self.interval_widths.clone();
        widths.sort_by(|left, right| left.cmp_exact(*right));
        let rank = (widths.len() * usize::from(percentile))
            .div_ceil(100)
            .max(1);
        widths.get(rank - 1).copied()
    }
}

pub fn evaluate_toi(evaluations: &[ToiEvaluation]) -> Result<ToiScore, Vec<ToiError>> {
    let mut errors = Vec::new();
    let mut identities = HashSet::with_capacity(evaluations.len());
    let mut counts = ToiCounts {
        total_queries: evaluations.len() as u64,
        ..ToiCounts::default()
    };
    let mut interval_widths = Vec::new();

    for evaluation in evaluations {
        if !evaluation.key.is_valid() {
            errors.push(ToiError::EmptyDatasetOrScene);
        }
        if !identities.insert(evaluation.key.clone()) {
            errors.push(ToiError::DuplicateQueryIdentity);
        }

        if let Some(root) = evaluation.oracle_root {
            counts.oracle_positive_queries += 1;
            if !root.is_in_unit_interval() {
                errors.push(ToiError::OracleRootOutsideUnitDomain);
                continue;
            }
            match evaluation.claim {
                ToiClaim::Unavailable => {
                    counts.unavailable_positive_claims += 1;
                }
                ToiClaim::PointEstimate(estimate) => {
                    counts.point_estimate_claims += 1;
                    if !estimate.is_in_unit_interval() {
                        errors.push(ToiError::PointEstimateOutsideUnitDomain);
                    } else if estimate == root {
                        counts.point_exact_matches += 1;
                    } else {
                        counts.point_estimate_misses += 1;
                    }
                }
                ToiClaim::ReportedInterval(interval) => {
                    counts.interval_claims += 1;
                    if interval.contains(root) {
                        counts.interval_root_enclosures += 1;
                    } else {
                        counts.interval_under_enclosures += 1;
                    }
                    match interval.width() {
                        Ok(width) => interval_widths.push(width),
                        Err(_) => errors.push(ToiError::IntervalWidthArithmeticOverflow),
                    }
                }
            }
        } else {
            counts.oracle_negative_queries += 1;
            if evaluation.claim != ToiClaim::Unavailable {
                errors.push(ToiError::UnexpectedToiClaimForNoCollision);
            }
        }
    }

    if errors.is_empty() {
        Ok(ToiScore {
            counts,
            interval_widths,
        })
    } else {
        Err(errors)
    }
}

fn rate_basis_points(numerator: u64, denominator: u64) -> Option<u64> {
    if denominator == 0 {
        return None;
    }
    Some((u128::from(numerator) * 10_000 / u128::from(denominator)) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(kind: QueryKind, index: usize) -> QueryKey {
        QueryKey::new("known-answer-v0.1", "unit-scene", 0, kind, index)
    }

    fn rational(numerator: u128, denominator: u128) -> RationalTime {
        RationalTime::new(numerator, denominator).unwrap()
    }

    #[test]
    fn reduces_rationals_and_compares_without_cross_product_overflow() {
        assert_eq!(rational(2, 4), rational(1, 2));
        let large = rational(u128::MAX - 1, u128::MAX);
        let near_one = rational(u128::MAX - 2, u128::MAX - 1);
        assert_eq!(large.cmp_exact(near_one), Ordering::Greater);
        assert_eq!(rational(1, 2).cmp_exact(rational(2, 3)), Ordering::Less);
    }

    #[test]
    fn validates_unit_domain_and_interval_order() {
        let root = rational(1, 2);
        assert!(ToiInterval::new(rational(1, 4), rational(3, 4)).is_ok());
        assert_eq!(
            ToiInterval::new(rational(3, 4), rational(1, 4)),
            Err(ToiError::InvertedInterval)
        );
        assert_eq!(
            ToiInterval::new(root, rational(2, 1)),
            Err(ToiError::IntervalOutsideUnitDomain)
        );
        assert_eq!(RationalTime::new(1, 0), Err(RationalError::ZeroDenominator));
    }

    #[test]
    fn interval_score_checks_exact_root_enclosure_and_width_quantiles() {
        let root = rational(1, 2);
        let interval = ToiInterval::new(rational(1, 4), rational(3, 4)).unwrap();
        let score = evaluate_toi(&[
            ToiEvaluation::new(
                key(QueryKind::EdgeEdge, 0),
                Some(root),
                ToiClaim::ReportedInterval(interval),
            ),
            ToiEvaluation::new(key(QueryKind::EdgeEdge, 1), None, ToiClaim::Unavailable),
        ])
        .unwrap();

        assert_eq!(score.counts.oracle_positive_queries, 1);
        assert_eq!(score.counts.oracle_negative_queries, 1);
        assert_eq!(score.counts.interval_root_enclosures, 1);
        assert_eq!(score.counts.interval_under_enclosures, 0);
        assert_eq!(score.counts.interval_enclosure_basis_points(), Some(10_000));
        assert_eq!(score.interval_width_percentile(95), Some(rational(1, 2)));
        assert!(score.counts.toi_claims_qualified());
    }

    #[test]
    fn exact_half_time_roots_are_enclosed_for_both_primitive_families() {
        let root = rational(1, 2);
        let narrow = ToiInterval::new(root, root).unwrap();
        let score = evaluate_toi(&[
            ToiEvaluation::new(
                key(QueryKind::EdgeEdge, 0),
                Some(root),
                ToiClaim::ReportedInterval(narrow),
            ),
            ToiEvaluation::new(
                key(QueryKind::VertexFace, 0),
                Some(root),
                ToiClaim::ReportedInterval(narrow),
            ),
        ])
        .unwrap();

        assert_eq!(score.counts.oracle_positive_queries, 2);
        assert_eq!(score.counts.interval_claims, 2);
        assert_eq!(score.counts.interval_root_enclosures, 2);
        assert_eq!(score.counts.interval_enclosure_basis_points(), Some(10_000));
        assert_eq!(
            score.interval_width_percentile(50),
            Some(RationalTime::ZERO)
        );
        assert!(score.counts.toi_claims_qualified());
    }

    #[test]
    fn point_estimate_is_not_mislabeled_as_interval_certificate() {
        let root = rational(1, 2);
        let exact = evaluate_toi(&[ToiEvaluation::new(
            key(QueryKind::VertexFace, 0),
            Some(root),
            ToiClaim::PointEstimate(rational(1, 2)),
        )])
        .unwrap();
        assert_eq!(exact.counts.point_exact_matches, 1);
        assert_eq!(exact.counts.interval_claims, 0);
        assert_eq!(exact.counts.interval_enclosure_basis_points(), None);
        assert!(exact.counts.toi_claims_qualified());

        let miss = evaluate_toi(&[ToiEvaluation::new(
            key(QueryKind::VertexFace, 1),
            Some(root),
            ToiClaim::PointEstimate(rational(3, 4)),
        )])
        .unwrap();
        assert_eq!(miss.counts.point_estimate_misses, 1);
        assert!(!miss.counts.toi_claims_qualified());
    }

    #[test]
    fn rejects_under_enclosing_intervals_duplicate_ids_and_toi_on_negative_queries() {
        let root = rational(1, 2);
        let late_interval = ToiInterval::new(rational(3, 4), rational(1, 1)).unwrap();
        let score = evaluate_toi(&[ToiEvaluation::new(
            key(QueryKind::EdgeEdge, 0),
            Some(root),
            ToiClaim::ReportedInterval(late_interval),
        )])
        .unwrap();
        assert_eq!(score.counts.interval_under_enclosures, 1);
        assert_eq!(score.counts.interval_enclosure_basis_points(), Some(0));
        assert!(!score.counts.toi_claims_qualified());

        assert_eq!(
            evaluate_toi(&[
                ToiEvaluation::new(
                    key(QueryKind::EdgeEdge, 1),
                    Some(root),
                    ToiClaim::Unavailable,
                ),
                ToiEvaluation::new(
                    key(QueryKind::EdgeEdge, 1),
                    Some(root),
                    ToiClaim::Unavailable,
                ),
            ]),
            Err(vec![ToiError::DuplicateQueryIdentity])
        );

        assert_eq!(
            evaluate_toi(&[ToiEvaluation::new(
                key(QueryKind::EdgeEdge, 2),
                None,
                ToiClaim::PointEstimate(root),
            )]),
            Err(vec![ToiError::UnexpectedToiClaimForNoCollision])
        );
    }

    #[test]
    fn interval_width_overflow_fails_closed_instead_of_rounding() {
        let lower = rational(1, u128::MAX);
        let upper = rational(1, u128::MAX - 1);
        let interval = ToiInterval::new(lower, upper).unwrap();
        assert_eq!(interval.width(), Err(RationalError::ArithmeticOverflow));

        assert_eq!(
            evaluate_toi(&[ToiEvaluation::new(
                key(QueryKind::EdgeEdge, 9),
                Some(rational(1, 2)),
                ToiClaim::ReportedInterval(interval),
            )]),
            Err(vec![ToiError::IntervalWidthArithmeticOverflow])
        );
    }

    #[test]
    fn zero_positive_population_has_no_toi_recall() {
        let score = evaluate_toi(&[ToiEvaluation::new(
            key(QueryKind::EdgeEdge, 0),
            None,
            ToiClaim::Unavailable,
        )])
        .unwrap();
        assert_eq!(score.counts.interval_enclosure_basis_points(), None);
        assert!(!score.counts.toi_claims_qualified());
    }
}
