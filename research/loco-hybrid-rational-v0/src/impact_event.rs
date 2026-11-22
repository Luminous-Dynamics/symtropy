//! Exact rational event-time enclosure for a constant-gravity point impact.
//!
//! This module verifies a caller-supplied rational time interval against the
//! exact guard y(t) = h + v0*t - (g*t^2)/2. It proves existence and uniqueness
//! of a downward crossing when the guard changes sign and its derivative is
//! strictly negative throughout the interval. It does not locate a bracket or
//! certify a general ODE trajectory.

use std::cmp::Ordering;

use crate::{HybridMathError, Rational, RationalInterval};

/// Model identity that must be matched by the event-window claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactModelIdentity {
    pub model_build_sha256: String,
    pub guard_sha256: String,
    pub time_basis_id: String,
}

/// Exact constant-gravity point-impact model.
///
/// Time and length units are determined by the identity's declared time basis
/// and the caller's model contract. Gravity is a positive magnitude; the guard
/// uses y(t) = h + v0*t - g*t^2/2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantGravityImpactModel {
    pub identity: ImpactModelIdentity,
    pub initial_height_m: Rational,
    pub initial_velocity_m_s: Rational,
    pub gravity_m_s2: Rational,
}

/// A claimed interval for one event, bound to an exact model and guard identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactEventTimeClaim {
    pub event_id: String,
    pub model_build_sha256: String,
    pub guard_sha256: String,
    pub time_basis_id: String,
    pub event_time_s: RationalInterval,
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
/// construct proof-grade results by relabelling arbitrary f64 endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedImpactEventEnclosure {
    event_id: String,
    model_build_sha256: String,
    guard_sha256: String,
    time_basis_id: String,
    event_time_s: RationalInterval,
    lower_guard_value_m: Rational,
    upper_guard_value_m: Rational,
    derivative_upper_bound_m_s: Rational,
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

    pub fn event_time_s(&self) -> &RationalInterval {
        &self.event_time_s
    }

    /// Exact guard value at the lower endpoint.
    pub fn lower_guard_value_m(&self) -> &Rational {
        &self.lower_guard_value_m
    }

    /// Exact guard value at the upper endpoint.
    pub fn upper_guard_value_m(&self) -> &Rational {
        &self.upper_guard_value_m
    }

    /// Exact upper bound on y'(t) over the interval. Since y'(t)=v0-g*t
    /// decreases for positive g, its maximum occurs at the lower endpoint.
    pub fn derivative_upper_bound_m_s(&self) -> &Rational {
        &self.derivative_upper_bound_m_s
    }
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
    time_s: &Rational,
) -> Result<Rational, HybridMathError> {
    let time_squared = time_s.checked_mul(time_s)?;
    let gravity_time_squared = model.gravity_m_s2.checked_mul(&time_squared)?;
    let gravity_term = gravity_time_squared.checked_div(&Rational::integer(2))?;
    let velocity_term = model.initial_velocity_m_s.checked_mul(time_s)?;
    model
        .initial_height_m
        .checked_add(&velocity_term)?
        .checked_sub(&gravity_term)
}

fn evaluate_guard_derivative(
    model: &ConstantGravityImpactModel,
    time_s: &Rational,
) -> Result<Rational, HybridMathError> {
    let gravity_time = model.gravity_m_s2.checked_mul(time_s)?;
    model.initial_velocity_m_s.checked_sub(&gravity_time)
}

/// Independently verify a rational event-time bracket for the first downward
/// impact of the declared constant-gravity point-mass model.
///
/// The proof uses exact rational guard evaluations at both endpoints and an
/// exact derivative upper bound. Since g > 0, y'(t)=v0-g*t is decreasing; if
/// y'(t_lo) < 0 then the guard is strictly decreasing on the entire interval.
/// Combined with y(t_lo) >= 0 and y(t_hi) <= 0, this proves one and only one
/// downward crossing in the closed interval (including exact endpoint roots).
///
/// A singleton interval is accepted only when its sole endpoint is an exact
/// guard root and the derivative is strictly negative. Arithmetic overflow
/// returns an error; no f64 fallback, rounding or saturation is used.
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

    if claim.model_build_sha256 != model.identity.model_build_sha256 {
        return Err(ImpactEnclosureError::IdentityMismatch("model_build_sha256"));
    }
    if claim.guard_sha256 != model.identity.guard_sha256 {
        return Err(ImpactEnclosureError::IdentityMismatch("guard_sha256"));
    }
    if claim.time_basis_id != model.identity.time_basis_id {
        return Err(ImpactEnclosureError::IdentityMismatch("time_basis_id"));
    }

    let zero = Rational::zero();
    if model.initial_height_m.checked_cmp(&zero)? != Ordering::Greater {
        return Err(ImpactEnclosureError::InvalidInitialHeight);
    }
    if model.gravity_m_s2.checked_cmp(&zero)? != Ordering::Greater {
        return Err(ImpactEnclosureError::InvalidGravity);
    }

    let lower = claim.event_time_s.lower();
    let upper = claim.event_time_s.upper();
    if lower.checked_cmp(&zero)? == Ordering::Less {
        return Err(ImpactEnclosureError::NegativeTime);
    }

    let lower_guard = evaluate_guard(model, lower)?;
    let upper_guard = evaluate_guard(model, upper)?;
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

    if lower == upper && lower_guard != zero {
        return Err(ImpactEnclosureError::SingletonIsNotExactEvent);
    }

    Ok(VerifiedImpactEventEnclosure {
        event_id: claim.event_id.clone(),
        model_build_sha256: claim.model_build_sha256.clone(),
        guard_sha256: claim.guard_sha256.clone(),
        time_basis_id: claim.time_basis_id.clone(),
        event_time_s: claim.event_time_s.clone(),
        lower_guard_value_m: lower_guard,
        upper_guard_value_m: upper_guard,
        derivative_upper_bound_m_s: derivative_upper_bound,
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
            },
            initial_height_m: Rational::integer(1),
            initial_velocity_m_s: Rational::zero(),
            gravity_m_s2: Rational::integer(2),
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
            event_time_s: RationalInterval::new(lower, upper).unwrap(),
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
        assert_eq!(verified.derivative_upper_bound_m_s(), &r(-3, 2));
        assert_eq!(verified.event_time_s().lower(), &r(3, 4));
        assert_eq!(verified.event_time_s().upper(), &r(5, 4));
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
        model.initial_velocity_m_s = Rational::integer(2);
        // The bracket contains the first positive root, but y'(0)=+2. The
        // declared interval does not prove a downward crossing throughout.
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
        let seconds = verify_constant_gravity_impact_enclosure(&seconds_model, &seconds_claim)
            .unwrap();

        let mut milliseconds_model = model();
        milliseconds_model.identity.time_basis_id = "milliseconds-si-v1".to_owned();
        milliseconds_model.gravity_m_s2 = r(1, 500_000);
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
        assert_eq!(seconds.derivative_upper_bound_m_s(), &r(-3, 2));
        assert_eq!(
            milliseconds.derivative_upper_bound_m_s(),
            &r(-3, 2000)
        );
    }

    #[test]
    fn exact_arithmetic_overflow_is_reported_without_fallback() {
        let mut model = model();
        model.initial_height_m = Rational::integer(i128::MAX);
        model.gravity_m_s2 = Rational::integer(i128::MAX);
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
        let mut model = model();
        model.initial_height_m = Rational::zero();
        let claim_value = claim(&model, "impact", r(3, 4), r(5, 4));
        assert_eq!(
            verify_constant_gravity_impact_enclosure(&model, &claim_value),
            Err(ImpactEnclosureError::InvalidInitialHeight)
        );

        let mut model = model();
        model.gravity_m_s2 = Rational::zero();
        let claim_value = claim(&model, "impact", r(3, 4), r(5, 4));
        assert_eq!(
            verify_constant_gravity_impact_enclosure(&model, &claim_value),
            Err(ImpactEnclosureError::InvalidGravity)
        );

        let mut model = model();
        model.identity.guard_sha256 = "not-a-digest".to_owned();
        let claim_value = claim(&model, "impact", r(3, 4), r(5, 4));
        assert_eq!(
            verify_constant_gravity_impact_enclosure(&model, &claim_value),
            Err(ImpactEnclosureError::InvalidSha256("guard_sha256"))
        );
    }
}
