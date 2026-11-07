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
