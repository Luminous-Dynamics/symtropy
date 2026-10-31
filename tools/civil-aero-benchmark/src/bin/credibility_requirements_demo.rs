use std::process;

use civil_aero_benchmark::risk_credibility::{
    ArtifactAccess, DecisionConsequence, LogicTransparency, ModelCreation, ModelCredibilityContext,
    ModelInfluence, assess_model_credibility, report_to_csv,
};

fn context(
    model_id: &str,
    intended_use: &str,
    creation: ModelCreation,
    logic: LogicTransparency,
    artifacts: ArtifactAccess,
    influence: ModelInfluence,
    consequence: DecisionConsequence,
) -> ModelCredibilityContext {
    ModelCredibilityContext {
        model_id: model_id.to_owned(),
        intended_use: intended_use.to_owned(),
        model_creation: creation,
        logic_transparency: logic,
        artifact_access: artifacts,
        model_influence: influence,
        decision_consequence: consequence,
        influence_rationale: "synthetic demonstration label; not a real system assessment".to_owned(),
        consequence_rationale: "synthetic demonstration label; not a real hazard analysis".to_owned(),
        classification_reference: "luminous-risk-matrix-v0.1-synthetic-demo".to_owned(),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("credibility requirements demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let contexts = [
        context(
            "concept-screen",
            "offline concept ranking only",
            ModelCreation::Mechanistic,
            LogicTransparency::Inspectable,
            ArtifactAccess::IndependentReplayAvailable,
            ModelInfluence::Advisory,
            DecisionConsequence::Limited,
        ),
        context(
            "mission-design-support",
            "mission design recommendation",
            ModelCreation::Hybrid,
            LogicTransparency::PartiallyInspectable,
            ArtifactAccess::Restricted,
            ModelInfluence::Material,
            DecisionConsequence::Severe,
        ),
        context(
            "safety-critical-controller",
            "safety-critical operational control",
            ModelCreation::Empirical,
            LogicTransparency::Opaque,
            ArtifactAccess::Unavailable,
            ModelInfluence::Controlling,
            DecisionConsequence::Severe,
        ),
    ];

    let mut output =
        format!("{}\n", civil_aero_benchmark::risk_credibility::CREDIBILITY_CSV_HEADER);
    for input in &contexts {
        let assessment = assess_model_credibility(input)?;
        let record = report_to_csv(&assessment);
        let row = record
            .lines()
            .nth(1)
            .ok_or_else(|| "credibility assessment CSV row is missing".to_owned())?;
        output.push_str(row);
        output.push('\n');
    }
    print!("{output}");
    eprintln!(
        "synthetic_credibility_policy_demo profiles=3 status=requirements_assessment_only_no_evidence_completion_or_release_claim"
    );
    Ok(())
}
