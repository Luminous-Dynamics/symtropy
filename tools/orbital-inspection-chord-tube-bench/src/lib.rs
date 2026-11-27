#![deny(unsafe_code)]

pub const RECEIPT_SCHEMA: &str = "luminous.orbital-inspection-chord-tube.receipt.v0.1";
pub const COORDINATE_SCALE: i128 = 1_000_000;
pub const CURVATURE_DENOMINATOR: i128 = 8_000_000_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    ConditionalClear,
    NotProven,
    InvalidInput,
    Overflow,
}

impl Disposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConditionalClear => "CONDITIONAL_CLEAR",
            Self::NotProven => "NOT_PROVEN",
            Self::InvalidInput => "INVALID_INPUT",
            Self::Overflow => "OVERFLOW",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseInput {
    pub case_id: String,
    pub start: [i64; 3],
    pub end: [i64; 3],
    pub acceleration_bound_scaled_per_s2: i64,
    pub duration_ns: u64,
    pub endpoint_error_bound_scaled: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseOutput {
    pub case_id: String,
    pub disposition: Disposition,
    pub curvature_error_scaled: Option<i128>,
    pub total_tube_error_scaled: Option<i128>,
}

fn ceil_div_nonnegative(numerator: i128, denominator: i128) -> Option<i128> {
    if numerator < 0 || denominator <= 0 {
        return None;
    }
    let quotient = numerator / denominator;
    if numerator.is_multiple_of(denominator) {
        Some(quotient)
    } else {
        quotient.checked_add(1)
    }
}

fn dot(left: [i128; 3], right: [i128; 3]) -> Option<i128> {
    let mut sum = 0_i128;
    for (left_component, right_component) in left.into_iter().zip(right) {
        sum = sum.checked_add(left_component.checked_mul(right_component)?)?;
    }
    Some(sum)
}

fn strictly_outside_chord_tube(
    start: [i128; 3],
    end: [i128; 3],
    total_error_scaled: i128,
) -> Option<bool> {
    let direction = [
        end[0].checked_sub(start[0])?,
        end[1].checked_sub(start[1])?,
        end[2].checked_sub(start[2])?,
    ];
    let direction_squared = dot(direction, direction)?;
    let start_squared = dot(start, start)?;
    let threshold = COORDINATE_SCALE.checked_add(total_error_scaled)?;
    let threshold_squared = threshold.checked_mul(threshold)?;

    if direction_squared == 0 {
        return Some(start_squared > threshold_squared);
    }

    let projection = dot(start, direction)?;
    if projection >= 0 {
        return Some(start_squared > threshold_squared);
    }

    if projection <= direction_squared.checked_neg()? {
        let end_squared = dot(end, end)?;
        return Some(end_squared > threshold_squared);
    }

    let projection_squared = projection.checked_mul(projection)?;
    let distance_numerator = start_squared
        .checked_mul(direction_squared)?
        .checked_sub(projection_squared)?;
    if distance_numerator < 0 {
        return None;
    }
    let comparison_bound = threshold_squared.checked_mul(direction_squared)?;
    Some(distance_numerator > comparison_bound)
}

/// Checks a vector-valued C2 curve against a unit keep-out sphere in
/// normalized coordinates, conditional on the supplied endpoint-error and
/// second-derivative-norm bounds being valid for the entire interval.
///
/// The chord interpolation error bound is M*h^2/8. This function verifies
/// the exact integer comparison after outward ceiling of that term. It does
/// not derive or authenticate M, endpoint enclosures, frame transforms, or
/// the physical dynamics model. Therefore CONDITIONAL_CLEAR is never a
/// standalone spacecraft safety claim.
pub fn verify(case: &CaseInput) -> CaseOutput {
    let invalid = |disposition| CaseOutput {
        case_id: case.case_id.clone(),
        disposition,
        curvature_error_scaled: None,
        total_tube_error_scaled: None,
    };

    if case.case_id.trim().is_empty()
        || case.duration_ns == 0
        || case.acceleration_bound_scaled_per_s2 < 0
        || case.endpoint_error_bound_scaled < 0
    {
        return invalid(Disposition::InvalidInput);
    }

    let acceleration = i128::from(case.acceleration_bound_scaled_per_s2);
    let duration = i128::from(case.duration_ns);
    let endpoint_error = i128::from(case.endpoint_error_bound_scaled);

    let curvature_numerator = match acceleration
        .checked_mul(duration)
        .and_then(|value| value.checked_mul(duration))
    {
        Some(value) => value,
        None => return invalid(Disposition::Overflow),
    };

    let curvature_error = match ceil_div_nonnegative(curvature_numerator, CURVATURE_DENOMINATOR) {
        Some(value) => value,
        None => return invalid(Disposition::Overflow),
    };

    let total_error = match curvature_error.checked_add(endpoint_error) {
        Some(value) => value,
        None => return invalid(Disposition::Overflow),
    };

    let start = case.start.map(i128::from);
    let end = case.end.map(i128::from);
    let outside = match strictly_outside_chord_tube(start, end, total_error) {
        Some(value) => value,
        None => {
            return CaseOutput {
                case_id: case.case_id.clone(),
                disposition: Disposition::Overflow,
                curvature_error_scaled: Some(curvature_error),
                total_tube_error_scaled: Some(total_error),
            };
        }
    };

    CaseOutput {
        case_id: case.case_id.clone(),
        disposition: if outside {
            Disposition::ConditionalClear
        } else {
            Disposition::NotProven
        },
        curvature_error_scaled: Some(curvature_error),
        total_tube_error_scaled: Some(total_error),
    }
}

#[cfg(test)]
mod tests {
    use super::{CaseInput, Disposition, verify};

    fn case(
        start: [i64; 3],
        end: [i64; 3],
        acceleration: i64,
        duration_ns: u64,
        endpoint_error: i64,
    ) -> CaseInput {
        CaseInput {
            case_id: "unit-test".to_owned(),
            start,
            end,
            acceleration_bound_scaled_per_s2: acceleration,
            duration_ns,
            endpoint_error_bound_scaled: endpoint_error,
        }
    }

    #[test]
    fn stationary_point_outside_is_conditionally_clear() {
        let result = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            0,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.disposition, Disposition::ConditionalClear);
    }

    #[test]
    fn chord_through_keepout_is_not_proven_clear() {
        let result = verify(&case(
            [-2_000_000, 0, 0],
            [2_000_000, 0, 0],
            0,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.disposition, Disposition::NotProven);
    }

    #[test]
    fn exact_tangency_is_not_a_clearance_proof() {
        let result = verify(&case(
            [1_000_000, 1_000_000, 0],
            [1_000_000, -1_000_000, 0],
            0,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.disposition, Disposition::NotProven);
    }

    #[test]
    fn curvature_error_is_rounded_outward() {
        let result = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            4_000_000,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.curvature_error_scaled, Some(500_000));
        assert_eq!(result.disposition, Disposition::ConditionalClear);
    }

    #[test]
    fn curvature_can_remove_an_apparent_chord_clearance() {
        let result = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            16_000_000,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.curvature_error_scaled, Some(2_000_000));
        assert_eq!(result.disposition, Disposition::NotProven);
    }

    #[test]
    fn endpoint_uncertainty_is_added_to_the_tube() {
        let result = verify(&case(
            [1_200_000, 0, 0],
            [1_200_000, 0, 0],
            0,
            1_000_000_000,
            300_000,
        ));
        assert_eq!(result.total_tube_error_scaled, Some(300_000));
        assert_eq!(result.disposition, Disposition::NotProven);
    }

    #[test]
    fn closest_point_can_be_at_either_endpoint() {
        let start_closest = verify(&case(
            [2_000_000, 0, 0],
            [3_000_000, 0, 0],
            0,
            1_000_000_000,
            0,
        ));
        let end_closest = verify(&case(
            [3_000_000, 0, 0],
            [2_000_000, 0, 0],
            0,
            1_000_000_000,
            0,
        ));
        assert_eq!(start_closest.disposition, Disposition::ConditionalClear);
        assert_eq!(end_closest.disposition, Disposition::ConditionalClear);
    }

    #[test]
    fn three_dimensional_segment_distance_is_used() {
        let result = verify(&case(
            [2_000_000, 2_000_000, 1_000_000],
            [2_000_000, -2_000_000, 1_000_000],
            0,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.disposition, Disposition::ConditionalClear);
    }

    #[test]
    fn zero_duration_and_negative_bounds_are_invalid() {
        let zero_duration = verify(&case([2_000_000, 0, 0], [2_000_000, 0, 0], 0, 0, 0));
        let negative_acceleration = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            -1,
            1_000_000_000,
            0,
        ));
        let negative_endpoint_error = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            0,
            1_000_000_000,
            -1,
        ));
        assert_eq!(zero_duration.disposition, Disposition::InvalidInput);
        assert_eq!(negative_acceleration.disposition, Disposition::InvalidInput);
        assert_eq!(
            negative_endpoint_error.disposition,
            Disposition::InvalidInput
        );
    }

    #[test]
    fn arithmetic_overflow_abstains() {
        let result = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            i64::MAX,
            u64::MAX,
            0,
        ));
        assert_eq!(result.disposition, Disposition::Overflow);
    }

    #[test]
    fn a_strictly_positive_one_micro_unit_margin_can_pass() {
        let result = verify(&case(
            [1_000_001, 0, 0],
            [1_000_001, 0, 0],
            0,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.disposition, Disposition::ConditionalClear);
    }

    #[test]
    fn exact_boundary_after_endpoint_uncertainty_does_not_pass() {
        let result = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            0,
            1_000_000_000,
            1_000_000,
        ));
        assert_eq!(result.disposition, Disposition::NotProven);
    }

    #[test]
    fn endpoint_reversal_and_axis_isometry_preserve_clearance() {
        let original = case(
            [2_000_000, 2_000_000, 1_000_000],
            [2_000_000, -2_000_000, 1_000_000],
            4_000_000,
            1_000_000_000,
            50_000,
        );
        let reversed = case(
            original.end,
            original.start,
            original.acceleration_bound_scaled_per_s2,
            original.duration_ns,
            original.endpoint_error_bound_scaled,
        );
        // (x,y,z) -> (z,x,-y): an integer orthogonal transformation.
        let transformed = case(
            [1_000_000, 2_000_000, -2_000_000],
            [1_000_000, 2_000_000, 2_000_000],
            original.acceleration_bound_scaled_per_s2,
            original.duration_ns,
            original.endpoint_error_bound_scaled,
        );
        let baseline = verify(&original);
        assert_eq!(verify(&reversed).disposition, baseline.disposition);
        assert_eq!(verify(&transformed).disposition, baseline.disposition);
        assert_eq!(
            verify(&reversed).curvature_error_scaled,
            baseline.curvature_error_scaled
        );
        assert_eq!(
            verify(&transformed).total_tube_error_scaled,
            baseline.total_tube_error_scaled
        );
    }

    #[test]
    fn enlarging_uncertainty_or_curvature_never_rescues_a_boundary_case() {
        let base = case([1_100_000, 0, 0], [1_100_000, 0, 0], 0, 1_000_000_000, 0);
        let with_endpoint_error = case(base.start, base.end, 0, base.duration_ns, 100_000);
        let with_curvature = case(base.start, base.end, 800_000, base.duration_ns, 0);
        assert_eq!(verify(&base).disposition, Disposition::ConditionalClear);
        assert_eq!(
            verify(&with_endpoint_error).disposition,
            Disposition::NotProven
        );
        assert_eq!(verify(&with_curvature).disposition, Disposition::NotProven);
    }

    #[test]
    fn non_integral_curvature_remainder_is_rounded_outward() {
        let result = verify(&case(
            [2_000_000, 0, 0],
            [2_000_000, 0, 0],
            1,
            1_000_000_000,
            0,
        ));
        assert_eq!(result.curvature_error_scaled, Some(1));
        assert_eq!(result.disposition, Disposition::ConditionalClear);
    }
}
