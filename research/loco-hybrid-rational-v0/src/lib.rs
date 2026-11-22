//! Exact rational interval and hybrid event-order reference fixtures.
//!
//! This crate is intentionally bounded and dependency-free. It provides exact
//! rational arithmetic for affine interval images and enumerates all event
//! orders consistent with declared closed event-time windows.
//!
//! It is not an ODE integrator, validated continuous reachability solver,
//! contact model, or physical robot safety certificate. A result is
//! conditional on the caller supplying sound state, coefficient, guard and
//! event-time enclosures. Arithmetic overflow and resource exhaustion return
//! errors rather than silently narrowing an enclosure.

use std::cmp::Ordering;
use std::collections::BTreeSet;

/// Fail-closed arithmetic, interval and event-enumeration errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HybridMathError {
    DivisionByZero,
    InvalidDenominator,
    ArithmeticOverflow,
    ReversedInterval,
    EmptyEventId,
    UntrimmedEventId(String),
    DuplicateEventId(String),
    NegativeEventTime(String),
    EventOrderLimitExceeded { maximum: usize },
}

/// A canonical exact rational number with a positive denominator.
///
/// The numerator and denominator are bounded by i128. Operations that cannot
/// be represented exactly return ArithmeticOverflow; no floating-point
/// rounding or saturation is performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, HybridMathError> {
        if denominator == 0 {
            return Err(HybridMathError::DivisionByZero);
        }

        let negative = (numerator < 0) ^ (denominator < 0);
        let divisor = gcd(numerator.unsigned_abs(), denominator.unsigned_abs());
        let reduced_numerator = numerator.unsigned_abs() / divisor;
        let reduced_denominator = denominator.unsigned_abs() / divisor;

        let denominator =
            i128::try_from(reduced_denominator).map_err(|_| HybridMathError::ArithmeticOverflow)?;
        let numerator = if negative {
            let min_magnitude = (i128::MAX as u128) + 1;
            if reduced_numerator == min_magnitude {
                i128::MIN
            } else {
                -i128::try_from(reduced_numerator)
                    .map_err(|_| HybridMathError::ArithmeticOverflow)?
            }
        } else {
            i128::try_from(reduced_numerator).map_err(|_| HybridMathError::ArithmeticOverflow)?
        };

        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub fn integer(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    pub fn zero() -> Self {
        Self::integer(0)
    }

    pub fn numerator(&self) -> i128 {
        self.numerator
    }

    pub fn denominator(&self) -> i128 {
        self.denominator
    }

    pub fn checked_add(&self, other: &Self) -> Result<Self, HybridMathError> {
        let common = gcd(self.denominator as u128, other.denominator as u128);
        let common = i128::try_from(common).map_err(|_| HybridMathError::ArithmeticOverflow)?;
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;

        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        let numerator = left
            .checked_add(right)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or(HybridMathError::ArithmeticOverflow)?;

        Self::new(numerator, denominator)
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self, HybridMathError> {
        self.checked_add(&other.checked_neg()?)
    }

    pub fn checked_neg(&self) -> Result<Self, HybridMathError> {
        Self::new(
            self.numerator
                .checked_neg()
                .ok_or(HybridMathError::ArithmeticOverflow)?,
            self.denominator,
        )
    }

    pub fn checked_mul(&self, other: &Self) -> Result<Self, HybridMathError> {
        let left_divisor = gcd(self.numerator.unsigned_abs(), other.denominator as u128);
        let right_divisor = gcd(other.numerator.unsigned_abs(), self.denominator as u128);
        let left_divisor =
            i128::try_from(left_divisor).map_err(|_| HybridMathError::ArithmeticOverflow)?;
        let right_divisor =
            i128::try_from(right_divisor).map_err(|_| HybridMathError::ArithmeticOverflow)?;

        let left_numerator = self.numerator / left_divisor;
        let right_numerator = other.numerator / right_divisor;
        let left_denominator = self.denominator / right_divisor;
        let right_denominator = other.denominator / left_divisor;

        let numerator = left_numerator
            .checked_mul(right_numerator)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        let denominator = left_denominator
            .checked_mul(right_denominator)
            .ok_or(HybridMathError::ArithmeticOverflow)?;

        Self::new(numerator, denominator)
    }

    pub fn checked_div(&self, other: &Self) -> Result<Self, HybridMathError> {
        if other.numerator == 0 {
            return Err(HybridMathError::DivisionByZero);
        }

        let numerator_divisor = gcd(
            self.numerator.unsigned_abs(),
            other.numerator.unsigned_abs(),
        );
        let denominator_divisor = gcd(self.denominator as u128, other.denominator as u128);

        let numerator_left = self.numerator.unsigned_abs() / numerator_divisor;
        let numerator_right = other.denominator as u128 / denominator_divisor;
        let denominator_left = self.denominator as u128 / denominator_divisor;
        let denominator_right = other.numerator.unsigned_abs() / numerator_divisor;

        let numerator_magnitude = numerator_left
            .checked_mul(numerator_right)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        let denominator_magnitude = denominator_left
            .checked_mul(denominator_right)
            .ok_or(HybridMathError::ArithmeticOverflow)?;

        let denominator = i128::try_from(denominator_magnitude)
            .map_err(|_| HybridMathError::ArithmeticOverflow)?;
        let is_negative = (self.numerator < 0) ^ (other.numerator < 0);
        let min_magnitude = (i128::MAX as u128) + 1;
        let numerator = if is_negative {
            if numerator_magnitude == min_magnitude {
                i128::MIN
            } else {
                -i128::try_from(numerator_magnitude)
                    .map_err(|_| HybridMathError::ArithmeticOverflow)?
            }
        } else {
            i128::try_from(numerator_magnitude).map_err(|_| HybridMathError::ArithmeticOverflow)?
        };

        Self::new(numerator, denominator)
    }

    /// Compare exactly. Very large cross-products fail closed on overflow.
    pub fn checked_cmp(&self, other: &Self) -> Result<Ordering, HybridMathError> {
        let left = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(self.denominator)
            .ok_or(HybridMathError::ArithmeticOverflow)?;
        Ok(left.cmp(&right))
    }
}

/// A closed interval with exact rational endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RationalInterval {
    lower: Rational,
    upper: Rational,
}

impl RationalInterval {
    pub fn new(lower: Rational, upper: Rational) -> Result<Self, HybridMathError> {
        if lower.checked_cmp(&upper)? == Ordering::Greater {
            return Err(HybridMathError::ReversedInterval);
        }
        Ok(Self { lower, upper })
    }

    pub fn point(value: Rational) -> Self {
        Self {
            lower: value.clone(),
            upper: value,
        }
    }

    pub fn lower(&self) -> &Rational {
        &self.lower
    }

    pub fn upper(&self) -> &Rational {
        &self.upper
    }

    pub fn width(&self) -> Result<Rational, HybridMathError> {
        self.upper.checked_sub(&self.lower)
    }

    pub fn add(&self, other: &Self) -> Result<Self, HybridMathError> {
        Self::new(
            self.lower.checked_add(&other.lower)?,
            self.upper.checked_add(&other.upper)?,
        )
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, HybridMathError> {
        Self::new(
            self.lower.checked_sub(&other.upper)?,
            self.upper.checked_sub(&other.lower)?,
        )
    }

    /// Exact interval multiplication using all four endpoint products.
    pub fn multiply(&self, other: &Self) -> Result<Self, HybridMathError> {
        let products = [
            self.lower.checked_mul(&other.lower)?,
            self.lower.checked_mul(&other.upper)?,
            self.upper.checked_mul(&other.lower)?,
            self.upper.checked_mul(&other.upper)?,
        ];

        let mut lower = products[0].clone();
        let mut upper = products[0].clone();
        for product in products.iter().skip(1) {
            if product.checked_cmp(&lower)? == Ordering::Less {
                lower = product.clone();
            }
            if product.checked_cmp(&upper)? == Ordering::Greater {
                upper = product.clone();
            }
        }

        Self::new(lower, upper)
    }

    /// Image under y = slope * x + offset, where slope and offset may
    /// themselves be uncertain closed intervals.
    pub fn affine_image(&self, slope: &Self, offset: &Self) -> Result<Self, HybridMathError> {
        slope.multiply(self)?.add(offset)
    }

    /// Closed intersection; None means the two intervals are disjoint.
    pub fn intersect(&self, other: &Self) -> Result<Option<Self>, HybridMathError> {
        let lower = if self.lower.checked_cmp(&other.lower)? == Ordering::Less {
            other.lower.clone()
        } else {
            self.lower.clone()
        };
        let upper = if self.upper.checked_cmp(&other.upper)? == Ordering::Greater {
            other.upper.clone()
        } else {
            self.upper.clone()
        };

        if lower.checked_cmp(&upper)? == Ordering::Greater {
            Ok(None)
        } else {
            Ok(Some(Self::new(lower, upper)?))
        }
    }

    pub fn contains(&self, value: &Rational) -> Result<bool, HybridMathError> {
        Ok(self.lower.checked_cmp(value)? != Ordering::Greater
            && self.upper.checked_cmp(value)? != Ordering::Less)
    }
}

/// A closed interval enclosing the possible time of one hybrid event.
///
/// The interval is supplied by the caller; this crate does not compute guard
/// crossing times from continuous dynamics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventWindow {
    id: String,
    time: RationalInterval,
}

impl EventWindow {
    pub fn new(id: String, time: RationalInterval) -> Result<Self, HybridMathError> {
        if id.trim().is_empty() {
            return Err(HybridMathError::EmptyEventId);
        }
        if id.trim() != id {
            return Err(HybridMathError::UntrimmedEventId(id));
        }
        if time.lower.checked_cmp(&Rational::zero())? == Ordering::Less {
            return Err(HybridMathError::NegativeEventTime(id));
        }
        Ok(Self { id, time })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn time(&self) -> &RationalInterval {
        &self.time
    }
}

/// Enumerate conservative total-order linearizations for the declared event
/// windows.
///
/// If upper(A) < lower(B), A must precede B. If windows overlap or only touch,
/// both linearizations are retained. This is a conservative overapproximation:
/// it can include spurious orders, and it does not model a coupled simultaneous
/// reset. A caller must provide a joint reset branch when simultaneity matters.
/// The input is canonicalized by event ID. Exceeding maximum_orders returns an
/// error rather than a partial list.
pub fn enumerate_possible_event_orders(
    mut events: Vec<EventWindow>,
    maximum_orders: usize,
) -> Result<Vec<Vec<String>>, HybridMathError> {
    events.sort_by(|left, right| left.id.cmp(&right.id));
    let mut seen = BTreeSet::new();
    for event in &events {
        if !seen.insert(event.id.clone()) {
            return Err(HybridMathError::DuplicateEventId(event.id.clone()));
        }
    }

    let mut selected = vec![false; events.len()];
    let mut prefix = Vec::with_capacity(events.len());
    let mut orders = Vec::new();

    enumerate_orders_recursive(
        &events,
        maximum_orders,
        &mut selected,
        &mut prefix,
        &mut orders,
    )?;

    Ok(orders)
}

fn enumerate_orders_recursive(
    events: &[EventWindow],
    maximum_orders: usize,
    selected: &mut [bool],
    prefix: &mut Vec<String>,
    orders: &mut Vec<Vec<String>>,
) -> Result<(), HybridMathError> {
    if prefix.len() == events.len() {
        if orders.len() >= maximum_orders {
            return Err(HybridMathError::EventOrderLimitExceeded {
                maximum: maximum_orders,
            });
        }
        orders.push(prefix.clone());
        return Ok(());
    }

    for candidate in 0..events.len() {
        if selected[candidate] {
            continue;
        }

        let mut all_predecessors_selected = true;
        for predecessor in 0..events.len() {
            if predecessor == candidate || selected[predecessor] {
                continue;
            }
            if events[predecessor]
                .time
                .upper
                .checked_cmp(&events[candidate].time.lower)?
                == Ordering::Less
            {
                all_predecessors_selected = false;
                break;
            }
        }

        if all_predecessors_selected {
            selected[candidate] = true;
            prefix.push(events[candidate].id.clone());
            enumerate_orders_recursive(events, maximum_orders, selected, prefix, orders)?;
            prefix.pop();
            selected[candidate] = false;
        }
    }

    Ok(())
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(numerator: i128, denominator: i128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn interval(lower: (i128, i128), upper: (i128, i128)) -> RationalInterval {
        RationalInterval::new(r(lower.0, lower.1), r(upper.0, upper.1)).unwrap()
    }

    fn event(id: &str, start: i128, end: i128) -> EventWindow {
        EventWindow::new(
            id.to_owned(),
            RationalInterval::new(Rational::integer(start), Rational::integer(end)).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn rational_construction_reduces_sign_and_common_factors() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(-2, -4), r(1, 2));
        assert_eq!(r(2, -4), r(-1, 2));
        assert_eq!(r(0, 19), Rational::zero());
        assert_eq!(Rational::new(0, i128::MIN).unwrap(), Rational::zero());
        assert_eq!(
            Rational::new(i128::MIN, i128::MIN).unwrap(),
            Rational::integer(1)
        );
    }

    #[test]
    fn exact_rational_add_subtract_multiply_and_divide() {
        assert_eq!(r(1, 3).checked_add(&r(1, 6)).unwrap(), r(1, 2));
        assert_eq!(r(5, 6).checked_sub(&r(1, 3)).unwrap(), r(1, 2));
        assert_eq!(r(2, 3).checked_mul(&r(9, 4)).unwrap(), r(3, 2));
        assert_eq!(r(3, 5).checked_div(&r(9, 10)).unwrap(), r(2, 3));
        assert_eq!(r(2, 3).checked_div(&r(-4, 5)).unwrap(), r(-5, 6));
        assert_eq!(
            Rational::integer(i128::MIN)
                .checked_div(&Rational::integer(i128::MIN))
                .unwrap(),
            Rational::integer(1)
        );
        assert_eq!(
            Rational::zero()
                .checked_div(&Rational::integer(i128::MIN))
                .unwrap(),
            Rational::zero()
        );
    }

    #[test]
    fn division_by_zero_and_unrepresentable_arithmetic_fail_closed() {
        assert_eq!(Rational::new(1, 0), Err(HybridMathError::DivisionByZero));
        assert_eq!(
            Rational::integer(i128::MAX).checked_add(&Rational::integer(1)),
            Err(HybridMathError::ArithmeticOverflow)
        );
    }

    #[test]
    fn interval_multiplication_handles_negative_and_positive_products() {
        let left = interval((-2, 1), (3, 1));
        let right = interval((-4, 1), (5, 1));
        let product = left.multiply(&right).unwrap();
        assert_eq!(product.lower(), &Rational::integer(-12));
        assert_eq!(product.upper(), &Rational::integer(15));
    }

    #[test]
    fn uncertain_affine_image_is_exact_for_rational_endpoints() {
        let state = interval((1, 1), (2, 1));
        let slope = interval((2, 1), (3, 1));
        let offset = interval((-1, 1), (1, 1));
        let image = state.affine_image(&slope, &offset).unwrap();
        assert_eq!(image.lower(), &Rational::integer(1));
        assert_eq!(image.upper(), &Rational::integer(7));
    }

    #[test]
    fn negative_slope_affine_image_reverses_endpoint_order_correctly() {
        let state = interval((1, 1), (3, 1));
        let slope = RationalInterval::point(Rational::integer(-2));
        let offset = RationalInterval::point(Rational::integer(1));
        let image = state.affine_image(&slope, &offset).unwrap();
        assert_eq!(image.lower(), &Rational::integer(-5));
        assert_eq!(image.upper(), &Rational::integer(-1));
    }

    #[test]
    fn closed_interval_intersection_preserves_shared_boundary() {
        let left = interval((0, 1), (2, 1));
        let right = interval((2, 1), (4, 1));
        let overlap = left.intersect(&right).unwrap().unwrap();
        assert_eq!(overlap.lower(), &Rational::integer(2));
        assert_eq!(overlap.upper(), &Rational::integer(2));
    }

    #[test]
    fn disjoint_intervals_have_no_intersection() {
        let left = interval((0, 1), (1, 1));
        let right = interval((2, 1), (3, 1));
        assert_eq!(left.intersect(&right).unwrap(), None);
    }

    #[test]
    fn strictly_separated_event_windows_have_one_order() {
        let orders = enumerate_possible_event_orders(
            vec![event("touchdown", 2, 3), event("lift-off", 0, 1)],
            4,
        )
        .unwrap();
        assert_eq!(
            orders,
            vec![vec!["lift-off".to_owned(), "touchdown".to_owned()]]
        );
    }

    #[test]
    fn overlapping_event_windows_preserve_both_orders() {
        let orders =
            enumerate_possible_event_orders(vec![event("a", 0, 2), event("b", 1, 3)], 4).unwrap();
        assert_eq!(
            orders,
            vec![
                vec!["a".to_owned(), "b".to_owned()],
                vec!["b".to_owned(), "a".to_owned()]
            ]
        );
    }

    #[test]
    fn touching_event_windows_retain_both_conservative_linearizations() {
        let orders =
            enumerate_possible_event_orders(vec![event("early", 0, 1), event("late", 1, 2)], 4)
                .unwrap();
        assert_eq!(orders.len(), 2);
    }

    #[test]
    fn event_order_results_are_invariant_to_input_permutation() {
        let left = enumerate_possible_event_orders(
            vec![event("a", 0, 2), event("b", 1, 3), event("c", 0, 4)],
            8,
        )
        .unwrap();
        let right = enumerate_possible_event_orders(
            vec![event("c", 0, 4), event("b", 1, 3), event("a", 0, 2)],
            8,
        )
        .unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn event_order_resource_exhaustion_returns_error_not_partial_orders() {
        let result = enumerate_possible_event_orders(
            vec![event("a", 0, 3), event("b", 0, 3), event("c", 0, 3)],
            5,
        );
        assert_eq!(
            result,
            Err(HybridMathError::EventOrderLimitExceeded { maximum: 5 })
        );
    }

    #[test]
    fn duplicate_and_negative_time_events_are_rejected() {
        assert_eq!(
            enumerate_possible_event_orders(vec![event("same", 0, 1), event("same", 1, 2)], 4),
            Err(HybridMathError::DuplicateEventId("same".to_owned()))
        );
        assert_eq!(
            EventWindow::new("negative".to_owned(), interval((-1, 1), (0, 1))),
            Err(HybridMathError::NegativeEventTime("negative".to_owned()))
        );
    }

    #[test]
    fn no_events_has_one_empty_total_order() {
        assert_eq!(
            enumerate_possible_event_orders(Vec::new(), 1).unwrap(),
            vec![Vec::<String>::new()]
        );
    }

    #[test]
    fn interval_multiplication_contains_every_endpoint_product_on_small_grid() {
        for a_lower in -2..=2 {
            for a_upper in a_lower..=2 {
                for b_lower in -2..=2 {
                    for b_upper in b_lower..=2 {
                        let left = interval((a_lower, 1), (a_upper, 1));
                        let right = interval((b_lower, 1), (b_upper, 1));
                        let image = left.multiply(&right).unwrap();
                        for x in [a_lower, a_upper] {
                            for y in [b_lower, b_upper] {
                                let value = Rational::integer(x * y);
                                assert!(image.contains(&value).unwrap());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn widening_state_interval_cannot_shrink_affine_image() {
        let narrow_state = interval((1, 1), (2, 1));
        let wide_state = interval((0, 1), (3, 1));
        let slope = interval((-2, 1), (3, 1));
        let offset = interval((-1, 1), (1, 1));

        let narrow_image = narrow_state.affine_image(&slope, &offset).unwrap();
        let wide_image = wide_state.affine_image(&slope, &offset).unwrap();

        assert!(
            wide_image
                .lower()
                .checked_cmp(narrow_image.lower())
                .unwrap()
                != Ordering::Greater
        );
        assert!(
            wide_image
                .upper()
                .checked_cmp(narrow_image.upper())
                .unwrap()
                != Ordering::Less
        );
    }

    #[test]
    fn event_order_enumerator_omits_no_strict_order_from_small_integer_grid() {
        for a_lower in 0..=2 {
            for a_upper in a_lower..=2 {
                for b_lower in 0..=2 {
                    for b_upper in b_lower..=2 {
                        let orders = enumerate_possible_event_orders(
                            vec![event("a", a_lower, a_upper), event("b", b_lower, b_upper)],
                            2,
                        )
                        .unwrap();

                        for time_a in a_lower..=a_upper {
                            for time_b in b_lower..=b_upper {
                                if time_a < time_b {
                                    assert!(orders.contains(&vec!["a".to_owned(), "b".to_owned()]));
                                } else if time_b < time_a {
                                    assert!(orders.contains(&vec!["b".to_owned(), "a".to_owned()]));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn touching_windows_are_conservative_and_can_include_spurious_reverse_order() {
        let orders =
            enumerate_possible_event_orders(vec![event("a", 0, 1), event("b", 1, 2)], 2).unwrap();

        // A at t=0.9 and B at t=1.1 are ordered A-before-B; equality at
        // t=1 is a simultaneous case. B-before-A is intentionally retained
        // as a conservative linearization, not claimed physically necessary.
        assert_eq!(orders.len(), 2);
        assert!(orders.contains(&vec!["a".to_owned(), "b".to_owned()]));
        assert!(orders.contains(&vec!["b".to_owned(), "a".to_owned()]));
    }

    #[test]
    fn rational_comparison_overflow_is_explicit() {
        let left = r(i128::MAX, i128::MAX - 1);
        let right = r(i128::MAX - 1, i128::MAX);
        assert_eq!(
            left.checked_cmp(&right),
            Err(HybridMathError::ArithmeticOverflow)
        );
    }
}

pub mod impact_enclosure {
    use std::cmp::Ordering;

    use super::{HybridMathError, Rational, RationalInterval};

    /// Model identity that must be matched by the event-window claim.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ImpactModelIdentity {
        pub model_build_sha256: String,
        pub guard_sha256: String,
        pub time_basis_id: String,
        /// Digest of the shared time origin / epoch definition, not just units.
        pub time_origin_sha256: String,
        /// Digest identifying the one simulation/trajectory timeline.
        pub timeline_sha256: String,
    }

    /// Exact constant-gravity point-impact model.
    ///
    /// Length is measured in metres. Time units are determined by the declared
    /// time basis, so velocity and gravity must use that same basis.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ConstantGravityImpactModel {
        pub identity: ImpactModelIdentity,
        pub initial_height_m: Rational,
        pub initial_velocity_m_per_time: Rational,
        pub gravity_m_per_time_squared: Rational,
    }

    /// A claimed interval for one event, bound to an exact model and guard.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ImpactEventTimeClaim {
        pub event_id: String,
        pub model_build_sha256: String,
        pub guard_sha256: String,
        pub time_basis_id: String,
        pub time_origin_sha256: String,
        pub timeline_sha256: String,
        pub event_time: RationalInterval,
    }

    /// Failure to verify an exact constant-gravity impact-time enclosure.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ImpactEnclosureError {
        EmptyIdentity(&'static str),
        InvalidSha256(&'static str),
        IdentityMismatch(&'static str),
        InvalidInitialHeight,
        InvalidGravity,
        NegativeTime,
        LowerGuardBelowSurface,
        UpperGuardAboveSurface,
        NotStrictlyDownwardTransverse,
        SingletonIsNotExactEvent,
        Arithmetic(HybridMathError),
    }

    impl From<HybridMathError> for ImpactEnclosureError {
        fn from(error: HybridMathError) -> Self {
            Self::Arithmetic(error)
        }
    }

    /// A verified event-time enclosure. Fields are private so callers cannot
    /// construct proof-grade results by relabelling arbitrary floating endpoints.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct VerifiedImpactEventEnclosure {
        event_id: String,
        model_build_sha256: String,
        guard_sha256: String,
        time_basis_id: String,
        time_origin_sha256: String,
        timeline_sha256: String,
        event_time: RationalInterval,
        lower_guard_value_m: Rational,
        upper_guard_value_m: Rational,
        derivative_upper_bound: Rational,
    }

    impl VerifiedImpactEventEnclosure {
        pub fn event_id(&self) -> &str {
            &self.event_id
        }

        pub fn model_build_sha256(&self) -> &str {
            &self.model_build_sha256
        }

        pub fn guard_sha256(&self) -> &str {
            &self.guard_sha256
        }

        pub fn time_basis_id(&self) -> &str {
            &self.time_basis_id
        }

        pub fn time_origin_sha256(&self) -> &str {
            &self.time_origin_sha256
        }

        pub fn timeline_sha256(&self) -> &str {
            &self.timeline_sha256
        }

        pub fn event_time(&self) -> &RationalInterval {
            &self.event_time
        }

        /// Exact guard value at the lower endpoint.
        pub fn lower_guard_value_m(&self) -> &Rational {
            &self.lower_guard_value_m
        }

        /// Exact guard value at the upper endpoint.
        pub fn upper_guard_value_m(&self) -> &Rational {
            &self.upper_guard_value_m
        }

        /// Exact upper bound on y'(t) over the interval. Since
        /// y'(t)=v0-g*t decreases for positive g, its maximum is at the lower
        /// endpoint.
        pub fn derivative_upper_bound(&self) -> &Rational {
            &self.derivative_upper_bound
        }
    }

    /// Certified relation between two independently verified event-time
    /// enclosures. Touching endpoints and all overlaps remain unresolved;
    /// interval overlap is not evidence of simultaneity.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CertifiedEventOrder {
        ProvenBefore,
        ProvenAfter,
        UnresolvedOverlap,
    }

    /// Compare only proof-grade impact enclosures. A strict gap between the
    /// closed intervals proves order; touching or overlapping intervals do
    /// not imply simultaneity and cannot be arbitrarily total-ordered.
    pub fn compare_verified_event_time_order(
        left: &VerifiedImpactEventEnclosure,
        right: &VerifiedImpactEventEnclosure,
    ) -> Result<CertifiedEventOrder, ImpactEnclosureError> {
        if left.time_basis_id != right.time_basis_id {
            return Err(ImpactEnclosureError::IdentityMismatch("time_basis_id"));
        }
        if left.time_origin_sha256 != right.time_origin_sha256 {
            return Err(ImpactEnclosureError::IdentityMismatch("time_origin_sha256"));
        }
        if left.timeline_sha256 != right.timeline_sha256 {
            return Err(ImpactEnclosureError::IdentityMismatch("timeline_sha256"));
        }

        if left
            .event_time
            .upper()
            .checked_cmp(right.event_time.lower())?
            == Ordering::Less
        {
            return Ok(CertifiedEventOrder::ProvenBefore);
        }
        if right
            .event_time
            .upper()
            .checked_cmp(left.event_time.lower())?
            == Ordering::Less
        {
            return Ok(CertifiedEventOrder::ProvenAfter);
        }
        Ok(CertifiedEventOrder::UnresolvedOverlap)
    }

    fn validate_non_empty(value: &str, field: &'static str) -> Result<(), ImpactEnclosureError> {
        if value.trim().is_empty() {
            return Err(ImpactEnclosureError::EmptyIdentity(field));
        }
        Ok(())
    }

    fn validate_sha256(value: &str, field: &'static str) -> Result<(), ImpactEnclosureError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ImpactEnclosureError::InvalidSha256(field));
        }
        Ok(())
    }

    fn evaluate_guard(
        model: &ConstantGravityImpactModel,
        time: &Rational,
    ) -> Result<Rational, HybridMathError> {
        let time_squared = time.checked_mul(time)?;
        let gravity_time_squared = model
            .gravity_m_per_time_squared
            .checked_mul(&time_squared)?;
        let gravity_term = gravity_time_squared.checked_div(&Rational::integer(2))?;
        let velocity_term = model.initial_velocity_m_per_time.checked_mul(time)?;
        model
            .initial_height_m
            .checked_add(&velocity_term)?
            .checked_sub(&gravity_term)
    }

    fn evaluate_guard_derivative(
        model: &ConstantGravityImpactModel,
        time: &Rational,
    ) -> Result<Rational, HybridMathError> {
        let gravity_time = model.gravity_m_per_time_squared.checked_mul(time)?;
        model.initial_velocity_m_per_time.checked_sub(&gravity_time)
    }

    /// Independently verify a rational event-time bracket for the first
    /// downward impact of the declared constant-gravity point-mass model.
    ///
    /// The proof uses exact rational guard evaluations at both endpoints and
    /// an exact derivative upper bound. Since g>0, y'(t)=v0-g*t decreases; if
    /// y'(t_lo)<0 then the guard is strictly decreasing on the full interval.
    /// Together with y(t_lo)>=0 and y(t_hi)<=0 this proves exactly one downward
    /// crossing in the closed interval, including exact endpoint roots.
    ///
    /// A singleton interval is accepted only for an exact root with negative
    /// derivative. Arithmetic overflow returns an error; there is no f64
    /// fallback, rounding or saturation.
    pub fn verify_constant_gravity_impact_enclosure(
        model: &ConstantGravityImpactModel,
        claim: &ImpactEventTimeClaim,
    ) -> Result<VerifiedImpactEventEnclosure, ImpactEnclosureError> {
        validate_non_empty(&claim.event_id, "event_id")?;
        validate_non_empty(&model.identity.time_basis_id, "model.time_basis_id")?;
        validate_non_empty(&claim.time_basis_id, "claim.time_basis_id")?;
        validate_sha256(&model.identity.model_build_sha256, "model_build_sha256")?;
        validate_sha256(&model.identity.guard_sha256, "guard_sha256")?;
        validate_sha256(&claim.model_build_sha256, "claim.model_build_sha256")?;
        validate_sha256(&claim.guard_sha256, "claim.guard_sha256")?;
        validate_sha256(&model.identity.time_origin_sha256, "time_origin_sha256")?;
        validate_sha256(&claim.time_origin_sha256, "claim.time_origin_sha256")?;
        validate_sha256(&model.identity.timeline_sha256, "timeline_sha256")?;
        validate_sha256(&claim.timeline_sha256, "claim.timeline_sha256")?;

        if claim.model_build_sha256 != model.identity.model_build_sha256 {
            return Err(ImpactEnclosureError::IdentityMismatch("model_build_sha256"));
        }
        if claim.guard_sha256 != model.identity.guard_sha256 {
            return Err(ImpactEnclosureError::IdentityMismatch("guard_sha256"));
        }
        if claim.time_basis_id != model.identity.time_basis_id {
            return Err(ImpactEnclosureError::IdentityMismatch("time_basis_id"));
        }
        if claim.time_origin_sha256 != model.identity.time_origin_sha256 {
            return Err(ImpactEnclosureError::IdentityMismatch("time_origin_sha256"));
        }
        if claim.timeline_sha256 != model.identity.timeline_sha256 {
            return Err(ImpactEnclosureError::IdentityMismatch("timeline_sha256"));
        }

        let zero = Rational::zero();
        if model.initial_height_m.checked_cmp(&zero)? != Ordering::Greater {
            return Err(ImpactEnclosureError::InvalidInitialHeight);
        }
        if model.gravity_m_per_time_squared.checked_cmp(&zero)? != Ordering::Greater {
            return Err(ImpactEnclosureError::InvalidGravity);
        }

        let lower = claim.event_time.lower();
        let upper = claim.event_time.upper();
        if lower.checked_cmp(&zero)? == Ordering::Less {
            return Err(ImpactEnclosureError::NegativeTime);
        }

        let lower_guard = evaluate_guard(model, lower)?;
        let upper_guard = evaluate_guard(model, upper)?;

        // Classify singleton windows before the directional sign test so a
        // non-root point receives the precise singleton-not-event disposition.
        if lower == upper && lower_guard != zero {
            return Err(ImpactEnclosureError::SingletonIsNotExactEvent);
        }

        if lower_guard.checked_cmp(&zero)? == Ordering::Less {
            return Err(ImpactEnclosureError::LowerGuardBelowSurface);
        }
        if upper_guard.checked_cmp(&zero)? == Ordering::Greater {
            return Err(ImpactEnclosureError::UpperGuardAboveSurface);
        }

        let derivative_upper_bound = evaluate_guard_derivative(model, lower)?;
        if derivative_upper_bound.checked_cmp(&zero)? != Ordering::Less {
            return Err(ImpactEnclosureError::NotStrictlyDownwardTransverse);
        }

        Ok(VerifiedImpactEventEnclosure {
            event_id: claim.event_id.clone(),
            model_build_sha256: claim.model_build_sha256.clone(),
            guard_sha256: claim.guard_sha256.clone(),
            time_basis_id: claim.time_basis_id.clone(),
            time_origin_sha256: claim.time_origin_sha256.clone(),
            timeline_sha256: claim.timeline_sha256.clone(),
            event_time: claim.event_time.clone(),
            lower_guard_value_m: lower_guard,
            upper_guard_value_m: upper_guard,
            derivative_upper_bound,
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn r(numerator: i128, denominator: i128) -> Rational {
            Rational::new(numerator, denominator).unwrap()
        }

        fn model() -> ConstantGravityImpactModel {
            ConstantGravityImpactModel {
                identity: ImpactModelIdentity {
                    model_build_sha256: "a".repeat(64),
                    guard_sha256: "b".repeat(64),
                    time_basis_id: "seconds-si-v1".to_owned(),
                    time_origin_sha256: "f".repeat(64),
                    timeline_sha256: "1".repeat(64),
                },
                initial_height_m: Rational::integer(1),
                initial_velocity_m_per_time: Rational::zero(),
                gravity_m_per_time_squared: Rational::integer(2),
            }
        }

        fn claim(
            model: &ConstantGravityImpactModel,
            event_id: &str,
            lower: Rational,
            upper: Rational,
        ) -> ImpactEventTimeClaim {
            ImpactEventTimeClaim {
                event_id: event_id.to_owned(),
                model_build_sha256: model.identity.model_build_sha256.clone(),
                guard_sha256: model.identity.guard_sha256.clone(),
                time_basis_id: model.identity.time_basis_id.clone(),
                time_origin_sha256: model.identity.time_origin_sha256.clone(),
                timeline_sha256: model.identity.timeline_sha256.clone(),
                event_time: RationalInterval::new(lower, upper).unwrap(),
            }
        }

        #[test]
        fn exact_rational_bracket_proves_unique_downward_crossing() {
            // y(t)=1-t^2 for h=1, v0=0, g=2. The true root is t=1.
            let model = model();
            let verified = verify_constant_gravity_impact_enclosure(
                &model,
                &claim(&model, "first-impact", r(3, 4), r(5, 4)),
            )
            .unwrap();

            assert_eq!(verified.event_id(), "first-impact");
            assert_eq!(verified.lower_guard_value_m(), &r(7, 16));
            assert_eq!(verified.upper_guard_value_m(), &r(-9, 16));
            assert_eq!(verified.derivative_upper_bound(), &r(-3, 2));
            assert_eq!(verified.event_time().lower(), &r(3, 4));
            assert_eq!(verified.event_time().upper(), &r(5, 4));
        }

        #[test]
        fn exact_lower_and_upper_endpoint_roots_are_admitted() {
            let model = model();
            let lower_root = verify_constant_gravity_impact_enclosure(
                &model,
                &claim(&model, "lower-root", r(1, 1), r(5, 4)),
            )
            .unwrap();
            assert_eq!(lower_root.lower_guard_value_m(), &Rational::zero());
            assert_eq!(lower_root.upper_guard_value_m(), &r(-9, 16));

            let upper_root = verify_constant_gravity_impact_enclosure(
                &model,
                &claim(&model, "upper-root", r(3, 4), r(1, 1)),
            )
            .unwrap();
            assert_eq!(upper_root.upper_guard_value_m(), &Rational::zero());
        }

        #[test]
        fn exact_singleton_root_is_admitted_but_non_root_singleton_is_rejected() {
            let model = model();
            let exact = verify_constant_gravity_impact_enclosure(
                &model,
                &claim(&model, "exact-root", r(1, 1), r(1, 1)),
            )
            .unwrap();
            assert_eq!(exact.lower_guard_value_m(), &Rational::zero());

            assert_eq!(
                verify_constant_gravity_impact_enclosure(
                    &model,
                    &claim(&model, "not-a-root", r(1, 2), r(1, 2)),
                ),
                Err(ImpactEnclosureError::SingletonIsNotExactEvent)
            );
        }

        #[test]
        fn rejects_same_sign_and_inverted_guard_brackets() {
            let model = model();
            assert_eq!(
                verify_constant_gravity_impact_enclosure(
                    &model,
                    &claim(&model, "too-early", r(0, 1), r(1, 2)),
                ),
                Err(ImpactEnclosureError::UpperGuardAboveSurface)
            );
            assert_eq!(
                verify_constant_gravity_impact_enclosure(
                    &model,
                    &claim(&model, "too-late", r(5, 4), r(3, 2)),
                ),
                Err(ImpactEnclosureError::LowerGuardBelowSurface)
            );
        }

        #[test]
        fn rejects_brackets_without_a_strict_downward_transversality_bound() {
            let mut model = model();
            model.initial_velocity_m_per_time = Rational::integer(2);
            assert_eq!(
                verify_constant_gravity_impact_enclosure(
                    &model,
                    &claim(&model, "crosses-the-apex", r(0, 1), r(3, 1)),
                ),
                Err(ImpactEnclosureError::NotStrictlyDownwardTransverse)
            );
        }

        #[test]
        fn rejects_mismatched_model_guard_and_time_basis_identities() {
            let model = model();
            let mut claim_value = claim(&model, "impact", r(3, 4), r(5, 4));
            claim_value.model_build_sha256 = "c".repeat(64);
            assert_eq!(
                verify_constant_gravity_impact_enclosure(&model, &claim_value),
                Err(ImpactEnclosureError::IdentityMismatch("model_build_sha256"))
            );

            let mut claim_value = claim(&model, "impact", r(3, 4), r(5, 4));
            claim_value.guard_sha256 = "c".repeat(64);
            assert_eq!(
                verify_constant_gravity_impact_enclosure(&model, &claim_value),
                Err(ImpactEnclosureError::IdentityMismatch("guard_sha256"))
            );

            let mut claim_value = claim(&model, "impact", r(3, 4), r(5, 4));
            claim_value.time_basis_id = "wall-clock-seconds".to_owned();
            assert_eq!(
                verify_constant_gravity_impact_enclosure(&model, &claim_value),
                Err(ImpactEnclosureError::IdentityMismatch("time_basis_id"))
            );
        }

        #[test]
        fn positive_time_unit_conversion_preserves_the_physical_event() {
            let seconds_model = model();
            let seconds_claim = claim(&seconds_model, "impact-seconds", r(3, 4), r(5, 4));
            let seconds =
                verify_constant_gravity_impact_enclosure(&seconds_model, &seconds_claim).unwrap();

            let mut milliseconds_model = model();
            milliseconds_model.identity.time_basis_id = "milliseconds-si-v1".to_owned();
            milliseconds_model.gravity_m_per_time_squared = r(1, 500_000);
            let milliseconds_claim = claim(
                &milliseconds_model,
                "impact-milliseconds",
                Rational::integer(750),
                Rational::integer(1250),
            );
            let milliseconds =
                verify_constant_gravity_impact_enclosure(&milliseconds_model, &milliseconds_claim)
                    .unwrap();

            assert_eq!(
                seconds.lower_guard_value_m(),
                milliseconds.lower_guard_value_m()
            );
            assert_eq!(
                seconds.upper_guard_value_m(),
                milliseconds.upper_guard_value_m()
            );
            assert_eq!(seconds.derivative_upper_bound(), &r(-3, 2));
            assert_eq!(milliseconds.derivative_upper_bound(), &r(-3, 2000));
        }

        #[test]
        fn verified_event_order_requires_strict_interval_separation() {
            let first_model = model();
            let first = verify_constant_gravity_impact_enclosure(
                &first_model,
                &claim(&first_model, "first", r(3, 4), r(5, 4)),
            )
            .unwrap();

            let mut second_model = model();
            second_model.initial_height_m = Rational::integer(4);
            second_model.identity.guard_sha256 = "c".repeat(64);
            let second = verify_constant_gravity_impact_enclosure(
                &second_model,
                &claim(&second_model, "second", r(7, 4), r(9, 4)),
            )
            .unwrap();

            assert_eq!(
                compare_verified_event_time_order(&first, &second),
                Ok(CertifiedEventOrder::ProvenBefore)
            );
            assert_eq!(
                compare_verified_event_time_order(&second, &first),
                Ok(CertifiedEventOrder::ProvenAfter)
            );
        }

        #[test]
        fn touching_verified_event_windows_remain_unresolved() {
            let first_model = model();
            let first = verify_constant_gravity_impact_enclosure(
                &first_model,
                &claim(&first_model, "first", r(3, 4), r(5, 4)),
            )
            .unwrap();

            let mut second_model = model();
            second_model.initial_height_m = r(25, 16);
            second_model.identity.guard_sha256 = "c".repeat(64);
            let second = verify_constant_gravity_impact_enclosure(
                &second_model,
                &claim(&second_model, "second", r(5, 4), r(3, 2)),
            )
            .unwrap();

            assert_eq!(
                compare_verified_event_time_order(&first, &second),
                Ok(CertifiedEventOrder::UnresolvedOverlap)
            );
        }

        #[test]
        fn overlapping_and_nested_verified_event_windows_remain_unresolved() {
            let first_model = model();
            let first = verify_constant_gravity_impact_enclosure(
                &first_model,
                &claim(&first_model, "first", r(3, 4), r(5, 4)),
            )
            .unwrap();

            let mut overlap_model = model();
            overlap_model.initial_height_m = r(81, 64);
            overlap_model.identity.guard_sha256 = "c".repeat(64);
            let overlapping = verify_constant_gravity_impact_enclosure(
                &overlap_model,
                &claim(&overlap_model, "overlap", r(1, 1), r(3, 2)),
            )
            .unwrap();
            assert_eq!(
                compare_verified_event_time_order(&first, &overlapping),
                Ok(CertifiedEventOrder::UnresolvedOverlap)
            );

            let mut nested_model = model();
            nested_model.identity.guard_sha256 = "d".repeat(64);
            let nested = verify_constant_gravity_impact_enclosure(
                &nested_model,
                &claim(&nested_model, "nested", r(7, 8), r(9, 8)),
            )
            .unwrap();
            assert_eq!(
                compare_verified_event_time_order(&first, &nested),
                Ok(CertifiedEventOrder::UnresolvedOverlap)
            );
        }

        #[test]
        fn verified_event_order_rejects_unrelated_trajectory_timelines() {
            let first_model = model();
            let first = verify_constant_gravity_impact_enclosure(
                &first_model,
                &claim(&first_model, "first", r(3, 4), r(5, 4)),
            )
            .unwrap();

            let mut second_model = model();
            second_model.initial_height_m = Rational::integer(4);
            second_model.identity.guard_sha256 = "c".repeat(64);
            second_model.identity.timeline_sha256 = "2".repeat(64);
            let second = verify_constant_gravity_impact_enclosure(
                &second_model,
                &claim(&second_model, "second", r(7, 4), r(9, 4)),
            )
            .unwrap();

            assert_eq!(
                compare_verified_event_time_order(&first, &second),
                Err(ImpactEnclosureError::IdentityMismatch("timeline_sha256"))
            );
        }

        #[test]
        fn verified_event_order_rejects_unrelated_clock_origins() {
            let first_model = model();
            let first = verify_constant_gravity_impact_enclosure(
                &first_model,
                &claim(&first_model, "first", r(3, 4), r(5, 4)),
            )
            .unwrap();

            let mut second_model = model();
            second_model.initial_height_m = Rational::integer(4);
            second_model.identity.guard_sha256 = "c".repeat(64);
            second_model.identity.time_origin_sha256 = "e".repeat(64);
            let second = verify_constant_gravity_impact_enclosure(
                &second_model,
                &claim(&second_model, "second", r(7, 4), r(9, 4)),
            )
            .unwrap();

            assert_eq!(
                compare_verified_event_time_order(&first, &second),
                Err(ImpactEnclosureError::IdentityMismatch("time_origin_sha256"))
            );
        }

        #[test]
        fn verified_event_order_rejects_incompatible_time_bases() {
            let first_model = model();
            let first = verify_constant_gravity_impact_enclosure(
                &first_model,
                &claim(&first_model, "first", r(3, 4), r(5, 4)),
            )
            .unwrap();

            let mut second_model = model();
            second_model.identity.time_basis_id = "milliseconds-si-v1".to_owned();
            second_model.gravity_m_per_time_squared = r(1, 500_000);
            second_model.initial_height_m = Rational::integer(4);
            second_model.identity.guard_sha256 = "c".repeat(64);
            let second = verify_constant_gravity_impact_enclosure(
                &second_model,
                &claim(
                    &second_model,
                    "second",
                    Rational::integer(1750),
                    Rational::integer(2250),
                ),
            )
            .unwrap();

            assert_eq!(
                compare_verified_event_time_order(&first, &second),
                Err(ImpactEnclosureError::IdentityMismatch("time_basis_id"))
            );
        }

        #[test]
        fn exact_arithmetic_overflow_is_reported_without_fallback() {
            let mut model = model();
            model.initial_height_m = Rational::integer(i128::MAX);
            model.gravity_m_per_time_squared = Rational::integer(i128::MAX);
            assert_eq!(
                verify_constant_gravity_impact_enclosure(
                    &model,
                    &claim(
                        &model,
                        "overflow",
                        Rational::integer(i128::MAX),
                        Rational::integer(i128::MAX),
                    ),
                ),
                Err(ImpactEnclosureError::Arithmetic(
                    HybridMathError::ArithmeticOverflow
                ))
            );
        }

        #[test]
        fn rejects_invalid_model_parameters_and_digests() {
            let mut invalid_height_model = model();
            invalid_height_model.initial_height_m = Rational::zero();
            let claim_value = claim(&invalid_height_model, "impact", r(3, 4), r(5, 4));
            assert_eq!(
                verify_constant_gravity_impact_enclosure(&invalid_height_model, &claim_value),
                Err(ImpactEnclosureError::InvalidInitialHeight)
            );

            let mut invalid_gravity_model = model();
            invalid_gravity_model.gravity_m_per_time_squared = Rational::zero();
            let claim_value = claim(&invalid_gravity_model, "impact", r(3, 4), r(5, 4));
            assert_eq!(
                verify_constant_gravity_impact_enclosure(&invalid_gravity_model, &claim_value),
                Err(ImpactEnclosureError::InvalidGravity)
            );

            let mut invalid_digest_model = model();
            invalid_digest_model.identity.guard_sha256 = "not-a-digest".to_owned();
            let claim_value = claim(&invalid_digest_model, "impact", r(3, 4), r(5, 4));
            assert_eq!(
                verify_constant_gravity_impact_enclosure(&invalid_digest_model, &claim_value),
                Err(ImpactEnclosureError::InvalidSha256("guard_sha256"))
            );
        }
    }
}
