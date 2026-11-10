// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::claim_assurance::{
    ASSURANCE_POLICY_VERSION, AssuranceContext, AssuranceStatus, ClaimAssuranceReport,
    ClaimClass, EvidenceObligation, EvidenceReceipt, EvidenceStatus, EvidenceTier,
    EvidenceType, OrdinalLevel, ObligationStatus, assess_claim_assurance,
    required_obligations,
};

const CSV_COLUMNS: usize = 18;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "claim assurance receipt must match frozen schema"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn context(
    claim: ClaimClass,
    influence: OrdinalLevel,
    consequence: OrdinalLevel,
    suffix: &str,
) -> AssuranceContext {
    AssuranceContext {
        context_id: format!("district-water-decision-{suffix}"),
        context_digest: format!("sha256-water-context-{suffix}"),
        decision_id: "district-water-allocation".to_string(),
        model_digest: "sha256-civdyn-water-model".to_string(),
        configuration_digest: "sha256-civdyn-water-config".to_string(),
        population_id: "district-households".to_string(),
        time_horizon: "2026-2030".to_string(),
        source_digest: Some("sha256-water-observations".to_string()),
        calibration_digest: Some("sha256-water-calibration".to_string()),
        split_manifest_digest: Some("sha256-water-challenge-split".to_string()),
        model_influence: influence,
        decision_consequence: consequence,
        claim,
    }
}

fn receipt_for(
    context: &AssuranceContext,
    requirement: EvidenceObligation,
    scenario: &str,
) -> EvidenceReceipt {
    EvidenceReceipt {
        receipt_id: format!("{scenario}-{}", requirement.evidence_type.name()),
        evidence_type: requirement.evidence_type,
        status: EvidenceStatus::Pass,
        evidence_tier: requirement.minimum_tier,
        model_digest: context.model_digest.clone(),
        configuration_digest: context.configuration_digest.clone(),
        context_digest: context.context_digest.clone(),
        population_id: context.population_id.clone(),
        time_horizon: context.time_horizon.clone(),
        source_digest: if requirement.requires_source_digest {
            context.source_digest.clone()
        } else {
            None
        },
        calibration_digest: if requirement.requires_calibration_digest {
            context.calibration_digest.clone()
        } else {
            None
        },
        split_manifest_digest: if requirement.requires_split_manifest_digest {
            context.split_manifest_digest.clone()
        } else {
            None
        },
        applicable_claims: vec![context.claim],
        assessor_id: format!("assessor-{scenario}"),
        independent_review: requirement.requires_independent_review,
        receipt_digest: format!("sha256-{scenario}-{}", requirement.evidence_type.name()),
    }
}

fn complete_receipts(
    context: &AssuranceContext,
    scenario: &str,
) -> Vec<EvidenceReceipt> {
    let tier = match context.model_influence.max(context.decision_consequence) {
        OrdinalLevel::Low => EvidenceTier::Tier1,
        OrdinalLevel::Moderate => EvidenceTier::Tier2,
        OrdinalLevel::High => EvidenceTier::Tier3,
    };
    required_obligations(context.claim, tier)
        .expect("well-formed claim requirements")
        .into_iter()
        .map(|requirement| receipt_for(context, requirement, scenario))
        .collect()
}

fn emit_report(
    scenario: &str,
    context: &AssuranceContext,
    report: &ClaimAssuranceReport,
) {
    for obligation in &report.obligations {
        emit(vec![
            scenario.to_string(),
            report.claim.name().to_string(),
            report.model_influence.name().to_string(),
            report.decision_consequence.name().to_string(),
            report.evidence_tier.name().to_string(),
            report.status.name().to_string(),
            report.policy_version.clone(),
            obligation.evidence_type.name().to_string(),
            obligation.minimum_tier.name().to_string(),
            obligation.status.name().to_string(),
            obligation.matched_receipt_id.clone().unwrap_or_default(),
            obligation.explanation_code.to_string(),
            context.model_digest.clone(),
            context.configuration_digest.clone(),
            context.context_digest.clone(),
            context.decision_id.clone(),
            report.qualified_obligation_count.to_string(),
            report.required_obligation_count.to_string(),
        ]);
    }
}

fn run_scenario(
    scenario: &str,
    context: &AssuranceContext,
    receipts: &[EvidenceReceipt],
) -> Result<(), Box<dyn std::error::Error>> {
    let report = assess_claim_assurance(context, receipts)?;
    emit_report(scenario, context, &report);
    Ok(())
}

fn tier_one_only(
    context: &AssuranceContext,
    scenario: &str,
) -> Vec<EvidenceReceipt> {
    required_obligations(context.claim, EvidenceTier::Tier1)
        .expect("Tier 1 requirements")
        .into_iter()
        .filter(|requirement| requirement.minimum_tier == EvidenceTier::Tier1)
        .map(|requirement| receipt_for(context, requirement, scenario))
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,claim,model_influence,decision_consequence,evidence_tier,assurance_status,policy_version,evidence_type,minimum_tier,obligation_status,matched_receipt_id,explanation_code,model_digest,configuration_digest,context_digest,decision_id,qualified_obligation_count,required_obligation_count"
    );

    let exploratory = context(
        ClaimClass::Exploratory,
        OrdinalLevel::Low,
        OrdinalLevel::Low,
        "explore",
    );
    let exploratory_receipts = complete_receipts(&exploratory, "explore");
    run_scenario("low_risk_exploration", &exploratory, &exploratory_receipts)?;

    let forecast = context(
        ClaimClass::Forecast,
        OrdinalLevel::Low,
        OrdinalLevel::Low,
        "forecast",
    );
    let forecast_receipts = complete_receipts(&forecast, "forecast");
    run_scenario("forecast_with_split_evidence", &forecast, &forecast_receipts)?;
    let exploratory_only = tier_one_only(&forecast, "forecast-from-exploration");
    run_scenario("forecast_without_predictive_evidence", &forecast, &exploratory_only)?;

    let high_influence = context(
        ClaimClass::Exploratory,
        OrdinalLevel::High,
        OrdinalLevel::Low,
        "high-influence",
    );
    let high_influence_tier_one = tier_one_only(&high_influence, "high-influence");
    run_scenario(
        "high_influence_requires_tier_three",
        &high_influence,
        &high_influence_tier_one,
    )?;

    let causal = context(
        ClaimClass::CausalEffect,
        OrdinalLevel::Low,
        OrdinalLevel::Low,
        "causal",
    );
    let mut causal_receipts = complete_receipts(&causal, "causal");
    causal_receipts.retain(|receipt| {
        receipt.evidence_type != EvidenceType::IdentificationAssumptions
    });
    run_scenario("causal_without_identification", &causal, &causal_receipts)?;

    let policy = context(
        ClaimClass::PolicyRecommendation,
        OrdinalLevel::High,
        OrdinalLevel::High,
        "policy",
    );
    let policy_receipts = complete_receipts(&policy, "policy");
    run_scenario("high_risk_policy_complete", &policy, &policy_receipts)?;

    let mut failed_policy_receipts = policy_receipts.clone();
    let conservation = failed_policy_receipts
        .iter_mut()
        .find(|receipt| receipt.evidence_type == EvidenceType::PhysicalConservation)
        .expect("physical conservation receipt");
    conservation.status = EvidenceStatus::Fail;
    run_scenario(
        "policy_with_failed_conservation",
        &policy,
        &failed_policy_receipts,
    )?;

    let stale_context = context(
        ClaimClass::Exploratory,
        OrdinalLevel::Low,
        OrdinalLevel::Low,
        "stale",
    );
    let mut stale_receipts = complete_receipts(&stale_context, "stale");
    stale_receipts[0].configuration_digest = "sha256-old-configuration".to_string();
    run_scenario("stale_configuration_receipt", &stale_context, &stale_receipts)?;

    // Keep these imported contracts visibly tied to the CSV/status vocabulary.
    assert_eq!(ASSURANCE_POLICY_VERSION, "civdyn-claim-assurance-v0.1");
    assert_eq!(AssuranceStatus::QualifiedForDeclaredUse.name(), "qualified_for_declared_use");
    assert_eq!(ObligationStatus::Satisfied.name(), "satisfied");
    Ok(())
}
