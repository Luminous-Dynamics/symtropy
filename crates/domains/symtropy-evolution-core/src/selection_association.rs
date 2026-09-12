use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisAuthorityRef,
    AnalysisContentDigest, AnalysisMethodId, BinaryComparisonGroup,
    ExplicitSelectionAnalysisFrameDigest, InsufficientSupportPolicy, MaterializedOutcome,
    MaterializedPredictorValue, PredictorMaterializationStatus, PredictorRepresentation,
    SelectionDesignClass, SelectionEstimand, ValidatedSelectionAnalysisFrame,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const CONSEQUENCE_ASSOCIATION_ESTIMATE_VERSION: u32 = 1;
pub const BINARY_VIABILITY_EXACT_REFERENCE_MAX_ROWS: u64 = 64;

const ASSOCIATION_DOMAIN: &[u8] = b"symtropy:evolution:consequence-association-estimate:v1\0";
const METHOD_DOMAIN: &[u8] = b"symtropy:evolution:analysis-method-authority:v1\0";
const BINARY_VIABILITY_METHOD_ID: &str = "binary-viability-exact-reference-v1";
const BINARY_VIABILITY_METHOD_SPEC: &[u8] = b"binary predictor; viability-window outcome; comparison-minus-reference exact risk difference; exact risk-ratio status; fixed-margin hypergeometric reference distribution; two-sided Fisher-style probability ordering; no stochastic resampling; no multiplicity adjustment; max 64 estimand rows";
const BINARY_VIABILITY_MINIMUM_INFORMATION_ID: &str =
    "binary-viability-exact-reference-minimum-information-v1";
const BINARY_VIABILITY_MINIMUM_INFORMATION_SPEC: &[u8] = b"all estimand rows must have observed binary predictor and observed viability outcome; reference and comparison groups must each contain at least one row; estimand denominator must not exceed 64 rows";

pub fn binary_viability_exact_reference_method_v1() -> AnalysisAuthorityRef {
    authority_from_spec(BINARY_VIABILITY_METHOD_ID, BINARY_VIABILITY_METHOD_SPEC)
}

pub fn binary_viability_exact_reference_minimum_information_v1() -> AnalysisAuthorityRef {
    authority_from_spec(
        BINARY_VIABILITY_MINIMUM_INFORMATION_ID,
        BINARY_VIABILITY_MINIMUM_INFORMATION_SPEC,
    )
}

fn authority_from_spec(id: &str, spec: &[u8]) -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(METHOD_DOMAIN);
    put_text(&mut digest, id);
    put_u64(&mut digest, 1);
    put_u64(&mut digest, spec.len() as u64);
    digest.update(spec);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(id).expect("built-in analysis method ID must remain valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactUnsignedFraction {
    pub numerator: u64,
    pub denominator: u64,
}

impl ExactUnsignedFraction {
    fn new(numerator: u64, denominator: u64) -> Result<Self, AssociationExecutionError> {
        if denominator == 0 {
            return Err(AssociationExecutionError::ArithmeticInvariant);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn update_digest(&self, digest: &mut Sha256) {
        put_u64(digest, self.numerator);
        put_u64(digest, self.denominator);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactSignedFraction {
    pub negative: bool,
    pub numerator: u64,
    pub denominator: u64,
}

impl ExactSignedFraction {
    fn new(numerator: i64, denominator: u64) -> Result<Self, AssociationExecutionError> {
        if denominator == 0 {
            return Err(AssociationExecutionError::ArithmeticInvariant);
        }
        let negative = numerator < 0;
        let magnitude = numerator.unsigned_abs();
        let divisor = gcd(magnitude, denominator);
        Ok(Self {
            negative: negative && magnitude != 0,
            numerator: magnitude / divisor,
            denominator: denominator / divisor,
        })
    }

    fn update_digest(&self, digest: &mut Sha256) {
        digest.update([u8::from(self.negative)]);
        put_u64(digest, self.numerator);
        put_u64(digest, self.denominator);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExactRiskRatio {
    Finite(ExactUnsignedFraction),
    PositiveInfinity,
    BothRisksZero,
}

impl ExactRiskRatio {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Finite(value) => {
                digest.update([0]);
                value.update_digest(digest);
            }
            Self::PositiveInfinity => digest.update([1]),
            Self::BothRisksZero => digest.update([2]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryViabilityGroupSupport {
    pub total: u64,
    pub died_during_window: u64,
    pub survived_window: u64,
}

impl BinaryViabilityGroupSupport {
    fn update_digest(&self, digest: &mut Sha256) {
        put_u64(digest, self.total);
        put_u64(digest, self.died_during_window);
        put_u64(digest, self.survived_window);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypergeometricSupportPoint {
    pub comparison_deaths: u64,
    pub weight: u64,
}

impl HypergeometricSupportPoint {
    fn update_digest(&self, digest: &mut Sha256) {
        put_u64(digest, self.comparison_deaths);
        put_u64(digest, self.weight);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactHypergeometricReference {
    pub total_weight: u64,
    pub observed_weight: u64,
    pub support: Vec<HypergeometricSupportPoint>,
    pub two_sided_probability_ordered_p: ExactUnsignedFraction,
}

impl ExactHypergeometricReference {
    fn update_digest(&self, digest: &mut Sha256) {
        put_u64(digest, self.total_weight);
        put_u64(digest, self.observed_weight);
        put_u64(digest, self.support.len() as u64);
        for point in &self.support {
            point.update_digest(digest);
        }
        self.two_sided_probability_ordered_p.update_digest(digest);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryViabilityAssociation {
    pub reference: BinaryViabilityGroupSupport,
    pub comparison: BinaryViabilityGroupSupport,
    pub comparison_minus_reference_risk_difference: ExactSignedFraction,
    pub comparison_over_reference_risk_ratio: ExactRiskRatio,
    pub exact_reference: ExactHypergeometricReference,
}

impl BinaryViabilityAssociation {
    fn update_digest(&self, digest: &mut Sha256) {
        self.reference.update_digest(digest);
        self.comparison.update_digest(digest);
        self.comparison_minus_reference_risk_difference
            .update_digest(digest);
        self.comparison_over_reference_risk_ratio
            .update_digest(digest);
        self.exact_reference.update_digest(digest);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociationInsufficiency {
    MissingPredictor(crate::EvolutionIndividualId),
    MissingViabilityOutcome(crate::EvolutionIndividualId),
    EmptyReferenceGroup,
    EmptyComparisonGroup,
}

impl AssociationInsufficiency {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::MissingPredictor(id) => {
                digest.update([0]);
                put_text(digest, id.as_str());
            }
            Self::MissingViabilityOutcome(id) => {
                digest.update([1]);
                put_text(digest, id.as_str());
            }
            Self::EmptyReferenceGroup => digest.update([2]),
            Self::EmptyComparisonGroup => digest.update([3]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsequenceAssociationStatus {
    Estimated(BinaryViabilityAssociation),
    InsufficientSupport(AssociationInsufficiency),
}

impl ConsequenceAssociationStatus {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Estimated(association) => {
                digest.update([0]);
                association.update_digest(digest);
            }
            Self::InsufficientSupport(reason) => {
                digest.update([1]);
                reason.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsequenceAssociationEstimate {
    estimate_version: u32,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    pub source_design_class: SelectionDesignClass,
    pub estimand: SelectionEstimand,
    pub original_denominator: u64,
    pub estimand_denominator: u64,
    pub method: AnalysisAuthorityRef,
    pub minimum_information: AnalysisAuthorityRef,
    pub status: ConsequenceAssociationStatus,
}

impl ConsequenceAssociationEstimate {
    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ConsequenceAssociationEstimateDigest, AssociationExecutionError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(ASSOCIATION_DOMAIN);
        put_u32(&mut digest, self.estimate_version);
        digest.update(self.frame_digest.as_bytes());
        digest.update([design_class_tag(self.source_design_class)]);
        digest.update([estimand_tag(self.estimand)]);
        put_u64(&mut digest, self.original_denominator);
        put_u64(&mut digest, self.estimand_denominator);
        update_authority_digest(&mut digest, &self.method);
        update_authority_digest(&mut digest, &self.minimum_information);
        self.status.update_digest(&mut digest);
        Ok(ConsequenceAssociationEstimateDigest(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), AssociationExecutionError> {
        if self.estimate_version != CONSEQUENCE_ASSOCIATION_ESTIMATE_VERSION {
            return Err(AssociationExecutionError::UnsupportedEstimateVersion(
                self.estimate_version,
            ));
        }
        if self.estimand != SelectionEstimand::ViabilityWindowRiskContrast {
            return Err(AssociationExecutionError::UnsupportedEstimand(self.estimand));
        }
        if self.method != binary_viability_exact_reference_method_v1()
            || self.minimum_information
                != binary_viability_exact_reference_minimum_information_v1()
        {
            return Err(AssociationExecutionError::MethodAuthorityMismatch);
        }
        if self.estimand_denominator == 0
            || self.estimand_denominator > self.original_denominator
        {
            return Err(AssociationExecutionError::DenominatorInvariant);
        }
        if let ConsequenceAssociationStatus::Estimated(association) = &self.status {
            validate_estimated_association(association, self.estimand_denominator)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsequenceAssociationEstimateDigest([u8; 32]);

impl ConsequenceAssociationEstimateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ConsequenceAssociationEstimateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConsequenceAssociationEstimateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ConsequenceAssociationEstimateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated descriptive association should be consumed only as descriptive evidence"]
pub struct ValidatedConsequenceAssociation<'a> {
    estimate: &'a ConsequenceAssociationEstimate,
    estimate_digest: ConsequenceAssociationEstimateDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
}

impl<'a> ValidatedConsequenceAssociation<'a> {
    pub fn validate_current(
        estimate: &'a ConsequenceAssociationEstimate,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
    ) -> Result<Self, AssociationExecutionError> {
        estimate.validate_local()?;
        let recomputed = execute_binary_viability_association(frame)?;
        if recomputed != *estimate {
            return Err(AssociationExecutionError::EstimateReplayMismatch);
        }
        Ok(Self {
            estimate,
            estimate_digest: estimate.canonical_digest()?,
            frame_digest: frame.frame_digest(),
        })
    }

    pub fn estimate(&self) -> &'a ConsequenceAssociationEstimate {
        self.estimate
    }

    pub fn estimate_digest(&self) -> ConsequenceAssociationEstimateDigest {
        self.estimate_digest
    }

    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }
}

pub fn execute_binary_viability_association(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
) -> Result<ConsequenceAssociationEstimate, AssociationExecutionError> {
    validate_plan(frame)?;

    if frame.frame().estimand_denominator() > BINARY_VIABILITY_EXACT_REFERENCE_MAX_ROWS {
        return Err(AssociationExecutionError::ExactReferencePopulationTooLarge {
            rows: frame.frame().estimand_denominator(),
            max_rows: BINARY_VIABILITY_EXACT_REFERENCE_MAX_ROWS,
        });
    }

    let mut reference_total = 0u64;
    let mut reference_died = 0u64;
    let mut comparison_total = 0u64;
    let mut comparison_died = 0u64;

    for row in &frame.frame().rows {
        let group = match &row.predictor.status {
            PredictorMaterializationStatus::Observed(MaterializedPredictorValue::Binary(group)) => {
                *group
            }
            PredictorMaterializationStatus::Missing { .. } => {
                return handle_insufficient(
                    frame,
                    AssociationInsufficiency::MissingPredictor(row.individual_id.clone()),
                );
            }
            _ => return Err(AssociationExecutionError::UnsupportedPredictorValue),
        };
        let died = match &row.outcome {
            MaterializedOutcome::ViabilityObserved(crate::ViabilityConsequence::SurvivedWindow) => {
                false
            }
            MaterializedOutcome::ViabilityObserved(crate::ViabilityConsequence::DiedDuringWindow) => {
                true
            }
            MaterializedOutcome::Unavailable => {
                return handle_insufficient(
                    frame,
                    AssociationInsufficiency::MissingViabilityOutcome(row.individual_id.clone()),
                );
            }
            _ => return Err(AssociationExecutionError::UnsupportedOutcomeValue),
        };

        match group {
            BinaryComparisonGroup::Reference => {
                reference_total += 1;
                reference_died += if died { 1 } else { 0 };
            }
            BinaryComparisonGroup::Comparison => {
                comparison_total += 1;
                comparison_died += if died { 1 } else { 0 };
            }
        }
    }

    if reference_total == 0 {
        return handle_insufficient(frame, AssociationInsufficiency::EmptyReferenceGroup);
    }
    if comparison_total == 0 {
        return handle_insufficient(frame, AssociationInsufficiency::EmptyComparisonGroup);
    }

    let reference = BinaryViabilityGroupSupport {
        total: reference_total,
        died_during_window: reference_died,
        survived_window: reference_total - reference_died,
    };
    let comparison = BinaryViabilityGroupSupport {
        total: comparison_total,
        died_during_window: comparison_died,
        survived_window: comparison_total - comparison_died,
    };
    let risk_difference = exact_risk_difference(reference, comparison)?;
    let risk_ratio = exact_risk_ratio(reference, comparison)?;
    let exact_reference = exact_hypergeometric_reference(reference, comparison)?;

    let estimate = base_estimate(
        frame,
        ConsequenceAssociationStatus::Estimated(BinaryViabilityAssociation {
            reference,
            comparison,
            comparison_minus_reference_risk_difference: risk_difference,
            comparison_over_reference_risk_ratio: risk_ratio,
            exact_reference,
        }),
    );
    estimate.validate_local()?;
    Ok(estimate)
}

fn validate_plan(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
) -> Result<(), AssociationExecutionError> {
    if frame.estimand() != SelectionEstimand::ViabilityWindowRiskContrast {
        return Err(AssociationExecutionError::UnsupportedEstimand(frame.estimand()));
    }
    if frame.frame().predictor_representation != PredictorRepresentation::BinaryComparisonGroup {
        return Err(AssociationExecutionError::UnsupportedPredictorRepresentation);
    }
    let uncertainty = frame.uncertainty();
    if uncertainty.method != binary_viability_exact_reference_method_v1()
        || uncertainty.minimum_information
            != binary_viability_exact_reference_minimum_information_v1()
    {
        return Err(AssociationExecutionError::MethodAuthorityMismatch);
    }
    if uncertainty.replicate_count.is_some()
        || uncertainty.seed_lineage_digest.is_some()
        || uncertainty.multiplicity.is_some()
    {
        return Err(AssociationExecutionError::UnsupportedUncertaintyConfiguration);
    }
    Ok(())
}

fn handle_insufficient(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    reason: AssociationInsufficiency,
) -> Result<ConsequenceAssociationEstimate, AssociationExecutionError> {
    match frame.uncertainty().insufficient_support {
        InsufficientSupportPolicy::FailClosed => {
            Err(AssociationExecutionError::InsufficientSupport(reason))
        }
        InsufficientSupportPolicy::ReportInsufficientSupport
        | InsufficientSupportPolicy::DescriptiveOnlyFallback => {
            let estimate = base_estimate(
                frame,
                ConsequenceAssociationStatus::InsufficientSupport(reason),
            );
            estimate.validate_local()?;
            Ok(estimate)
        }
    }
}

fn base_estimate(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    status: ConsequenceAssociationStatus,
) -> ConsequenceAssociationEstimate {
    ConsequenceAssociationEstimate {
        estimate_version: CONSEQUENCE_ASSOCIATION_ESTIMATE_VERSION,
        frame_digest: frame.frame_digest(),
        source_design_class: frame.design_class(),
        estimand: frame.estimand(),
        original_denominator: frame.frame().original_denominator(),
        estimand_denominator: frame.frame().estimand_denominator(),
        method: binary_viability_exact_reference_method_v1(),
        minimum_information: binary_viability_exact_reference_minimum_information_v1(),
        status,
    }
}

fn exact_risk_difference(
    reference: BinaryViabilityGroupSupport,
    comparison: BinaryViabilityGroupSupport,
) -> Result<ExactSignedFraction, AssociationExecutionError> {
    let comparison_cross = comparison
        .died_during_window
        .checked_mul(reference.total)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    let reference_cross = reference
        .died_during_window
        .checked_mul(comparison.total)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    let numerator = i64::try_from(comparison_cross)
        .map_err(|_| AssociationExecutionError::ArithmeticOverflow)?
        - i64::try_from(reference_cross)
            .map_err(|_| AssociationExecutionError::ArithmeticOverflow)?;
    let denominator = comparison
        .total
        .checked_mul(reference.total)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    ExactSignedFraction::new(numerator, denominator)
}

fn exact_risk_ratio(
    reference: BinaryViabilityGroupSupport,
    comparison: BinaryViabilityGroupSupport,
) -> Result<ExactRiskRatio, AssociationExecutionError> {
    if reference.died_during_window == 0 {
        return Ok(if comparison.died_during_window == 0 {
            ExactRiskRatio::BothRisksZero
        } else {
            ExactRiskRatio::PositiveInfinity
        });
    }
    let numerator = comparison
        .died_during_window
        .checked_mul(reference.total)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    let denominator = reference
        .died_during_window
        .checked_mul(comparison.total)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    Ok(ExactRiskRatio::Finite(ExactUnsignedFraction::new(
        numerator,
        denominator,
    )?))
}

fn exact_hypergeometric_reference(
    reference: BinaryViabilityGroupSupport,
    comparison: BinaryViabilityGroupSupport,
) -> Result<ExactHypergeometricReference, AssociationExecutionError> {
    let total = reference
        .total
        .checked_add(comparison.total)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    let deaths = reference
        .died_during_window
        .checked_add(comparison.died_during_window)
        .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
    let comparison_size = comparison.total;
    let survivors = total - deaths;
    let min_comparison_deaths = comparison_size.saturating_sub(survivors);
    let max_comparison_deaths = comparison_size.min(deaths);
    let total_weight = choose_u64(total, comparison_size)?;

    let mut support = Vec::new();
    let mut observed_weight = None;
    for comparison_deaths in min_comparison_deaths..=max_comparison_deaths {
        let death_choices = choose_u64(deaths, comparison_deaths)?;
        let survivor_choices = choose_u64(survivors, comparison_size - comparison_deaths)?;
        let weight_u128 = u128::from(death_choices) * u128::from(survivor_choices);
        let weight = u64::try_from(weight_u128)
            .map_err(|_| AssociationExecutionError::ArithmeticOverflow)?;
        if comparison_deaths == comparison.died_during_window {
            observed_weight = Some(weight);
        }
        support.push(HypergeometricSupportPoint {
            comparison_deaths,
            weight,
        });
    }

    let observed_weight = observed_weight.ok_or(AssociationExecutionError::ArithmeticInvariant)?;
    let p_numerator = support
        .iter()
        .filter(|point| point.weight <= observed_weight)
        .try_fold(0u64, |sum, point| {
            sum.checked_add(point.weight)
                .ok_or(AssociationExecutionError::ArithmeticOverflow)
        })?;
    let support_weight = support.iter().try_fold(0u64, |sum, point| {
        sum.checked_add(point.weight)
            .ok_or(AssociationExecutionError::ArithmeticOverflow)
    })?;
    if support_weight != total_weight {
        return Err(AssociationExecutionError::ArithmeticInvariant);
    }

    Ok(ExactHypergeometricReference {
        total_weight,
        observed_weight,
        support,
        two_sided_probability_ordered_p: ExactUnsignedFraction::new(p_numerator, total_weight)?,
    })
}

fn choose_u64(n: u64, k: u64) -> Result<u64, AssociationExecutionError> {
    if k > n {
        return Ok(0);
    }
    let k = k.min(n - k);
    let mut value = 1u128;
    for i in 0..k {
        value = value
            .checked_mul(u128::from(n - i))
            .ok_or(AssociationExecutionError::ArithmeticOverflow)?;
        value /= u128::from(i + 1);
    }
    u64::try_from(value).map_err(|_| AssociationExecutionError::ArithmeticOverflow)
}

fn validate_estimated_association(
    association: &BinaryViabilityAssociation,
    estimand_denominator: u64,
) -> Result<(), AssociationExecutionError> {
    if association.reference.total == 0 || association.comparison.total == 0 {
        return Err(AssociationExecutionError::DenominatorInvariant);
    }
    if association.reference.died_during_window + association.reference.survived_window
        != association.reference.total
        || association.comparison.died_during_window + association.comparison.survived_window
            != association.comparison.total
        || association.reference.total + association.comparison.total != estimand_denominator
    {
        return Err(AssociationExecutionError::DenominatorInvariant);
    }
    if association.exact_reference.support.is_empty()
        || association.exact_reference.total_weight == 0
        || association.exact_reference.observed_weight == 0
    {
        return Err(AssociationExecutionError::ArithmeticInvariant);
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

fn estimand_tag(value: SelectionEstimand) -> u8 {
    match value {
        SelectionEstimand::ViabilityWindowRiskContrast => 0,
        SelectionEstimand::ReproductiveEventOpportunityConditionedContrast => 1,
        SelectionEstimand::DescendantProductionContrast => 2,
        SelectionEstimand::DescendantRecruitmentContrast => 3,
    }
}

#[derive(Debug)]
pub enum AssociationExecutionError {
    UnsupportedEstimateVersion(u32),
    UnsupportedEstimand(SelectionEstimand),
    UnsupportedPredictorRepresentation,
    UnsupportedPredictorValue,
    UnsupportedOutcomeValue,
    MethodAuthorityMismatch,
    UnsupportedUncertaintyConfiguration,
    ExactReferencePopulationTooLarge { rows: u64, max_rows: u64 },
    InsufficientSupport(AssociationInsufficiency),
    ArithmeticOverflow,
    ArithmeticInvariant,
    DenominatorInvariant,
    EstimateReplayMismatch,
}

impl fmt::Display for AssociationExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEstimateVersion(version) => {
                write!(f, "unsupported consequence-association estimate version {version}")
            }
            Self::UnsupportedEstimand(estimand) => {
                write!(f, "SEL-07B2 V1 does not support estimand {estimand:?}")
            }
            Self::UnsupportedPredictorRepresentation => write!(
                f,
                "SEL-07B2 V1 requires binary reference/comparison predictor representation"
            ),
            Self::UnsupportedPredictorValue => {
                write!(f, "analysis frame contains a predictor value unsupported by SEL-07B2 V1")
            }
            Self::UnsupportedOutcomeValue => {
                write!(f, "analysis frame contains an outcome value unsupported by SEL-07B2 V1")
            }
            Self::MethodAuthorityMismatch => write!(
                f,
                "frozen analysis method/minimum-information authority does not match SEL-07B2 V1"
            ),
            Self::UnsupportedUncertaintyConfiguration => write!(
                f,
                "SEL-07B2 V1 exact reference method does not accept replicate, seed, or multiplicity configuration"
            ),
            Self::ExactReferencePopulationTooLarge { rows, max_rows } => write!(
                f,
                "exact hypergeometric reference supports at most {max_rows} estimand rows, got {rows}"
            ),
            Self::InsufficientSupport(reason) => {
                write!(f, "frozen minimum-information rule failed: {reason:?}")
            }
            Self::ArithmeticOverflow => write!(f, "exact association arithmetic overflowed"),
            Self::ArithmeticInvariant => write!(f, "exact association arithmetic invariant failed"),
            Self::DenominatorInvariant => write!(f, "association denominator invariant failed"),
            Self::EstimateReplayMismatch => write!(f, "consequence-association replay mismatch"),
        }
    }
}

impl Error for AssociationExecutionError {}
