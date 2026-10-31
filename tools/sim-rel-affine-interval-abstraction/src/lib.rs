//! Exact one-step interval abstraction for bounded affine state transitions.
//!
//! This is a support abstraction, not a probability model or a multi-step
//! robust planner. It uses signed rational endpoints and fail-closed arithmetic.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::fmt;

/// A reduced signed rational with a strictly positive denominator.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SignedRational {
    numerator: i128,
    denominator: i128,
}

impl SignedRational {
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, String> {
        if denominator <= 0 {
            return Err("rational denominator must be positive".into());
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub const fn numerator(self) -> i128 {
        self.numerator
    }

    pub const fn denominator(self) -> i128 {
        self.denominator
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        let common = gcd(self.denominator as u128, other.denominator as u128) as i128;
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, String> {
        let cross_left = gcd(self.numerator.unsigned_abs(), other.denominator as u128) as i128;
        let cross_right = gcd(other.numerator.unsigned_abs(), self.denominator as u128) as i128;
        let left_num = self.numerator / cross_left;
        let right_num = other.numerator / cross_right;
        let left_den = self.denominator / cross_right;
        let right_den = other.denominator / cross_left;
        let numerator = left_num
            .checked_mul(right_num)
            .ok_or_else(|| "rational multiplication numerator overflow".to_string())?;
        let denominator = left_den
            .checked_mul(right_den)
            .ok_or_else(|| "rational multiplication denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }
}

impl Ord for SignedRational {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.numerator < 0, other.numerator < 0) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => compare_unsigned_fractions(
                self.numerator as u128,
                self.denominator as u128,
                other.numerator as u128,
                other.denominator as u128,
            ),
            (true, true) => compare_unsigned_fractions(
                other.numerator.unsigned_abs(),
                other.denominator as u128,
                self.numerator.unsigned_abs(),
                self.denominator as u128,
            ),
        }
    }
}

impl PartialOrd for SignedRational {
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

/// Compare non-negative fractions without cross-multiplication overflow.
fn compare_unsigned_fractions(
    mut left_num: u128,
    mut left_den: u128,
    mut right_num: u128,
    mut right_den: u128,
) -> Ordering {
    let mut reversed = false;
    loop {
        let left_quotient = left_num / left_den;
        let right_quotient = right_num / right_den;
        if left_quotient != right_quotient {
            return if reversed {
                right_quotient.cmp(&left_quotient)
            } else {
                left_quotient.cmp(&right_quotient)
            };
        }

        let left_remainder = left_num % left_den;
        let right_remainder = right_num % right_den;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reversed {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if reversed {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }

        left_num = left_den;
        left_den = left_remainder;
        right_num = right_den;
        right_den = right_remainder;
        reversed = !reversed;
    }
}

/// A closed interval with exact signed-rational endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
    lower: SignedRational,
    upper: SignedRational,
}

impl Interval {
    pub fn new(
        lower: SignedRational,
        upper: SignedRational,
    ) -> Result<Self, String> {
        if lower > upper {
            return Err("interval lower endpoint exceeds upper endpoint".into());
        }
        Ok(Self { lower, upper })
    }

    pub const fn lower(self) -> SignedRational {
        self.lower
    }

    pub const fn upper(self) -> SignedRational {
        self.upper
    }

    pub fn add(self, other: Self) -> Result<Self, String> {
        Self::new(
            self.lower.checked_add(other.lower)?,
            self.upper.checked_add(other.upper)?,
        )
    }

    pub fn multiply(self, other: Self) -> Result<Self, String> {
        let products = [
            self.lower.checked_mul(other.lower)?,
            self.lower.checked_mul(other.upper)?,
            self.upper.checked_mul(other.lower)?,
            self.upper.checked_mul(other.upper)?,
        ];
        let lower = *products.iter().min().expect("four products exist");
        let upper = *products.iter().max().expect("four products exist");
        Self::new(lower, upper)
    }

    pub fn intersects(self, other: Self) -> bool {
        self.lower <= other.upper && other.lower <= self.upper
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        let lower = self.lower.max(other.lower);
        let upper = self.upper.min(other.upper);
        Some(Self { lower, upper })
    }

    pub fn is_subset_of(self, other: Self) -> bool {
        other.lower <= self.lower && self.upper <= other.upper
    }
}

/// The uncertain scalar affine transition x' = a*x + b*u.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AffineMap {
    multiplier: Interval,
    offset: Interval,
    control: Interval,
}

impl AffineMap {
    pub const fn new(
        multiplier: Interval,
        offset: Interval,
        control: Interval,
    ) -> Self {
        Self {
            multiplier,
            offset,
            control,
        }
    }

    pub fn propagate(self, state: Interval) -> Result<Interval, String> {
        let state_term = self.multiplier.multiply(state)?;
        let control_term = self.offset.multiply(self.control)?;
        state_term.add(control_term)
    }
}

/// A finite closed-cell partition of one declared continuous state domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatePartition {
    domain: Interval,
    cells: Vec<Interval>,
}

impl StatePartition {
    pub fn new(domain: Interval, cells: Vec<Interval>) -> Result<Self, String> {
        if cells.is_empty() {
            return Err("state partition must contain at least one cell".into());
        }
        if domain.lower == domain.upper {
            return Err("state partition domain must have positive width".into());
        }
        if cells.iter().any(|cell| cell.lower == cell.upper) {
            return Err("state partition cells must have positive width".into());
        }
        if cells[0].lower != domain.lower || cells[cells.len() - 1].upper != domain.upper {
            return Err("state cells must cover both domain endpoints".into());
        }
        for cell in &cells {
            if !cell.is_subset_of(domain) {
                return Err("state cell lies outside the declared domain".into());
            }
        }
        for pair in cells.windows(2) {
            if pair[0].upper != pair[1].lower {
                return Err("state partition contains a gap or overlap".into());
            }
        }
        Ok(Self { domain, cells })
    }

    pub const fn domain(&self) -> Interval {
        self.domain
    }

    pub fn cells(&self) -> &[Interval] {
        &self.cells
    }
}

/// A reachable non-hazardous abstract successor or a campaign-ending hazard.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AbstractOutcome {
    SuccessorCell(usize),
    Hazard,
}

/// Exact enclosure and complete finite support for one abstract transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractTransition {
    pub reachable: Interval,
    pub outcomes: Vec<AbstractOutcome>,
}

impl AbstractTransition {
    pub fn successor_cells(&self) -> Vec<usize> {
        self.outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                AbstractOutcome::SuccessorCell(cell) => Some(*cell),
                AbstractOutcome::Hazard => None,
            })
            .collect()
    }

    pub fn hazard_possible(&self) -> bool {
        self.outcomes.contains(&AbstractOutcome::Hazard)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractionError {
    Arithmetic(String),
    SourceOutsideDomain,
    HazardOutsideDomain,
    ReachOutsideDomain,
    EmptySupport,
}

impl fmt::Display for AbstractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arithmetic(message) => write!(f, "exact interval arithmetic failed: {message}"),
            Self::SourceOutsideDomain => write!(f, "source interval lies outside the state domain"),
            Self::HazardOutsideDomain => write!(f, "hazard interval lies outside the state domain"),
            Self::ReachOutsideDomain => write!(f, "reachable interval exceeds the declared state domain"),
            Self::EmptySupport => write!(f, "abstract transition has no supported outcome"),
        }
    }
}

impl std::error::Error for AbstractionError {}

/// Propagate one abstract state through an uncertain affine map.
///
/// The returned outcomes define a full probability simplex over the support:
/// this function deliberately does not invent probabilities for parameter
/// uncertainty. All possible concrete next states are contained in the returned
/// successor cells or the explicit hazard outcome.
pub fn abstract_affine_transition(
    source: Interval,
    map: AffineMap,
    partition: &StatePartition,
    hazard_region: Option<Interval>,
) -> Result<AbstractTransition, AbstractionError> {
    if !source.is_subset_of(partition.domain) {
        return Err(AbstractionError::SourceOutsideDomain);
    }
    if hazard_region.is_some_and(|region| !region.is_subset_of(partition.domain)) {
        return Err(AbstractionError::HazardOutsideDomain);
    }

    let reachable = map
        .propagate(source)
        .map_err(AbstractionError::Arithmetic)?;
    if !reachable.is_subset_of(partition.domain) {
        return Err(AbstractionError::ReachOutsideDomain);
    }

    let mut outcomes = Vec::new();
    for (index, cell) in partition.cells.iter().enumerate() {
        if let Some(overlap) = cell.intersection(reachable) {
            let safe_part_exists = hazard_region
                .map(|hazard| !overlap.is_subset_of(hazard))
                .unwrap_or(true);
            if safe_part_exists {
                outcomes.push(AbstractOutcome::SuccessorCell(index));
            }
        }
    }

    if hazard_region.is_some_and(|hazard| reachable.intersects(hazard)) {
        outcomes.push(AbstractOutcome::Hazard);
    }

    if outcomes.is_empty() {
        return Err(AbstractionError::EmptySupport);
    }

    Ok(AbstractTransition {
        reachable,
        outcomes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(numerator: i128, denominator: i128) -> SignedRational {
        SignedRational::new(numerator, denominator).unwrap()
    }

    fn interval(lower: (i128, i128), upper: (i128, i128)) -> Interval {
        Interval::new(q(lower.0, lower.1), q(upper.0, upper.1)).unwrap()
    }

    fn two_cell_partition() -> StatePartition {
        StatePartition::new(
            interval((0, 1), (2, 1)),
            vec![interval((0, 1), (1, 1)), interval((1, 1), (2, 1))],
        )
        .unwrap()
    }

    #[test]
    fn exact_affine_range_maps_to_every_intersected_cell() {
        let partition = StatePartition::new(
            interval((0, 1), (4, 1)),
            vec![interval((0, 1), (2, 1)), interval((2, 1), (4, 1))],
        )
        .unwrap();
        let source = interval((0, 1), (1, 1));
        let map = AffineMap::new(
            interval((2, 1), (2, 1)),
            interval((1, 1), (1, 1)),
            interval((1, 1), (1, 1)),
        );

        let result = abstract_affine_transition(source, map, &partition, None).unwrap();

        assert_eq!(result.reachable, interval((1, 1), (3, 1)));
        assert_eq!(result.successor_cells(), vec![0, 1]);
        assert!(!result.hazard_possible());
    }

    #[test]
    fn hazard_boundary_is_reported_even_when_cell_midpoint_is_safe() {
        let partition = two_cell_partition();
        let source = interval((4, 5), (1, 1));
        let identity = AffineMap::new(
            interval((1, 1), (1, 1)),
            interval((0, 1), (0, 1)),
            interval((0, 1), (0, 1)),
        );
        let hazard = interval((19, 20), (11, 10));

        let result = abstract_affine_transition(source, identity, &partition, Some(hazard))
            .unwrap();

        assert_eq!(result.reachable, source);
        assert_eq!(result.successor_cells(), vec![0]);
        assert!(result.hazard_possible());
    }

    #[test]
    fn fully_hazardous_reachable_range_has_no_safe_successor() {
        let partition = two_cell_partition();
        let source = interval((1, 2), (4, 5));
        let identity = AffineMap::new(
            interval((1, 1), (1, 1)),
            interval((0, 1), (0, 1)),
            interval((0, 1), (0, 1)),
        );
        let hazard = interval((2, 5), (9, 10));

        let result = abstract_affine_transition(source, identity, &partition, Some(hazard))
            .unwrap();

        assert!(result.successor_cells().is_empty());
        assert!(result.hazard_possible());
        assert_eq!(result.outcomes, vec![AbstractOutcome::Hazard]);
    }

    #[test]
    fn negative_interval_product_preserves_sign() {
        let negative = interval((-2, 1), (-1, 1));
        let positive = interval((2, 1), (3, 1));
        let product = negative.multiply(positive).unwrap();

        assert_eq!(product, interval((-6, 1), (-2, 1)));
    }

    #[test]
    fn parameter_uncertainty_is_not_replaced_by_a_midpoint() {
        let partition = two_cell_partition();
        let source = interval((0, 1), (1, 1));
        let map = AffineMap::new(
            interval((1, 1), (2, 1)),
            interval((0, 1), (0, 1)),
            interval((1, 1), (1, 1)),
        );

        let result = abstract_affine_transition(source, map, &partition, None).unwrap();

        assert_eq!(result.reachable, interval((0, 1), (2, 1)));
        assert_eq!(result.successor_cells(), vec![0, 1]);
    }

    #[test]
    fn invalid_partition_gap_fails_closed() {
        let result = StatePartition::new(
            interval((0, 1), (4, 1)),
            vec![interval((0, 1), (1, 1)), interval((2, 1), (4, 1))],
        );
        assert!(result.is_err());
    }

    #[test]
    fn out_of_domain_reach_fails_closed() {
        let partition = two_cell_partition();
        let source = interval((0, 1), (1, 1));
        let map = AffineMap::new(
            interval((2, 1), (2, 1)),
            interval((0, 1), (0, 1)),
            interval((0, 1), (0, 1)),
        );

        let result = abstract_affine_transition(source, map, &partition, None);
        assert_eq!(result, Err(AbstractionError::ReachOutsideDomain));
    }

    #[test]
    fn rational_overflow_fails_closed() {
        let maximum = SignedRational::new(i128::MAX, 1).unwrap();
        assert!(maximum.checked_add(SignedRational::one()).is_err());
    }

    #[test]
    fn singleton_affine_state_is_exact() {
        let partition = StatePartition::new(
            interval((0, 1), (4, 1)),
            vec![interval((0, 1), (4, 1))],
        )
        .unwrap();
        let source = interval((1, 2), (1, 2));
        let map = AffineMap::new(
            interval((2, 1), (2, 1)),
            interval((1, 1), (1, 1)),
            interval((1, 1), (1, 1)),
        );

        let result = abstract_affine_transition(source, map, &partition, None).unwrap();

        assert_eq!(result.reachable, interval((2, 1), (2, 1)));
        assert_eq!(result.successor_cells(), vec![0]);
    }

    #[test]
    fn closed_partition_boundary_maps_to_both_adjacent_cells() {
        let partition = two_cell_partition();
        let source = interval((1, 1), (1, 1));
        let identity = AffineMap::new(
            interval((1, 1), (1, 1)),
            interval((0, 1), (0, 1)),
            interval((0, 1), (0, 1)),
        );

        let result = abstract_affine_transition(source, identity, &partition, None).unwrap();

        assert_eq!(result.successor_cells(), vec![0, 1]);
    }
}
