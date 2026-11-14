//! Risk-proportionate model credibility requirements.
//!
//! This is an explicit Luminous project policy inspired by risk-based VVUQ. It is not an
//! ASME/NIST risk formula, regulatory classification, or certification decision. The qualitative
//! influence/consequence matrix produces required evidence controls only; it never asserts that
//! any control has been completed.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCreation {
    Mechanistic,
    Empirical,
    Hybrid,
}

impl ModelCreation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mechanistic => "mechanistic_physics_based",
            Self::Empirical => "empirical_data_driven",
            Self::Hybrid => "hybrid_mechanistic_empirical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicTransparency {
    Inspectable,
    PartiallyInspectable,
    Opaque,
}

impl LogicTransparency {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inspectable => "inspectable",
            Self::PartiallyInspectable => "partially_inspectable",
            Self::Opaque => "opaque",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactAccess {
    IndependentReplayAvailable,
    Restricted,
    Unavailable,
}

impl ArtifactAccess {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IndependentReplayAvailable => "independent_replay_available",
            Self::Restricted => "restricted",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelInfluence {
    Advisory,
    Material,
    Controlling,
}

impl ModelInfluence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Material => "material",
            Self::Controlling => "controlling",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionConsequence {
    Limited,
    Significant,
    Severe,
}

impl DecisionConsequence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Limited => "limited",
            Self::Significant => "significant",
            Self::Severe => "severe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CredibilityTier {
    Screening,
    Low,
    Moderate,
    High,
    Critical,
}

impl CredibilityTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Screening => "screening",
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCredibilityContext {
    pub model_id: String,
    pub intended_use: String,
    pub model_creation: ModelCreation,
    pub logic_transparency: LogicTransparency,
    pub artifact_access: ArtifactAccess,
    pub model_influence: ModelInfluence,
    pub decision_consequence: DecisionConsequence,
    pub influence_rationale: String,
    pub consequence_rationale: String,
    pub classification_reference: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CredibilityRequirement {
    pub control_id: &'static str,
    pub rationale: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelCredibilityAssessment {
    pub model_id: String,
    pub intended_use: String,
    pub model_creation: ModelCreation,
    pub logic_transparency: LogicTransparency,
    pub artifact_access: ArtifactAccess,
    pub model_influence: ModelInfluence,
    pub decision_consequence: DecisionConsequence,
    pub tier: CredibilityTier,
    pub classification_reference: String,
    pub influence_rationale: String,
    pub consequence_rationale: String,
    pub requirements: Vec<CredibilityRequirement>,
    pub restrictions: Vec<&'static str>,
    pub status: &'static str,
}

pub fn classify_credibility_tier(
    influence: ModelInfluence,
    consequence: DecisionConsequence,
) -> CredibilityTier {
    use CredibilityTier::{Critical, High, Low, Moderate, Screening};
    use DecisionConsequence::{Limited, Severe, Significant};
    use ModelInfluence::{Advisory, Controlling, Material};

    match (influence, consequence) {
        (Advisory, Limited) => Screening,
        (Advisory, Significant) | (Material, Limited) => Low,
        (Advisory, Severe) | (Material, Significant) | (Controlling, Limited) => Moderate,
        (Material, Severe) | (Controlling, Significant) => High,
        (Controlling, Severe) => Critical,
    }
}

pub fn assess_model_credibility(
    context: &ModelCredibilityContext,
) -> Result<ModelCredibilityAssessment, String> {
    for (name, value) in [
        ("model_id", context.model_id.as_str()),
        ("intended_use", context.intended_use.as_str()),
        ("influence_rationale", context.influence_rationale.as_str()),
        (
            "consequence_rationale",
            context.consequence_rationale.as_str(),
        ),
        (
            "classification_reference",
            context.classification_reference.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name} must not be empty"));
        }
    }

    let tier = classify_credibility_tier(context.model_influence, context.decision_consequence);
    let mut required = BTreeSet::new();

    for control in [
        (
            "intended_use_and_applicability_domain",
            "The use and domain must be explicit.",
        ),
        (
            "model_assumptions_and_input_lineage",
            "Assumptions, inputs and their provenance must be reviewable.",
        ),
        (
            "exact_model_and_run_identity",
            "The model version, run configuration and outputs need immutable identities.",
        ),
        (
            "failure_modes_and_abstention_policy",
            "Known failure conditions and no-answer behaviour must be declared.",
        ),
        (
            "input_sensitivity_screen",
            "Check whether plausible input changes reverse the decision.",
        ),
        (
            "no_release_authorization_from_assessment",
            "This requirements assessment never authorizes flight, manufacturing or operational release.",
        ),
        (
            "screening_only_label",
            "Keep outputs explicitly labelled as screening or requirements-only; never as released designs.",
        ),
        (
            "independent_control_review",
            "A reviewer independent of model development must inspect the evidence record.",
        ),
    ] {
        required.insert(control);
    }

    match context.model_creation {
        ModelCreation::Mechanistic => {
            for control in [
                (
                    "code_verification",
                    "Verify implementation with analytical/known-answer tests.",
                ),
                (
                    "iterative_convergence",
                    "Demonstrate iterative convergence for each admitted solve.",
                ),
                (
                    "conservation_and_consistency",
                    "Demonstrate relevant conservation and consistency properties.",
                ),
                (
                    "spatial_and_temporal_convergence",
                    "Quantify spatial and temporal discretization error as applicable.",
                ),
                (
                    "model_form_sensitivity",
                    "Assess constitutive, turbulence, transition and boundary-model choices.",
                ),
            ] {
                required.insert(control);
            }
        }
        ModelCreation::Empirical => {
            for control in [
                (
                    "dataset_source_and_license_provenance",
                    "Retain source, license, version and exact data identities.",
                ),
                (
                    "train_validation_test_separation",
                    "Demonstrate split independence and leakage controls.",
                ),
                (
                    "predictive_calibration",
                    "Measure predictive interval coverage and calibration on held-out data.",
                ),
                (
                    "applicability_domain_and_ood",
                    "Detect distribution shift and abstain outside the supported domain.",
                ),
            ] {
                required.insert(control);
            }
        }
        ModelCreation::Hybrid => {
            for control in [
                (
                    "code_verification",
                    "Verify mechanistic implementation with analytical/known-answer tests.",
                ),
                (
                    "iterative_convergence",
                    "Demonstrate iterative convergence for each admitted solve.",
                ),
                (
                    "conservation_and_consistency",
                    "Demonstrate relevant conservation and consistency properties.",
                ),
                (
                    "spatial_and_temporal_convergence",
                    "Quantify spatial and temporal discretization error as applicable.",
                ),
                (
                    "model_form_sensitivity",
                    "Assess constitutive, turbulence, transition and boundary-model choices.",
                ),
                (
                    "dataset_source_and_license_provenance",
                    "Retain source, license, version and exact data identities.",
                ),
                (
                    "train_validation_test_separation",
                    "Demonstrate split independence and leakage controls.",
                ),
                (
                    "predictive_calibration",
                    "Measure predictive interval coverage and calibration on held-out data.",
                ),
                (
                    "applicability_domain_and_ood",
                    "Detect distribution shift and abstain outside the supported domain.",
                ),
                (
                    "coupling_and_interface_verification",
                    "Verify data exchange, units, conservation and coupled-error propagation.",
                ),
            ] {
                required.insert(control);
            }
        }
    }

    match context.logic_transparency {
        LogicTransparency::Inspectable => {}
        LogicTransparency::PartiallyInspectable => {
            required.insert((
                "independent_interface_and_decision_trace_audit",
                "Independently inspect exposed logic, interfaces and decision traces.",
            ));
        }
        LogicTransparency::Opaque => {
            required.insert((
                "independent_black_box_reproduction",
                "Reproduce outputs from pinned inputs without relying on vendor claims.",
            ));
        }
    }

    let mut restrictions = Vec::new();
    match context.artifact_access {
        ArtifactAccess::IndependentReplayAvailable => {
            required.insert((
                "independent_artifact_replay",
                "A reviewer must be able to replay the exact model and run artifacts.",
            ));
        }
        ArtifactAccess::Restricted => {
            required.insert((
                "approved_third_party_artifact_audit",
                "An authorized independent party must inspect restricted artifacts.",
            ));
            restrictions.push(
                "No unrestricted reproducibility claim while artifacts remain access-restricted.",
            );
        }
        ArtifactAccess::Unavailable => {
            restrictions.push("No independent verification or validation claim without access to the decisive artifacts.");
            restrictions.push("Do not admit model outputs as verified design evidence.");
        }
    }

    match tier {
        CredibilityTier::Screening => {
            restrictions.push(
                "No design release, manufacturing authorization or operational safety claim.",
            );
        }
        CredibilityTier::Low => {
            restrictions.push("No design release or safety certification from this assessment.");
        }
        CredibilityTier::Moderate => {
            required.insert((
                "experimental_comparison",
                "Compare relevant predictions with independent measurements and their uncertainty.",
            ));
            required.insert((
                "decision_uncertainty_budget",
                "Propagate numerical, input, experimental and model-form uncertainty as applicable.",
            ));
            restrictions
                .push("No safety-critical reliance without a separate qualified safety case.");
        }
        CredibilityTier::High => {
            for control in [
                (
                    "blind_validation_holdout",
                    "Evaluate on untouched geometries/conditions not used for tuning.",
                ),
                (
                    "independent_reproduction",
                    "An independent team must reproduce the central result.",
                ),
                (
                    "decision_uncertainty_budget",
                    "Propagate numerical, input, experimental and model-form uncertainty as applicable.",
                ),
                (
                    "experimental_comparison",
                    "Compare relevant predictions with independent measurements and their uncertainty.",
                ),
            ] {
                required.insert(control);
            }
            restrictions.push(
                "No safety-critical use without independent review and a qualified safety case.",
            );
        }
        CredibilityTier::Critical => {
            for control in [
                (
                    "blind_validation_holdout",
                    "Evaluate on untouched geometries/conditions not used for tuning.",
                ),
                (
                    "independent_reproduction",
                    "An independent team must reproduce the central result.",
                ),
                (
                    "decision_uncertainty_budget",
                    "Propagate numerical, input, experimental and model-form uncertainty as applicable.",
                ),
                (
                    "experimental_comparison",
                    "Compare relevant predictions with independent measurements and their uncertainty.",
                ),
                (
                    "physical_test_or_justified_equivalent",
                    "Require a physical test or a formally justified alternative evidence route.",
                ),
                (
                    "independent_safety_case_and_authorization",
                    "Require independent hazard review and explicit authorization outside this software.",
                ),
                (
                    "fail_safe_human_override",
                    "Provide a safe fallback and human override for consequential decisions.",
                ),
            ] {
                required.insert(control);
            }
            restrictions.push("This assessment cannot authorize flight, manufacturing release, or safety-critical operation.");
        }
    }

    Ok(ModelCredibilityAssessment {
        model_id: context.model_id.clone(),
        intended_use: context.intended_use.clone(),
        model_creation: context.model_creation,
        logic_transparency: context.logic_transparency,
        artifact_access: context.artifact_access,
        model_influence: context.model_influence,
        decision_consequence: context.decision_consequence,
        tier,
        classification_reference: context.classification_reference.clone(),
        influence_rationale: context.influence_rationale.clone(),
        consequence_rationale: context.consequence_rationale.clone(),
        requirements: required
            .into_iter()
            .map(|(control_id, rationale)| CredibilityRequirement {
                control_id,
                rationale,
            })
            .collect(),
        restrictions,
        status: "requirements_assessment_only_no_evidence_completion_or_release_claim",
    })
}

pub const CREDIBILITY_CSV_HEADER: &str = "model_id,intended_use,model_creation,logic_transparency,artifact_access,model_influence,decision_consequence,credibility_tier,classification_reference,influence_rationale,consequence_rationale,required_control_count,required_controls,restrictions,status";

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn report_to_csv(assessment: &ModelCredibilityAssessment) -> String {
    let controls = assessment
        .requirements
        .iter()
        .map(|requirement| requirement.control_id)
        .collect::<Vec<_>>()
        .join(";");
    let restrictions = assessment.restrictions.join(";");
    format!(
        "{CREDIBILITY_CSV_HEADER}\n{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        csv_escape(&assessment.model_id),
        csv_escape(&assessment.intended_use),
        assessment.model_creation.as_str(),
        assessment.logic_transparency.as_str(),
        assessment.artifact_access.as_str(),
        assessment.model_influence.as_str(),
        assessment.decision_consequence.as_str(),
        assessment.tier.as_str(),
        csv_escape(&assessment.classification_reference),
        csv_escape(&assessment.influence_rationale),
        csv_escape(&assessment.consequence_rationale),
        assessment.requirements.len(),
        csv_escape(&controls),
        csv_escape(&restrictions),
        assessment.status,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(
        creation: ModelCreation,
        influence: ModelInfluence,
        consequence: DecisionConsequence,
    ) -> ModelCredibilityContext {
        ModelCredibilityContext {
            model_id: "synthetic-model".to_owned(),
            intended_use: "offline civil design exploration".to_owned(),
            model_creation: creation,
            logic_transparency: LogicTransparency::Inspectable,
            artifact_access: ArtifactAccess::IndependentReplayAvailable,
            model_influence: influence,
            decision_consequence: consequence,
            influence_rationale: "synthetic caller-declared influence".to_owned(),
            consequence_rationale: "synthetic caller-declared consequence".to_owned(),
            classification_reference: "luminous-risk-matrix-v0.1-synthetic".to_owned(),
        }
    }

    #[test]
    fn qualitative_matrix_is_explicit_and_monotone() {
        assert_eq!(
            classify_credibility_tier(ModelInfluence::Advisory, DecisionConsequence::Limited),
            CredibilityTier::Screening
        );
        assert_eq!(
            classify_credibility_tier(ModelInfluence::Material, DecisionConsequence::Significant),
            CredibilityTier::Moderate
        );
        assert_eq!(
            classify_credibility_tier(ModelInfluence::Controlling, DecisionConsequence::Severe),
            CredibilityTier::Critical
        );
        for consequence in [
            DecisionConsequence::Limited,
            DecisionConsequence::Significant,
            DecisionConsequence::Severe,
        ] {
            assert!(
                classify_credibility_tier(ModelInfluence::Advisory, consequence)
                    <= classify_credibility_tier(ModelInfluence::Material, consequence)
            );
            assert!(
                classify_credibility_tier(ModelInfluence::Material, consequence)
                    <= classify_credibility_tier(ModelInfluence::Controlling, consequence)
            );
        }
    }

    #[test]
    fn increasing_influence_or_consequence_never_removes_a_required_control() {
        let influences = [
            ModelInfluence::Advisory,
            ModelInfluence::Material,
            ModelInfluence::Controlling,
        ];
        let consequences = [
            DecisionConsequence::Limited,
            DecisionConsequence::Significant,
            DecisionConsequence::Severe,
        ];
        let ids = |assessment: &ModelCredibilityAssessment| {
            assessment
                .requirements
                .iter()
                .map(|requirement| requirement.control_id)
                .collect::<BTreeSet<_>>()
        };

        for consequence in consequences {
            for pair in influences.windows(2) {
                let lower =
                    assess_model_credibility(&context(ModelCreation::Hybrid, pair[0], consequence))
                        .expect("lower influence assessment");
                let higher =
                    assess_model_credibility(&context(ModelCreation::Hybrid, pair[1], consequence))
                        .expect("higher influence assessment");
                assert!(ids(&lower).is_subset(&ids(&higher)));
            }
        }
        for influence in influences {
            for pair in consequences.windows(2) {
                let lower =
                    assess_model_credibility(&context(ModelCreation::Hybrid, influence, pair[0]))
                        .expect("lower consequence assessment");
                let higher =
                    assess_model_credibility(&context(ModelCreation::Hybrid, influence, pair[1]))
                        .expect("higher consequence assessment");
                assert!(ids(&lower).is_subset(&ids(&higher)));
            }
        }
    }

    #[test]
    fn model_creation_type_adds_distinct_assurance_controls() {
        let mechanistic = assess_model_credibility(&context(
            ModelCreation::Mechanistic,
            ModelInfluence::Material,
            DecisionConsequence::Significant,
        ))
        .expect("mechanistic profile");
        let empirical = assess_model_credibility(&context(
            ModelCreation::Empirical,
            ModelInfluence::Material,
            DecisionConsequence::Significant,
        ))
        .expect("empirical profile");
        let hybrid = assess_model_credibility(&context(
            ModelCreation::Hybrid,
            ModelInfluence::Material,
            DecisionConsequence::Significant,
        ))
        .expect("hybrid profile");

        let contains = |assessment: &ModelCredibilityAssessment, id: &str| {
            assessment
                .requirements
                .iter()
                .any(|control| control.control_id == id)
        };
        assert!(contains(&mechanistic, "spatial_and_temporal_convergence"));
        assert!(contains(&empirical, "train_validation_test_separation"));
        assert!(contains(&hybrid, "spatial_and_temporal_convergence"));
        assert!(contains(&hybrid, "train_validation_test_separation"));
        assert!(contains(&hybrid, "coupling_and_interface_verification"));
    }

    #[test]
    fn critical_opaque_unavailable_model_has_explicit_prohibitions() {
        let mut input = context(
            ModelCreation::Empirical,
            ModelInfluence::Controlling,
            DecisionConsequence::Severe,
        );
        input.logic_transparency = LogicTransparency::Opaque;
        input.artifact_access = ArtifactAccess::Unavailable;
        let assessment = assess_model_credibility(&input).expect("risk requirement assessment");
        assert_eq!(assessment.tier, CredibilityTier::Critical);
        assert!(
            assessment
                .requirements
                .iter()
                .any(|control| control.control_id == "independent_black_box_reproduction")
        );
        assert!(
            assessment
                .restrictions
                .iter()
                .any(|restriction| restriction.contains("No independent verification"))
        );
        assert!(
            assessment
                .restrictions
                .iter()
                .any(|restriction| restriction.contains("cannot authorize flight"))
        );
        assert_eq!(
            assessment.status,
            "requirements_assessment_only_no_evidence_completion_or_release_claim"
        );
    }

    #[test]
    fn missing_rationale_and_reference_fail_closed() {
        let mut input = context(
            ModelCreation::Mechanistic,
            ModelInfluence::Advisory,
            DecisionConsequence::Limited,
        );
        input.influence_rationale.clear();
        assert!(
            assess_model_credibility(&input)
                .expect_err("missing rationale rejected")
                .contains("influence_rationale")
        );

        let mut input = context(
            ModelCreation::Mechanistic,
            ModelInfluence::Advisory,
            DecisionConsequence::Limited,
        );
        input.classification_reference.clear();
        assert!(
            assess_model_credibility(&input)
                .expect_err("missing rubric identity rejected")
                .contains("classification_reference")
        );
    }

    #[test]
    fn csv_is_deterministic_and_never_claims_completed_assurance() {
        let assessment = assess_model_credibility(&context(
            ModelCreation::Hybrid,
            ModelInfluence::Controlling,
            DecisionConsequence::Significant,
        ))
        .expect("valid assessment");
        let first = report_to_csv(&assessment);
        assert_eq!(first, report_to_csv(&assessment));
        assert_eq!(first.lines().count(), 2);
        assert!(
            first.contains("requirements_assessment_only_no_evidence_completion_or_release_claim")
        );
        assert!(first.contains("independent_reproduction"));
        assert!(first.contains("train_validation_test_separation"));
    }
}
