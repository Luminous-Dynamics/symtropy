use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisAuthorityRef,
    AnalysisContentDigest, AnalysisMethodId, BinaryComparisonGroup, BinaryViabilityGroupSupport,
    CausalIdentificationTier, CausalSelectionEffectId, CausalSelectionIdentificationDigest,
    CausalSelectionTarget, ConsequenceAssociationEstimateDigest, ConsequenceAssociationStatus,
    ExactSignedFraction, ExplicitSelectionAnalysisFrameDigest, MaterializedOutcome,
    MaterializedPredictorValue, PredictorMaterializationStatus, SelectionDesignClass,
    ValidatedCausalSelectionIdentification, ValidatedConsequenceAssociation,
    ValidatedSelectionAnalysisFrame,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const CAUSAL_VIABILITY_RISK_EFFECT_VERSION: u32 = 1;

const CAUSAL_EFFECT_DOMAIN: &[u8] =
    b"symtropy:evolution:causal-viability-risk-effect:v1\0";
const METHOD_DOMAIN: &[u8] = b"symtropy:evolution:causal-effect-method-authority:v1\0";
const METHOD_ID: &str = "intervention-identified-binary-viability-risk-difference-v1";
const METHOD_SPEC: &[u8] = b"requires current InterventionIdentified SEL-07C1 authority; exact binary Reference/Comparison frame; exact viability-window outcomes; point effect do(Comparison)-do(Reference) represented as reduced risk-difference fraction; exact agreement with current SEL-07B2 support table; no causal p-value or causal uncertainty interval";

pub fn causal_viability_risk_effect_method_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(METHOD_DOMAIN);
    put_text(&mut digest, METHOD_ID);
    put_u64(&mut digest, 1);
    put_u64(&mut digest, METHOD_SPEC.len() as u64);
    digest.update(METHOD_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(METHOD_ID).expect("built-in causal effect method ID must remain valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalViabilityRiskEffect {
    effect_version: u32,
    pub effect_id: CausalSelectionEffectId,
    identification_digest: CausalSelectionIdentificationDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    association_digest: ConsequenceAssociationEstimateDigest,
    pub source_design_class: SelectionDesignClass,
    pub target: CausalSelectionTarget,
    original_denominator: u64,
    estimand_denominator: u64,
    pub reference_support: BinaryViabilityGroupSupport,
    pub comparison_support: BinaryViabilityGroupSupport,
    pub comparison_minus_reference_causal_risk_difference: ExactSignedFraction,
    pub method: AnalysisAuthorityRef,
}

impl CausalViabilityRiskEffect {
    pub fn identification_digest(&self) -> CausalSelectionIdentificationDigest {
        self.identification_digest
    }

    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn association_digest(&self) -> ConsequenceAssociationEstimateDigest {
        self.association_digest
    }

    pub fn original_denominator(&self) -> u64 {
        self.original_denominator
    }

    pub fn estimand_denominator(&self) -> u64 {
        self.estimand_denominator
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CausalViabilityRiskEffectDigest, CausalEffectError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(CAUSAL_EFFECT_DOMAIN);
        put_u32(&mut digest, self.effect_version);
        put_text(&mut digest, self.effect_id.as_str());
        digest.update(self.identification_digest.as_bytes());
        digest.update(self.frame_digest.as_bytes());
        digest.update(self.association_digest.as_bytes());
        digest.update([design_class_tag(self.source_design_class)]);
        digest.update([target_tag(self.target)]);
        put_u64(&mut digest, self.original_denominator);
        put_u64(&mut digest, self.estimand_denominator);
        update_support_digest(&mut digest, self.reference_support);
        update_support_digest(&mut digest, self.comparison_support);
        update_signed_fraction_digest(
            &mut digest,
            self.comparison_minus_reference_causal_risk_difference,
        );
        update_authority_digest(&mut digest, &self.method);
        Ok(CausalViabilityRiskEffectDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), CausalEffectError> {
        if self.effect_version != CAUSAL_VIABILITY_RISK_EFFECT_VERSION {
            return Err(CausalEffectError::UnsupportedEffectVersion(
                self.effect_version,
            ));
        }
        if self.target != CausalSelectionTarget::BinaryViabilityRiskDifference {
            return Err(CausalEffectError::UnsupportedTarget);
        }
        if !matches!(
            self.source_design_class,
            SelectionDesignClass::RandomizedInterventional
                | SelectionDesignClass::ControlledSimulationIntervention
        ) {
            return Err(CausalEffectError::UnsupportedDesignClass(
                self.source_design_class,
            ));
        }
        if self.method != causal_viability_risk_effect_method_v1() {
            return Err(CausalEffectError::MethodAuthorityMismatch);
        }
        if self.estimand_denominator == 0
            || self.estimand_denominator > self.original_denominator
        {
            return Err(CausalEffectError::DenominatorInvariant);
        }
        validate_support(self.reference_support)?;
        validate_support(self.comparison_support)?;
        if self.reference_support.total == 0 || self.comparison_support.total == 0 {
            return Err(CausalEffectError::EmptyInterventionGroup);
        }
        let combined = self
            .reference_support
            .total
            .checked_add(self.comparison_support.total)
            .ok_or(CausalEffectError::ArithmeticOverflow)?;
        if combined != self.estimand_denominator {
            return Err(CausalEffectError::DenominatorInvariant);
        }
        let expected = exact_risk_difference(self.reference_support, self.comparison_support)?;
        if expected != self.comparison_minus_reference_causal_risk_difference {
            return Err(CausalEffectError::EffectArithmeticInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausalViabilityRiskEffectDigest([u8; 32]);

impl CausalViabilityRiskEffectDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CausalViabilityRiskEffectDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CausalViabilityRiskEffectDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for CausalViabilityRiskEffectDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated causal effect should remain bound to current frame, association, and identification authorities"]
pub struct ValidatedCausalSelectionEffect<'a> {
    effect: &'a CausalViabilityRiskEffect,
    effect_digest: CausalViabilityRiskEffectDigest,
    identification_digest: CausalSelectionIdentificationDigest,
    frame_digest: ExplicitSelectionAnalysisFrameDigest,
    association_digest: ConsequenceAssociationEstimateDigest,
}

impl<'a> ValidatedCausalSelectionEffect<'a> {
    pub fn validate_current(
        effect: &'a CausalViabilityRiskEffect,
        frame: &ValidatedSelectionAnalysisFrame<'_>,
        association: &ValidatedConsequenceAssociation<'_>,
        identification: &ValidatedCausalSelectionIdentification<'_>,
    ) -> Result<Self, CausalEffectError> {
        effect.validate_local()?;
        let recomputed = execute_causal_viability_risk_effect(
            effect.effect_id.clone(),
            frame,
            association,
            identification,
        )?;
        if recomputed != *effect {
            return Err(CausalEffectError::EffectReplayMismatch);
        }
        Ok(Self {
            effect,
            effect_digest: effect.canonical_digest()?,
            identification_digest: identification.identification_digest(),
            frame_digest: frame.frame_digest(),
            association_digest: association.estimate_digest(),
        })
    }

    pub fn effect(&self) -> &'a CausalViabilityRiskEffect {
        self.effect
    }

    pub fn effect_digest(&self) -> CausalViabilityRiskEffectDigest {
        self.effect_digest
    }

    pub fn identification_digest(&self) -> CausalSelectionIdentificationDigest {
        self.identification_digest
    }

    pub fn frame_digest(&self) -> ExplicitSelectionAnalysisFrameDigest {
        self.frame_digest
    }

    pub fn association_digest(&self) -> ConsequenceAssociationEstimateDigest {
        self.association_digest
    }
}

pub fn execute_causal_viability_risk_effect(
    effect_id: CausalSelectionEffectId,
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    association: &ValidatedConsequenceAssociation<'_>,
    identification: &ValidatedCausalSelectionIdentification<'_>,
) -> Result<CausalViabilityRiskEffect, CausalEffectError> {
    validate_capability_chain(frame, association, identification)?;
    let (reference_support, comparison_support) = support_from_frame(frame)?;

    let descriptive = match &association.estimate().status {
        ConsequenceAssociationStatus::Estimated(value) => value,
        ConsequenceAssociationStatus::InsufficientSupport(_) => {
            return Err(CausalEffectError::DescriptiveAssociationInsufficient)
        }
    };
    if descriptive.reference != reference_support || descriptive.comparison != comparison_support {
        return Err(CausalEffectError::DescriptiveSupportMismatch);
    }

    let effect = CausalViabilityRiskEffect {
        effect_version: CAUSAL_VIABILITY_RISK_EFFECT_VERSION,
        effect_id,
        identification_digest: identification.identification_digest(),
        frame_digest: frame.frame_digest(),
        association_digest: association.estimate_digest(),
        source_design_class: frame.design_class(),
        target: identification.target(),
        original_denominator: frame.frame().original_denominator(),
        estimand_denominator: frame.frame().estimand_denominator(),
        reference_support,
        comparison_support,
        comparison_minus_reference_causal_risk_difference: exact_risk_difference(
            reference_support,
            comparison_support,
        )?,
        method: causal_viability_risk_effect_method_v1(),
    };
    effect.validate_local()?;
    Ok(effect)
}

fn validate_capability_chain(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    association: &ValidatedConsequenceAssociation<'_>,
    identification: &ValidatedCausalSelectionIdentification<'_>,
) -> Result<(), CausalEffectError> {
    if association.frame_digest() != frame.frame_digest() {
        return Err(CausalEffectError::AssociationFrameMismatch);
    }
    if identification.frame_digest() != frame.frame_digest() {
        return Err(CausalEffectError::IdentificationFrameMismatch);
    }
    if identification.design_digest() != frame.design_digest() {
        return Err(CausalEffectError::IdentificationDesignMismatch);
    }
    if identification.tier() != CausalIdentificationTier::InterventionIdentified {
        return Err(CausalEffectError::UnsupportedIdentificationTier);
    }
    if identification.target() != CausalSelectionTarget::BinaryViabilityRiskDifference {
        return Err(CausalEffectError::UnsupportedTarget);
    }
    if association.estimate().source_design_class != frame.design_class()
        || identification.identification().source_design_class != frame.design_class()
    {
        return Err(CausalEffectError::SourceDesignClassMismatch);
    }
    if association.estimate().original_denominator != frame.frame().original_denominator()
        || association.estimate().estimand_denominator != frame.frame().estimand_denominator()
        || identification.identification().original_denominator()
            != frame.frame().original_denominator()
        || identification.identification().estimand_denominator()
            != frame.frame().estimand_denominator()
    {
        return Err(CausalEffectError::DenominatorInvariant);
    }
    Ok(())
}

fn support_from_frame(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
) -> Result<(BinaryViabilityGroupSupport, BinaryViabilityGroupSupport), CausalEffectError> {
    let mut reference_total = 0u64;
    let mut reference_died = 0u64;
    let mut comparison_total = 0u64;
    let mut comparison_died = 0u64;

    for row in &frame.frame().rows {
        let group = match &row.predictor.status {
            PredictorMaterializationStatus::Observed(MaterializedPredictorValue::Binary(group)) => {
                *group
            }
            _ => return Err(CausalEffectError::UnsupportedFramePredictor),
        };
        let died = match row.outcome {
            MaterializedOutcome::ViabilityObserved(crate::ViabilityConsequence::SurvivedWindow) => {
                false
            }
            MaterializedOutcome::ViabilityObserved(crate::ViabilityConsequence::DiedDuringWindow) => {
                true
            }
            _ => return Err(CausalEffectError::UnsupportedFrameOutcome),
        };
        match group {
            BinaryComparisonGroup::Reference => {
                reference_total += 1;
                reference_died += u64::from(died);
            }
            BinaryComparisonGroup::Comparison => {
                comparison_total += 1;
                comparison_died += u64::from(died);
            }
        }
    }

    let reference = BinaryViabilityGroupSupport {
        total: reference_total,
        died_during_window: reference_died,
        survived_window: reference_total
            .checked_sub(reference_died)
            .ok_or(CausalEffectError::ArithmeticInvariant)?,
    };
    let comparison = BinaryViabilityGroupSupport {
        total: comparison_total,
        died_during_window: comparison_died,
        survived_window: comparison_total
            .checked_sub(comparison_died)
            .ok_or(CausalEffectError::ArithmeticInvariant)?,
    };
    Ok((reference, comparison))
}

fn validate_support(support: BinaryViabilityGroupSupport) -> Result<(), CausalEffectError> {
    let total = support
        .died_during_window
        .checked_add(support.survived_window)
        .ok_or(CausalEffectError::ArithmeticOverflow)?;
    if total != support.total {
        return Err(CausalEffectError::ArithmeticInvariant);
    }
    Ok(())
}

fn exact_risk_difference(
    reference: BinaryViabilityGroupSupport,
    comparison: BinaryViabilityGroupSupport,
) -> Result<ExactSignedFraction, CausalEffectError> {
    if reference.total == 0 || comparison.total == 0 {
        return Err(CausalEffectError::EmptyInterventionGroup);
    }
    let comparison_cross = comparison
        .died_during_window
        .checked_mul(reference.total)
        .ok_or(CausalEffectError::ArithmeticOverflow)?;
    let reference_cross = reference
        .died_during_window
        .checked_mul(comparison.total)
        .ok_or(CausalEffectError::ArithmeticOverflow)?;
    let signed = i128::from(comparison_cross) - i128::from(reference_cross);
    let negative = signed < 0;
    let magnitude = signed.unsigned_abs();
    let magnitude = u64::try_from(magnitude).map_err(|_| CausalEffectError::ArithmeticOverflow)?;
    let denominator = comparison
        .total
        .checked_mul(reference.total)
        .ok_or(CausalEffectError::ArithmeticOverflow)?;
    let divisor = gcd(magnitude, denominator);
    Ok(ExactSignedFraction {
        negative: negative && magnitude != 0,
        numerator: magnitude / divisor,
        denominator: denominator / divisor,
    })
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn update_support_digest(digest: &mut Sha256, support: BinaryViabilityGroupSupport) {
    put_u64(digest, support.total);
    put_u64(digest, support.died_during_window);
    put_u64(digest, support.survived_window);
}

fn update_signed_fraction_digest(digest: &mut Sha256, value: ExactSignedFraction) {
    digest.update([u8::from(value.negative)]);
    put_u64(digest, value.numerator);
    put_u64(digest, value.denominator);
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
pub enum CausalEffectError {
    UnsupportedEffectVersion(u32),
    UnsupportedDesignClass(SelectionDesignClass),
    UnsupportedIdentificationTier,
    UnsupportedTarget,
    MethodAuthorityMismatch,
    AssociationFrameMismatch,
    IdentificationFrameMismatch,
    IdentificationDesignMismatch,
    SourceDesignClassMismatch,
    DescriptiveAssociationInsufficient,
    DescriptiveSupportMismatch,
    UnsupportedFramePredictor,
    UnsupportedFrameOutcome,
    EmptyInterventionGroup,
    DenominatorInvariant,
    ArithmeticOverflow,
    ArithmeticInvariant,
    EffectArithmeticInvariant,
    EffectReplayMismatch,
}

impl fmt::Display for CausalEffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEffectVersion(version) => {
                write!(f, "unsupported causal viability-risk effect version {version}")
            }
            Self::UnsupportedDesignClass(class) => write!(
                f,
                "causal viability-risk effect does not support design class {class:?}"
            ),
            Self::UnsupportedIdentificationTier => {
                write!(f, "causal effect requires intervention-identified authority")
            }
            Self::UnsupportedTarget => write!(f, "unsupported causal-effect target"),
            Self::MethodAuthorityMismatch => {
                write!(f, "causal-effect method authority mismatch")
            }
            Self::AssociationFrameMismatch => {
                write!(f, "descriptive association is bound to another analysis frame")
            }
            Self::IdentificationFrameMismatch => {
                write!(f, "causal identification is bound to another analysis frame")
            }
            Self::IdentificationDesignMismatch => {
                write!(f, "causal identification is bound to another design")
            }
            Self::SourceDesignClassMismatch => {
                write!(f, "causal effect source design classes do not agree")
            }
            Self::DescriptiveAssociationInsufficient => {
                write!(f, "causal effect requires an estimated descriptive association")
            }
            Self::DescriptiveSupportMismatch => write!(
                f,
                "descriptive support table does not equal independently materialized frame support"
            ),
            Self::UnsupportedFramePredictor => {
                write!(f, "causal V1 frame contains non-binary or missing predictor")
            }
            Self::UnsupportedFrameOutcome => {
                write!(f, "causal V1 frame contains non-viability or missing outcome")
            }
            Self::EmptyInterventionGroup => {
                write!(f, "causal effect requires non-empty Reference and Comparison groups")
            }
            Self::DenominatorInvariant => write!(f, "causal-effect denominator invariant failed"),
            Self::ArithmeticOverflow => write!(f, "causal-effect exact arithmetic overflowed"),
            Self::ArithmeticInvariant => write!(f, "causal-effect arithmetic invariant failed"),
            Self::EffectArithmeticInvariant => {
                write!(f, "stored causal point effect does not match stored support table")
            }
            Self::EffectReplayMismatch => write!(f, "causal-effect replay mismatch"),
        }
    }
}

impl Error for CausalEffectError {}
