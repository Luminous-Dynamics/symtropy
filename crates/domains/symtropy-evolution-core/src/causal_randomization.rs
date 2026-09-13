use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisAuthorityRef,
    AnalysisContentDigest, AnalysisMethodId, BinaryComparisonGroup, CausalRandomizationTestId,
    CausalSelectionIdentificationDigest, CausalSelectionTarget, CausalViabilityRiskEffectDigest,
    CompleteRandomizationReferenceDigest, MaterializedOutcome, SelectionDesignClass,
    ValidatedCausalSelectionEffect, ValidatedCompleteRandomizationReference,
    ValidatedSelectionAnalysisFrame,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const CAUSAL_RANDOMIZATION_TEST_VERSION: u32 = 1;
pub const CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS: u64 = 100_000;

const RANDOMIZATION_TEST_DOMAIN: &[u8] =
    b"symtropy:evolution:causal-randomization-test:v1\0";
const METHOD_DOMAIN: &[u8] = b"symtropy:evolution:causal-randomization-method-authority:v1\0";
const METHOD_ID: &str = "complete-randomization-sharp-null-viability-risk-difference-v1";
const METHOD_SPEC: &[u8] = b"requires current SEL-07D1 complete fixed-count randomization reference plus current SEL-07C2 causal viability-risk point effect and exact current frame; sharp null of no individual assignment effect on viability; enumerate every D1-permitted assignment when support <=100000; hold observed viability outcomes fixed; statistic comparison-minus-reference death-risk difference; two-sided ordering absolute statistic magnitude at least observed; exact rational probability; no approximation, confidence interval, selection coefficient, fitness, or adaptation claim";

pub fn causal_randomization_test_method_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(METHOD_DOMAIN);
    put_text(&mut digest, METHOD_ID);
    put_u64(&mut digest, 1);
    put_u64(&mut digest, METHOD_SPEC.len() as u64);
    digest.update(METHOD_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(METHOD_ID).expect("built-in randomization method ID must remain valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalSharpNull {
    NoIndividualAssignmentEffectOnViability,
}

impl CausalSharpNull {
    fn tag(self) -> u8 {
        match self {
            Self::NoIndividualAssignmentEffectOnViability => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalRandomizationStatistic {
    ComparisonMinusReferenceViabilityRiskDifference,
}

impl CausalRandomizationStatistic {
    fn tag(self) -> u8 {
        match self {
            Self::ComparisonMinusReferenceViabilityRiskDifference => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalTwoSidedOrdering {
    AbsoluteStatisticAtLeastObserved,
}

impl CausalTwoSidedOrdering {
    fn tag(self) -> u8 {
        match self {
            Self::AbsoluteStatisticAtLeastObserved => 0,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ExactRandomizationStatistic {
    pub negative: bool,
    pub numerator: u64,
    pub denominator: u64,
}

impl ExactRandomizationStatistic {
    fn new(numerator: i64, denominator: u64) -> Result<Self, CausalRandomizationError> {
        if denominator == 0 {
            return Err(CausalRandomizationError::ArithmeticInvariant);
        }
        let magnitude = numerator.unsigned_abs();
        let divisor = gcd(magnitude, denominator);
        Ok(Self {
            negative: numerator < 0 && magnitude != 0,
            numerator: magnitude / divisor,
            denominator: denominator / divisor,
        })
    }

    fn validate_canonical(self) -> Result<(), CausalRandomizationError> {
        if self.denominator == 0
            || (self.numerator == 0 && self.negative)
            || gcd(self.numerator, self.denominator) != 1
        {
            return Err(CausalRandomizationError::ArithmeticInvariant);
        }
        Ok(())
    }

    fn update_digest(self, digest: &mut Sha256) {
        digest.update([u8::from(self.negative)]);
        put_u64(digest, self.numerator);
        put_u64(digest, self.denominator);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactRandomizationProbability {
    pub numerator: u64,
    pub denominator: u64,
}

impl ExactRandomizationProbability {
    fn new(numerator: u64, denominator: u64) -> Result<Self, CausalRandomizationError> {
        if denominator == 0 || numerator > denominator {
            return Err(CausalRandomizationError::ProbabilityInvariant);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn validate_canonical(self) -> Result<(), CausalRandomizationError> {
        if self.denominator == 0
            || self.numerator > self.denominator
            || gcd(self.numerator, self.denominator) != 1
        {
            return Err(CausalRandomizationError::ProbabilityInvariant);
        }
        Ok(())
    }

    fn update_digest(self, digest: &mut Sha256) {
        put_u64(digest, self.numerator);
        put_u64(digest, self.denominator);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomizationStatisticMultiplicity {
    pub statistic: ExactRandomizationStatistic,
    pub assignments: u64,
}

impl RandomizationStatisticMultiplicity {
    fn update_digest(self, digest: &mut Sha256) {
        self.statistic.update_digest(digest);
        put_u64(digest, self.assignments);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalRandomizationTestResult {
    test_version: u32,
    pub test_id: CausalRandomizationTestId,
    randomization_reference_digest: CompleteRandomizationReferenceDigest,
    causal_effect_digest: CausalViabilityRiskEffectDigest,
    identification_digest: CausalSelectionIdentificationDigest,
    frame_digest: crate::ExplicitSelectionAnalysisFrameDigest,
    pub source_design_class: SelectionDesignClass,
    pub target: CausalSelectionTarget,
    original_denominator: u64,
    estimand_denominator: u64,
    pub sharp_null: CausalSharpNull,
    pub statistic: CausalRandomizationStatistic,
    pub two_sided_ordering: CausalTwoSidedOrdering,
    pub observed_statistic: ExactRandomizationStatistic,
    pub support_size: u64,
    pub extreme_assignments: u64,
    pub distribution: Vec<RandomizationStatisticMultiplicity>,
    pub two_sided_randomization_probability: ExactRandomizationProbability,
    pub method: AnalysisAuthorityRef,
}

impl CausalRandomizationTestResult {
    pub fn randomization_reference_digest(&self) -> CompleteRandomizationReferenceDigest {
        self.randomization_reference_digest
    }

    pub fn causal_effect_digest(&self) -> CausalViabilityRiskEffectDigest {
        self.causal_effect_digest
    }

    pub fn identification_digest(&self) -> CausalSelectionIdentificationDigest {
        self.identification_digest
    }

    pub fn frame_digest(&self) -> crate::ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn original_denominator(&self) -> u64 {
        self.original_denominator
    }

    pub fn estimand_denominator(&self) -> u64 {
        self.estimand_denominator
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CausalRandomizationTestDigest, CausalRandomizationError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(RANDOMIZATION_TEST_DOMAIN);
        put_u32(&mut digest, self.test_version);
        put_text(&mut digest, self.test_id.as_str());
        digest.update(self.randomization_reference_digest.as_bytes());
        digest.update(self.causal_effect_digest.as_bytes());
        digest.update(self.identification_digest.as_bytes());
        digest.update(self.frame_digest.as_bytes());
        digest.update([
            design_class_tag(self.source_design_class),
            target_tag(self.target),
            self.sharp_null.tag(),
            self.statistic.tag(),
            self.two_sided_ordering.tag(),
        ]);
        put_u64(&mut digest, self.original_denominator);
        put_u64(&mut digest, self.estimand_denominator);
        self.observed_statistic.update_digest(&mut digest);
        put_u64(&mut digest, self.support_size);
        put_u64(&mut digest, self.extreme_assignments);
        put_u64(&mut digest, self.distribution.len() as u64);
        for point in &self.distribution {
            point.update_digest(&mut digest);
        }
        self.two_sided_randomization_probability
            .update_digest(&mut digest);
        update_authority_digest(&mut digest, &self.method);
        Ok(CausalRandomizationTestDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), CausalRandomizationError> {
        if self.test_version != CAUSAL_RANDOMIZATION_TEST_VERSION {
            return Err(CausalRandomizationError::UnsupportedTestVersion(
                self.test_version,
            ));
        }
        if self.source_design_class != SelectionDesignClass::RandomizedInterventional {
            return Err(CausalRandomizationError::UnsupportedDesignClass(
                self.source_design_class,
            ));
        }
        if self.target != CausalSelectionTarget::BinaryViabilityRiskDifference {
            return Err(CausalRandomizationError::UnsupportedTarget);
        }
        if self.sharp_null != CausalSharpNull::NoIndividualAssignmentEffectOnViability
            || self.statistic
                != CausalRandomizationStatistic::ComparisonMinusReferenceViabilityRiskDifference
            || self.two_sided_ordering
                != CausalTwoSidedOrdering::AbsoluteStatisticAtLeastObserved
        {
            return Err(CausalRandomizationError::MethodSemanticMismatch);
        }
        if self.method != causal_randomization_test_method_v1() {
            return Err(CausalRandomizationError::MethodAuthorityMismatch);
        }
        if self.estimand_denominator == 0
            || self.estimand_denominator > self.original_denominator
        {
            return Err(CausalRandomizationError::DenominatorInvariant);
        }
        if self.support_size == 0
            || self.support_size > CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS
        {
            return Err(CausalRandomizationError::ExactEnumerationSupportTooLarge {
                assignments: self.support_size,
                max_assignments: CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS,
            });
        }
        if self.extreme_assignments == 0 || self.extreme_assignments > self.support_size {
            return Err(CausalRandomizationError::ProbabilityInvariant);
        }
        self.observed_statistic.validate_canonical()?;
        self.two_sided_randomization_probability
            .validate_canonical()?;
        if self.distribution.is_empty() {
            return Err(CausalRandomizationError::DistributionInvariant);
        }
        if self
            .distribution
            .windows(2)
            .any(|window| window[0].statistic >= window[1].statistic)
        {
            return Err(CausalRandomizationError::NonCanonicalDistributionOrder);
        }

        let mut total_assignments = 0u64;
        let mut extreme_assignments = 0u64;
        let mut observed_present = false;
        for point in &self.distribution {
            point.statistic.validate_canonical()?;
            if point.assignments == 0 {
                return Err(CausalRandomizationError::DistributionInvariant);
            }
            total_assignments = total_assignments
                .checked_add(point.assignments)
                .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
            if at_least_as_extreme(point.statistic, self.observed_statistic) {
                extreme_assignments = extreme_assignments
                    .checked_add(point.assignments)
                    .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
            }
            if point.statistic == self.observed_statistic {
                observed_present = true;
            }
        }
        if total_assignments != self.support_size
            || extreme_assignments != self.extreme_assignments
            || !observed_present
        {
            return Err(CausalRandomizationError::DistributionInvariant);
        }
        let expected_probability =
            ExactRandomizationProbability::new(self.extreme_assignments, self.support_size)?;
        if expected_probability != self.two_sided_randomization_probability {
            return Err(CausalRandomizationError::ProbabilityInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausalRandomizationTestDigest([u8; 32]);

impl CausalRandomizationTestDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CausalRandomizationTestDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CausalRandomizationTestDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for CausalRandomizationTestDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated causal randomization evidence should remain bound to current frame, causal effect, and assignment reference"]
pub struct ValidatedCausalRandomizationTest<'a> {
    result: &'a CausalRandomizationTestResult,
    result_digest: CausalRandomizationTestDigest,
    randomization_reference_digest: CompleteRandomizationReferenceDigest,
    causal_effect_digest: CausalViabilityRiskEffectDigest,
    identification_digest: CausalSelectionIdentificationDigest,
    frame_digest: crate::ExplicitSelectionAnalysisFrameDigest,
}

impl<'a> ValidatedCausalRandomizationTest<'a> {
    pub fn validate_current(
        result: &'a CausalRandomizationTestResult,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        effect: &ValidatedCausalSelectionEffect<'_>,
        reference: &ValidatedCompleteRandomizationReference<'_>,
    ) -> Result<Self, CausalRandomizationError> {
        result.validate_local()?;
        let recomputed = execute_exact_causal_randomization_test(
            result.test_id.clone(),
            frame,
            effect,
            reference,
        )?;
        if recomputed != *result {
            return Err(CausalRandomizationError::TestReplayMismatch);
        }
        Ok(Self {
            result,
            result_digest: result.canonical_digest()?,
            randomization_reference_digest: reference.reference_digest(),
            causal_effect_digest: effect.effect_digest(),
            identification_digest: effect.identification_digest(),
            frame_digest: frame.frame_digest(),
        })
    }

    pub fn result(&self) -> &'a CausalRandomizationTestResult {
        self.result
    }

    pub fn result_digest(&self) -> CausalRandomizationTestDigest {
        self.result_digest
    }

    pub fn randomization_reference_digest(&self) -> CompleteRandomizationReferenceDigest {
        self.randomization_reference_digest
    }

    pub fn causal_effect_digest(&self) -> CausalViabilityRiskEffectDigest {
        self.causal_effect_digest
    }

    pub fn identification_digest(&self) -> CausalSelectionIdentificationDigest {
        self.identification_digest
    }

    pub fn frame_digest(&self) -> crate::ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }
}

pub fn execute_exact_causal_randomization_test(
    test_id: CausalRandomizationTestId,
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    effect: &ValidatedCausalSelectionEffect<'_>,
    reference: &ValidatedCompleteRandomizationReference<'_>,
) -> Result<CausalRandomizationTestResult, CausalRandomizationError> {
    validate_capability_chain(frame, effect, reference)?;

    let assignment_reference = reference.reference();
    if assignment_reference.support_size > CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS {
        return Err(CausalRandomizationError::ExactEnumerationSupportTooLarge {
            assignments: assignment_reference.support_size,
            max_assignments: CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS,
        });
    }

    let outcomes = viability_outcomes(frame)?;
    let n = outcomes.len();
    let k = usize::try_from(assignment_reference.comparison_count)
        .map_err(|_| CausalRandomizationError::ArithmeticOverflow)?;
    if k == 0 || k >= n {
        return Err(CausalRandomizationError::AssignmentCountInvariant);
    }

    let observed_statistic = statistic_for_assignment(
        &outcomes,
        assignment_reference
            .realized_assignments
            .iter()
            .map(|assignment| assignment.group == BinaryComparisonGroup::Comparison),
    )?;
    let effect_statistic = statistic_from_effect(effect)?;
    if observed_statistic != effect_statistic {
        return Err(CausalRandomizationError::ObservedEffectMismatch);
    }

    let mut multiplicities: BTreeMap<ExactRandomizationStatistic, u64> = BTreeMap::new();
    let mut total_assignments = 0u64;
    let mut extreme_assignments = 0u64;

    for_each_combination(n, k, |comparison_indices| {
        let mut assigned = vec![false; n];
        for index in comparison_indices {
            assigned[*index] = true;
        }
        let statistic = statistic_for_assignment(&outcomes, assigned.into_iter())?;
        let entry = multiplicities.entry(statistic).or_insert(0);
        *entry = entry
            .checked_add(1)
            .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
        total_assignments = total_assignments
            .checked_add(1)
            .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
        if at_least_as_extreme(statistic, observed_statistic) {
            extreme_assignments = extreme_assignments
                .checked_add(1)
                .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
        }
        Ok(())
    })?;

    if total_assignments != assignment_reference.support_size {
        return Err(CausalRandomizationError::AssignmentSupportMismatch {
            enumerated: total_assignments,
            declared: assignment_reference.support_size,
        });
    }

    let distribution = multiplicities
        .into_iter()
        .map(|(statistic, assignments)| RandomizationStatisticMultiplicity {
            statistic,
            assignments,
        })
        .collect::<Vec<_>>();

    let result = CausalRandomizationTestResult {
        test_version: CAUSAL_RANDOMIZATION_TEST_VERSION,
        test_id,
        randomization_reference_digest: reference.reference_digest(),
        causal_effect_digest: effect.effect_digest(),
        identification_digest: effect.identification_digest(),
        frame_digest: frame.frame_digest(),
        source_design_class: effect.effect().source_design_class,
        target: effect.effect().target,
        original_denominator: effect.effect().original_denominator(),
        estimand_denominator: effect.effect().estimand_denominator(),
        sharp_null: CausalSharpNull::NoIndividualAssignmentEffectOnViability,
        statistic: CausalRandomizationStatistic::ComparisonMinusReferenceViabilityRiskDifference,
        two_sided_ordering: CausalTwoSidedOrdering::AbsoluteStatisticAtLeastObserved,
        observed_statistic,
        support_size: total_assignments,
        extreme_assignments,
        distribution,
        two_sided_randomization_probability: ExactRandomizationProbability::new(
            extreme_assignments,
            total_assignments,
        )?,
        method: causal_randomization_test_method_v1(),
    };
    result.validate_local()?;
    Ok(result)
}

fn validate_capability_chain(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    effect: &ValidatedCausalSelectionEffect<'_>,
    reference: &ValidatedCompleteRandomizationReference<'_>,
) -> Result<(), CausalRandomizationError> {
    if effect.frame_digest() != frame.frame_digest() {
        return Err(CausalRandomizationError::EffectFrameMismatch);
    }
    if reference.frame_digest() != frame.frame_digest() {
        return Err(CausalRandomizationError::ReferenceFrameMismatch);
    }
    if reference.identification_digest() != effect.identification_digest() {
        return Err(CausalRandomizationError::IdentificationMismatch);
    }
    if effect.effect().source_design_class != SelectionDesignClass::RandomizedInterventional {
        return Err(CausalRandomizationError::UnsupportedDesignClass(
            effect.effect().source_design_class,
        ));
    }
    if effect.effect().target != CausalSelectionTarget::BinaryViabilityRiskDifference {
        return Err(CausalRandomizationError::UnsupportedTarget);
    }
    if reference.reference().source_design_class != SelectionDesignClass::RandomizedInterventional
        || reference.reference().causal_target
            != CausalSelectionTarget::BinaryViabilityRiskDifference
    {
        return Err(CausalRandomizationError::ReferenceSemanticMismatch);
    }
    if reference.reference().original_denominator() != effect.effect().original_denominator()
        || reference.reference().estimand_denominator() != effect.effect().estimand_denominator()
        || frame.frame().original_denominator() != effect.effect().original_denominator()
        || frame.frame().estimand_denominator() != effect.effect().estimand_denominator()
    {
        return Err(CausalRandomizationError::DenominatorInvariant);
    }
    if reference.reference().reference_count != effect.effect().reference_support.total
        || reference.reference().comparison_count != effect.effect().comparison_support.total
    {
        return Err(CausalRandomizationError::AssignmentCountInvariant);
    }
    Ok(())
}

fn viability_outcomes(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
) -> Result<Vec<bool>, CausalRandomizationError> {
    frame
        .frame()
        .rows
        .iter()
        .map(|row| match &row.outcome {
            MaterializedOutcome::ViabilityObserved(crate::ViabilityConsequence::SurvivedWindow) => {
                Ok(false)
            }
            MaterializedOutcome::ViabilityObserved(crate::ViabilityConsequence::DiedDuringWindow) => {
                Ok(true)
            }
            _ => Err(CausalRandomizationError::UnsupportedOutcome),
        })
        .collect()
}

fn statistic_for_assignment(
    outcomes: &[bool],
    comparison_membership: impl IntoIterator<Item = bool>,
) -> Result<ExactRandomizationStatistic, CausalRandomizationError> {
    let mut reference_total = 0u64;
    let mut reference_died = 0u64;
    let mut comparison_total = 0u64;
    let mut comparison_died = 0u64;
    let mut seen = 0usize;

    for (died, comparison) in outcomes
        .iter()
        .copied()
        .zip(comparison_membership.into_iter())
    {
        seen = seen
            .checked_add(1)
            .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
        if comparison {
            comparison_total = comparison_total
                .checked_add(1)
                .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
            comparison_died = comparison_died
                .checked_add(u64::from(died))
                .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
        } else {
            reference_total = reference_total
                .checked_add(1)
                .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
            reference_died = reference_died
                .checked_add(u64::from(died))
                .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
        }
    }
    if seen != outcomes.len() || reference_total == 0 || comparison_total == 0 {
        return Err(CausalRandomizationError::AssignmentCountInvariant);
    }
    exact_risk_difference(
        reference_total,
        reference_died,
        comparison_total,
        comparison_died,
    )
}

fn statistic_from_effect(
    effect: &ValidatedCausalSelectionEffect<'_>,
) -> Result<ExactRandomizationStatistic, CausalRandomizationError> {
    let value = effect
        .effect()
        .comparison_minus_reference_causal_risk_difference;
    value_to_statistic(value.negative, value.numerator, value.denominator)
}

fn value_to_statistic(
    negative: bool,
    numerator: u64,
    denominator: u64,
) -> Result<ExactRandomizationStatistic, CausalRandomizationError> {
    if denominator == 0 || (negative && numerator == 0) {
        return Err(CausalRandomizationError::ArithmeticInvariant);
    }
    let divisor = gcd(numerator, denominator);
    let statistic = ExactRandomizationStatistic {
        negative,
        numerator: numerator / divisor,
        denominator: denominator / divisor,
    };
    statistic.validate_canonical()?;
    Ok(statistic)
}

fn exact_risk_difference(
    reference_total: u64,
    reference_died: u64,
    comparison_total: u64,
    comparison_died: u64,
) -> Result<ExactRandomizationStatistic, CausalRandomizationError> {
    let comparison_cross = comparison_died
        .checked_mul(reference_total)
        .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
    let reference_cross = reference_died
        .checked_mul(comparison_total)
        .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
    let comparison_cross =
        i64::try_from(comparison_cross).map_err(|_| CausalRandomizationError::ArithmeticOverflow)?;
    let reference_cross =
        i64::try_from(reference_cross).map_err(|_| CausalRandomizationError::ArithmeticOverflow)?;
    let numerator = comparison_cross
        .checked_sub(reference_cross)
        .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
    let denominator = comparison_total
        .checked_mul(reference_total)
        .ok_or(CausalRandomizationError::ArithmeticOverflow)?;
    ExactRandomizationStatistic::new(numerator, denominator)
}

fn at_least_as_extreme(
    candidate: ExactRandomizationStatistic,
    observed: ExactRandomizationStatistic,
) -> bool {
    u128::from(candidate.numerator) * u128::from(observed.denominator)
        >= u128::from(observed.numerator) * u128::from(candidate.denominator)
}

fn for_each_combination<F>(
    n: usize,
    k: usize,
    mut visit: F,
) -> Result<(), CausalRandomizationError>
where
    F: FnMut(&[usize]) -> Result<(), CausalRandomizationError>,
{
    if k == 0 || k > n {
        return Err(CausalRandomizationError::AssignmentCountInvariant);
    }
    let mut combination = (0..k).collect::<Vec<_>>();
    loop {
        visit(&combination)?;
        let mut pivot = None;
        for index in (0..k).rev() {
            if combination[index] != index + n - k {
                pivot = Some(index);
                break;
            }
        }
        let Some(index) = pivot else {
            break;
        };
        combination[index] += 1;
        for next in (index + 1)..k {
            combination[next] = combination[next - 1] + 1;
        }
    }
    Ok(())
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn update_authority_digest(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

fn design_class_tag(value: SelectionDesignClass) -> u8 {
    match value {
        SelectionDesignClass::DescriptiveAssociation => 0,
        SelectionDesignClass::RandomizedInterventional => 1,
        SelectionDesignClass::ControlledSimulationIntervention => 2,
        SelectionDesignClass::MatchedStratifiedObservational => 3,
        SelectionDesignClass::WithinFamilyOrLineageControlled => 4,
        SelectionDesignClass::CommonEnvironment => 5,
        SelectionDesignClass::ReciprocalContext => 6,
        SelectionDesignClass::DeclaredNeutralNullComparison => 7,
    }
}

fn target_tag(value: CausalSelectionTarget) -> u8 {
    match value {
        CausalSelectionTarget::BinaryViabilityRiskDifference => 0,
    }
}

#[derive(Debug)]
pub enum CausalRandomizationError {
    UnsupportedTestVersion(u32),
    UnsupportedDesignClass(SelectionDesignClass),
    UnsupportedTarget,
    MethodSemanticMismatch,
    MethodAuthorityMismatch,
    EffectFrameMismatch,
    ReferenceFrameMismatch,
    IdentificationMismatch,
    ReferenceSemanticMismatch,
    DenominatorInvariant,
    AssignmentCountInvariant,
    UnsupportedOutcome,
    ObservedEffectMismatch,
    ExactEnumerationSupportTooLarge {
        assignments: u64,
        max_assignments: u64,
    },
    AssignmentSupportMismatch { enumerated: u64, declared: u64 },
    NonCanonicalDistributionOrder,
    DistributionInvariant,
    ProbabilityInvariant,
    ArithmeticOverflow,
    ArithmeticInvariant,
    TestReplayMismatch,
}

impl fmt::Display for CausalRandomizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTestVersion(version) => {
                write!(f, "unsupported causal randomization test version {version}")
            }
            Self::UnsupportedDesignClass(class) => write!(
                f,
                "SEL-07D2 V1 requires randomized-interventional design, got {class:?}"
            ),
            Self::UnsupportedTarget => write!(
                f,
                "SEL-07D2 V1 requires binary viability-risk causal target"
            ),
            Self::MethodSemanticMismatch => {
                write!(f, "persisted causal randomization method semantics do not match V1")
            }
            Self::MethodAuthorityMismatch => {
                write!(f, "causal randomization method authority does not match V1")
            }
            Self::EffectFrameMismatch => write!(f, "causal effect is bound to another frame"),
            Self::ReferenceFrameMismatch => {
                write!(f, "randomization reference is bound to another frame")
            }
            Self::IdentificationMismatch => write!(
                f,
                "causal effect and randomization reference bind different identification authorities"
            ),
            Self::ReferenceSemanticMismatch => write!(
                f,
                "randomization reference semantics do not match D2 V1"
            ),
            Self::DenominatorInvariant => write!(f, "causal randomization denominator invariant failed"),
            Self::AssignmentCountInvariant => write!(f, "causal randomization assignment-count invariant failed"),
            Self::UnsupportedOutcome => write!(
                f,
                "SEL-07D2 V1 requires observed viability outcome for every estimand individual"
            ),
            Self::ObservedEffectMismatch => write!(
                f,
                "observed D1 assignment statistic does not equal current C2 causal point effect"
            ),
            Self::ExactEnumerationSupportTooLarge {
                assignments,
                max_assignments,
            } => write!(
                f,
                "exact D2 enumeration supports at most {max_assignments} assignments, got {assignments}; V1 refuses approximation"
            ),
            Self::AssignmentSupportMismatch { enumerated, declared } => write!(
                f,
                "enumerated {enumerated} assignments but D1 declares support size {declared}"
            ),
            Self::NonCanonicalDistributionOrder => write!(
                f,
                "persisted randomization statistic distribution is not in canonical order"
            ),
            Self::DistributionInvariant => write!(f, "randomization statistic distribution invariant failed"),
            Self::ProbabilityInvariant => write!(f, "exact randomization probability invariant failed"),
            Self::ArithmeticOverflow => write!(f, "exact causal randomization arithmetic overflowed"),
            Self::ArithmeticInvariant => write!(f, "exact causal randomization arithmetic invariant failed"),
            Self::TestReplayMismatch => write!(
                f,
                "persisted causal randomization result does not replay against current authorities"
            ),
        }
    }
}

impl Error for CausalRandomizationError {}
