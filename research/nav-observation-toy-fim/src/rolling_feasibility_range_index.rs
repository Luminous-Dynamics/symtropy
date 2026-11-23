//! Conservative interval-backed range-min index for rolling-window
//! feasibility acceleration.
//!
//! The production policy's enumerated O(k_t) implementation remains the
//! reference oracle. This module only proves a fast PASS when outward-rounded
//! intervals establish that every relevant future start passes. Any ambiguous
//! comparison, non-monotone lower-bound sequence, invalid range or arithmetic
//! failure must take the reference path.
//!
//! For a future start s, callers provide outward intervals enclosing
//! R_s = N_s + alpha * (tau_s - s)
//! and Q_s = S_s - s * omega'_s + alpha * (tau_s - s).
//! For one candidate outcome z, U_t^z = S_(t-1) + z + t*omega'_(t,z).
//!
//! Since R_s is mathematically nondecreasing for the supported schedules, the
//! starts not proven safe by R_s form a prefix. The index binary-searches that
//! prefix and queries min(lower(Q_s)) using a segment tree. It certifies PASS
//! only when min(lower(Q_s)) >= upper(U_t^z). This is intentionally
//! conservative: uncertain values force the caller back to exact enumeration.
//!
//! The caller is responsible for constructing each interval with the exact
//! source values and an outward error bound that also covers the reference
//! expression's floating-point evaluation. The interval type below provides
//! basic IEEE-754 outward operations; this module does not silently assume
//! algebraic rearrangement is bitwise equivalent to a separately evaluated
//! expression.

use std::fmt;

/// Closed finite interval enclosing one real-valued quantity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutwardInterval {
    lower: f64,
    upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutwardIntervalError {
    NonFiniteValue,
    InvalidBounds,
    NegativeScale,
    ArithmeticOverflow,
}

impl fmt::Display for OutwardIntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => write!(f, "interval input is non-finite"),
            Self::InvalidBounds => write!(f, "interval lower bound exceeds upper bound"),
            Self::NegativeScale => write!(f, "nonnegative interval scale is negative"),
            Self::ArithmeticOverflow => write!(f, "outward interval arithmetic overflowed"),
        }
    }
}

impl std::error::Error for OutwardIntervalError {}

impl OutwardInterval {
    /// A known exact f64 value (the binary floating-point number itself).
    pub fn point(value: f64) -> Result<Self, OutwardIntervalError> {
        if !value.is_finite() {
            return Err(OutwardIntervalError::NonFiniteValue);
        }
        Ok(Self { lower: value, upper: value })
    }

    /// Enclose a finite value that may already have incurred one rounding.
    pub fn around(value: f64) -> Result<Self, OutwardIntervalError> {
        if !value.is_finite() {
            return Err(OutwardIntervalError::NonFiniteValue);
        }
        let lower = value.next_down();
        let upper = value.next_up();
        if !lower.is_finite() || !upper.is_finite() {
            return Err(OutwardIntervalError::ArithmeticOverflow);
        }
        Ok(Self { lower, upper })
    }

    /// Enclose an integer converted to f64. Integers up to 2^53 are exact.
    pub fn from_u64(value: u64) -> Result<Self, OutwardIntervalError> {
        let converted = value as f64;
        if !converted.is_finite() {
            return Err(OutwardIntervalError::ArithmeticOverflow);
        }
        if value <= (1_u64 << 53) {
            return Ok(Self { lower: converted, upper: converted });
        }
        Self::around(converted)
    }

    pub fn lower(self) -> f64 {
        self.lower
    }

    pub fn upper(self) -> f64 {
        self.upper
    }

    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && self.lower <= value && value <= self.upper
    }

    pub fn add(self, other: Self) -> Result<Self, OutwardIntervalError> {
        let lower = outward_down(self.lower + other.lower)?;
        let upper = outward_up(self.upper + other.upper)?;
        Self::new(lower, upper)
    }

    pub fn subtract(self, other: Self) -> Result<Self, OutwardIntervalError> {
        let lower = outward_down(self.lower - other.upper)?;
        let upper = outward_up(self.upper - other.lower)?;
        Self::new(lower, upper)
    }

    /// Multiply by a nonnegative f64 coefficient and an exact u64 count,
    /// enclosing both integer-to-f64 conversion and product rounding.
    pub fn nonnegative_coefficient_times_u64(
        coefficient: f64,
        count: u64,
    ) -> Result<Self, OutwardIntervalError> {
        if !coefficient.is_finite() {
            return Err(OutwardIntervalError::NonFiniteValue);
        }
        if coefficient < 0.0 {
            return Err(OutwardIntervalError::NegativeScale);
        }
        if coefficient == 0.0 || count == 0 {
            return Self::point(0.0);
        }
        let count_interval = Self::from_u64(count)?;
        let lower = outward_down(coefficient * count_interval.lower)?;
        let upper = outward_up(coefficient * count_interval.upper)?;
        Self::new(lower, upper)
    }

    fn new(lower: f64, upper: f64) -> Result<Self, OutwardIntervalError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(OutwardIntervalError::ArithmeticOverflow);
        }
        if lower > upper {
            return Err(OutwardIntervalError::InvalidBounds);
        }
        Ok(Self { lower, upper })
    }
}

fn outward_down(value: f64) -> Result<f64, OutwardIntervalError> {
    if !value.is_finite() {
        return Err(OutwardIntervalError::ArithmeticOverflow);
    }
    let result = value.next_down();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(OutwardIntervalError::ArithmeticOverflow)
    }
}

fn outward_up(value: f64) -> Result<f64, OutwardIntervalError> {
    if !value.is_finite() {
        return Err(OutwardIntervalError::ArithmeticOverflow);
    }
    let result = value.next_up();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(OutwardIntervalError::ArithmeticOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeMinWitness {
    pub start_index: usize,
    pub end_index_exclusive: usize,
    pub argmin_index: usize,
    /// The smallest lower bound over every Q interval in the queried range.
    pub minimum_lower_bound: f64,
    /// The upper bound belonging to the interval with the smallest lower bound.
    pub argmin_upper_bound: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeMinIndexError {
    InvalidRange,
    NonMonotoneRLowerBounds { index: usize },
    CapacityOverflow,
    Interval(OutwardIntervalError),
}

impl fmt::Display for RangeMinIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange => write!(f, "range-min query bounds are invalid"),
            Self::NonMonotoneRLowerBounds { index } => write!(f, "R lower bounds decreased at index {index}"),
            Self::CapacityOverflow => write!(f, "range-min index capacity overflowed"),
            Self::Interval(error) => write!(f, "interval construction failed: {error}"),
        }
    }
}

impl std::error::Error for RangeMinIndexError {}

impl From<OutwardIntervalError> for RangeMinIndexError {
    fn from(value: OutwardIntervalError) -> Self {
        Self::Interval(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RangeMinNode {
    lower: f64,
    upper: f64,
    index: usize,
}

impl RangeMinNode {
    fn from_interval(interval: OutwardInterval, index: usize) -> Self {
        Self { lower: interval.lower, upper: interval.upper, index }
    }

    fn combine(left: Option<Self>, right: Option<Self>) -> Option<Self> {
        match (left, right) {
            (None, None) => None,
            (Some(node), None) | (None, Some(node)) => Some(node),
            (Some(a), Some(b)) => {
                if a.lower < b.lower || (a.lower == b.lower && a.index <= b.index) {
                    Some(a)
                } else {
                    Some(b)
                }
            }
        }
    }
}

/// Append-only segment tree over Q intervals. Queries return the minimum
/// lower bound and its original start index in O(log n).
#[derive(Clone, Debug, PartialEq)]
pub struct RangeMinimumIndex {
    capacity: usize,
    len: usize,
    tree: Vec<Option<RangeMinNode>>,
    values: Vec<OutwardInterval>,
}

impl Default for RangeMinimumIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl RangeMinimumIndex {
    pub fn new() -> Self {
        Self {
            capacity: 1,
            len: 0,
            tree: vec![None; 2],
            values: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn append(&mut self, interval: OutwardInterval) -> Result<usize, RangeMinIndexError> {
        if self.len == self.capacity {
            self.grow()?;
        }
        let index = self.len;
        self.values.push(interval);
        self.len += 1;
        self.tree[self.capacity + index] =
            Some(RangeMinNode::from_interval(interval, index));
        self.rebuild_ancestors(self.capacity + index);
        Ok(index)
    }

    pub fn query(
        &self,
        start_index: usize,
        end_index_exclusive: usize,
    ) -> Result<Option<RangeMinWitness>, RangeMinIndexError> {
        if start_index > end_index_exclusive || end_index_exclusive > self.len {
            return Err(RangeMinIndexError::InvalidRange);
        }
        if start_index == end_index_exclusive {
            return Ok(None);
        }

        let mut left = start_index + self.capacity;
        let mut right = end_index_exclusive + self.capacity;
        let mut result: Option<RangeMinNode> = None;

        while left < right {
            if left & 1 == 1 {
                result = RangeMinNode::combine(result, self.tree[left]);
                left += 1;
            }
            if right & 1 == 1 {
                right -= 1;
                result = RangeMinNode::combine(result, self.tree[right]);
            }
            left /= 2;
            right /= 2;
        }

        let node = result.ok_or(RangeMinIndexError::InvalidRange)?;
        Ok(Some(RangeMinWitness {
            start_index,
            end_index_exclusive,
            argmin_index: node.index,
            minimum_lower_bound: node.lower,
            argmin_upper_bound: node.upper,
        }))
    }

    fn grow(&mut self) -> Result<(), RangeMinIndexError> {
        self.capacity = self.capacity.checked_mul(2)
            .ok_or(RangeMinIndexError::CapacityOverflow)?;
        let tree_len = self.capacity.checked_mul(2)
            .ok_or(RangeMinIndexError::CapacityOverflow)?;
        self.tree = vec![None; tree_len];
        for (index, value) in self.values.iter().copied().enumerate() {
            self.tree[self.capacity + index] =
                Some(RangeMinNode::from_interval(value, index));
        }
        for node in (1..self.capacity).rev() {
            self.tree[node] = RangeMinNode::combine(
                self.tree[node * 2],
                self.tree[node * 2 + 1],
            );
        }
        Ok(())
    }

    fn rebuild_ancestors(&mut self, mut node: usize) {
        while node > 1 {
            node /= 2;
            self.tree[node] = RangeMinNode::combine(
                self.tree[node * 2],
                self.tree[node * 2 + 1],
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RollingRangeProofWitness {
    pub queried_start_index: usize,
    pub queried_end_index_exclusive: usize,
    pub r_threshold_upper: f64,
    pub q_threshold_upper: f64,
    pub q_minimum: Option<RangeMinWitness>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RollingRangeDecision {
    /// The R test excludes all starts, or every potentially failing start has
    /// a Q lower bound above the candidate U upper bound.
    ProvenPass(RollingRangeProofWitness),
    /// No conclusion is issued. The caller must run the enumerated O(k_t)
    /// reference check over the complete original start set.
    RequiresReference(RollingRangeProofWitness),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollingRangeIndexError {
    InvalidRange,
    NonMonotoneRLowerBounds { index: usize },
    RangeMin(RangeMinIndexError),
}

impl fmt::Display for RollingRangeIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange => write!(f, "rolling future-start range is invalid"),
            Self::NonMonotoneRLowerBounds { index } => write!(f, "conservative R lower bounds are non-monotone at index {index}; use reference enumeration"),
            Self::RangeMin(error) => write!(f, "range-min index failed: {error}"),
        }
    }
}

impl std::error::Error for RollingRangeIndexError {}

impl From<RangeMinIndexError> for RollingRangeIndexError {
    fn from(value: RangeMinIndexError) -> Self {
        Self::RangeMin(value)
    }
}

/// Append-only paired R/Q index. R lower bounds must be nondecreasing; if
/// outward interval construction cannot preserve this, append/query rejects
/// and the caller must use the reference implementation.
#[derive(Clone, Debug, PartialEq)]
pub struct ConservativeRollingRangeIndex {
    r_lower_bounds: Vec<f64>,
    q_index: RangeMinimumIndex,
}

impl Default for ConservativeRollingRangeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ConservativeRollingRangeIndex {
    pub fn new() -> Self {
        Self { r_lower_bounds: Vec::new(), q_index: RangeMinimumIndex::new() }
    }

    pub fn len(&self) -> usize {
        self.r_lower_bounds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.r_lower_bounds.is_empty()
    }

    pub fn append(
        &mut self,
        r_interval: OutwardInterval,
        q_interval: OutwardInterval,
    ) -> Result<usize, RollingRangeIndexError> {
        if let Some(previous) = self.r_lower_bounds.last().copied() {
            if r_interval.lower < previous {
                return Err(RollingRangeIndexError::NonMonotoneRLowerBounds {
                    index: self.r_lower_bounds.len(),
                });
            }
        }
        let index = self.q_index.append(q_interval)?;
        self.r_lower_bounds.push(r_interval.lower);
        Ok(index)
    }

    /// Consider every index in [start,end). An index is excluded from the Q
    /// query only when its R lower bound is at least the R-threshold upper
    /// bound. All ambiguous R comparisons are included in the Q prefix.
    ///
    /// A PASS is returned only when the minimum Q lower bound over the entire
    /// conservative prefix is at least the candidate U upper bound. Otherwise
    /// the caller must run the exact enumerated reference path.
    pub fn prove_future_prefix(
        &self,
        start_index: usize,
        end_index_exclusive: usize,
        r_threshold: OutwardInterval,
        q_threshold: OutwardInterval,
    ) -> Result<RollingRangeDecision, RollingRangeIndexError> {
        if start_index > end_index_exclusive || end_index_exclusive > self.len() {
            return Err(RollingRangeIndexError::InvalidRange);
        }

        // Binary search first R lower bound that is definitely >= the
        // threshold. R lower bounds are checked monotone on every append.
        let mut low = start_index;
        let mut high = end_index_exclusive;
        while low < high {
            let middle = low + (high - low) / 2;
            if self.r_lower_bounds[middle] >= r_threshold.upper {
                high = middle;
            } else {
                low = middle + 1;
            }
        }

        let q_minimum = self.q_index.query(start_index, low)?;
        let witness = RollingRangeProofWitness {
            queried_start_index: start_index,
            queried_end_index_exclusive: low,
            r_threshold_upper: r_threshold.upper,
            q_threshold_upper: q_threshold.upper,
            q_minimum,
        };

        let proven = match q_minimum {
            None => true,
            Some(minimum) => minimum.minimum_lower_bound >= q_threshold.upper,
        };

        if proven {
            Ok(RollingRangeDecision::ProvenPass(witness))
        } else {
            Ok(RollingRangeDecision::RequiresReference(witness))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(value: f64) -> OutwardInterval {
        OutwardInterval::around(value).unwrap()
    }

    #[test]
    fn outward_integer_product_contains_exact_binary_float_product() {
        let product = 0.1 * 3.0;
        let bounded = OutwardInterval::nonnegative_coefficient_times_u64(0.1, 3).unwrap();
        assert!(bounded.contains(product));
        assert!(bounded.lower() <= product && product <= bounded.upper());
        assert!(OutwardInterval::from_u64(1_u64 << 53).unwrap().lower()
            == OutwardInterval::from_u64(1_u64 << 53).unwrap().upper());
        assert!(OutwardInterval::from_u64((1_u64 << 53) + 1).unwrap().lower()
            < OutwardInterval::from_u64((1_u64 << 53) + 1).unwrap().upper());
    }

    #[test]
    fn interval_addition_and_subtraction_enclose_rounded_results() {
        let left = interval(0.1);
        let right = interval(0.2);
        let sum = left.add(right).unwrap();
        let difference = left.subtract(right).unwrap();
        assert!(sum.contains(0.1 + 0.2));
        assert!(difference.contains(0.1 - 0.2));
    }

    #[test]
    fn range_min_tree_matches_linear_scan_for_all_ranges() {
        let mut tree = RangeMinimumIndex::new();
        let mut values = Vec::new();
        let mut state = 0xA5A5_1234_u64;
        for index in 0..97 {
            state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let value = ((state >> 20) % 10_000) as f64 / 100.0;
            let bounded = interval(value);
            tree.append(bounded).unwrap();
            values.push(bounded);
            assert_eq!(tree.len(), index + 1);
        }

        for start in 0..values.len() {
            for end in start + 1..=values.len() {
                let expected = values[start..end]
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.lower().total_cmp(&b.lower())
                    })
                    .map(|(offset, value)| (start + offset, value.lower(), value.upper()))
                    .unwrap();
                let observed = tree.query(start, end).unwrap().unwrap();
                assert_eq!(observed.argmin_index, expected.0);
                assert_eq!(observed.minimum_lower_bound, expected.1);
                assert_eq!(observed.argmin_upper_bound, expected.2);
            }
        }
    }

    #[test]
    fn monotone_r_prefix_and_q_minimum_can_prove_a_pass() {
        let mut index = ConservativeRollingRangeIndex::new();
        for (r, q) in [(0.1, 5.0), (0.2, 4.0), (0.3, 3.5), (0.4, 3.0), (0.5, 4.0)] {
            index.append(interval(r), interval(q)).unwrap();
        }
        let result = index.prove_future_prefix(0, 5, interval(0.35), interval(2.5)).unwrap();
        let RollingRangeDecision::ProvenPass(witness) = result else {
            panic!("expected conservative pass");
        };
        // R indices 0..3 are included; index 3 is ambiguous because its R
        // interval overlaps the threshold interval. Q min is safely above U.
        assert_eq!(witness.queried_start_index, 0);
        assert!(witness.queried_end_index_exclusive >= 3);
        assert_eq!(witness.q_minimum.unwrap().argmin_index, 3);
    }

    #[test]
    fn r_only_proof_can_exclude_every_start_without_querying_q() {
        let mut index = ConservativeRollingRangeIndex::new();
        for r in [2.0, 3.0, 4.0] {
            index.append(interval(r), interval(-100.0)).unwrap();
        }
        let result = index.prove_future_prefix(0, 3, interval(1.0), interval(1.0)).unwrap();
        let RollingRangeDecision::ProvenPass(witness) = result else {
            panic!("all starts should be proven by R");
        };
        assert_eq!(witness.queried_end_index_exclusive, 0);
        assert_eq!(witness.q_minimum, None);
    }

    #[test]
    fn q_minimum_below_or_ambiguous_with_threshold_requires_reference() {
        let mut index = ConservativeRollingRangeIndex::new();
        for (r, q) in [(0.1, 0.5), (0.2, 0.7), (0.3, 1.0), (0.4, 0.6)] {
            index.append(interval(r), interval(q)).unwrap();
        }
        let result = index.prove_future_prefix(0, 4, interval(0.9), interval(0.6)).unwrap();
        assert!(matches!(result, RollingRangeDecision::RequiresReference(_)));
    }

    #[test]
    fn exact_threshold_equality_is_conservatively_included_or_falls_back() {
        let mut index = ConservativeRollingRangeIndex::new();
        index.append(interval(1.0), interval(1.0)).unwrap();
        let result = index.prove_future_prefix(0, 1, interval(1.0), interval(1.0)).unwrap();
        assert!(matches!(result, RollingRangeDecision::RequiresReference(_)));
    }

    #[test]
    fn nonmonotone_r_lower_bounds_reject_and_require_reference_fallback() {
        let mut index = ConservativeRollingRangeIndex::new();
        index.append(interval(1.0), interval(1.0)).unwrap();
        assert_eq!(
            index.append(interval(0.5), interval(2.0)),
            Err(RollingRangeIndexError::NonMonotoneRLowerBounds { index: 1 })
        );
    }

    #[test]
    fn invalid_ranges_reject_without_returning_a_pass() {
        let index = ConservativeRollingRangeIndex::new();
        assert_eq!(
            index.prove_future_prefix(1, 0, interval(0.0), interval(0.0)),
            Err(RollingRangeIndexError::InvalidRange)
        );
        let mut tree = RangeMinimumIndex::new();
        tree.append(interval(1.0)).unwrap();
        assert_eq!(tree.query(0, 2), Err(RangeMinIndexError::InvalidRange));
    }

    #[test]
    fn randomized_conservative_pass_never_violates_unrounded_disjunction() {
        let mut state = 0xC0FF_EE00_D15C_A11_u64;
        for _trial in 0..128 {
            let count = 1 + ((state >> 8) % 64) as usize;
            let mut r_values = Vec::with_capacity(count);
            let mut q_values = Vec::with_capacity(count);
            let mut index = ConservativeRollingRangeIndex::new();
            let mut r = 0.0_f64;
            for _ in 0..count {
                state = state.wrapping_mul(2_862_933_555_777_941_757).wrapping_add(3_037_000_493);
                r += ((state >> 32) % 17) as f64 / 10.0;
                state = state.wrapping_mul(2_862_933_555_777_941_757).wrapping_add(3_037_000_493);
                let q = ((state >> 24) % 100) as f64 / 10.0;
                r_values.push(r);
                q_values.push(q);
                index.append(interval(r), interval(q)).unwrap();
            }

            let r_threshold = ((state >> 16) % 100) as f64 / 10.0;
            let q_threshold = ((state >> 28) % 100) as f64 / 10.0;
            let start = ((state >> 40) as usize) % count;
            let end = start + 1 + (((state >> 48) as usize) % (count - start));
            let decision = index.prove_future_prefix(
                start, end, interval(r_threshold), interval(q_threshold)
            ).unwrap();

            if matches!(decision, RollingRangeDecision::ProvenPass(_)) {
                for position in start..end {
                    assert!(
                        r_values[position] >= r_threshold || q_values[position] >= q_threshold,
                        "unsafe pass at {position}: R={} threshold={}, Q={} threshold={}",
                        r_values[position], r_threshold, q_values[position], q_threshold
                    );
                }
            }
        }
    }
}
