// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Read-only structural and semantic conformance checks for the frozen
//! Living World Ethical Impact fixture corpus.
//!
//! This validator does not evaluate moral truth, infer sentience, authenticate
//! evidence, calculate ecological/thermodynamic quantities, or authorize action.
//! It verifies that fixture references and declared claim boundaries remain
//! internally consistent.

use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};

const EXPECTED_SCHEMA_VERSION: &str = "0.1.0";
const EXPECTED_CORPUS_ID: &str = "LIVING-WORLD-ETHICAL-IMPACT-001/v0.1";
const EXPECTED_PROFILE_STATUS_SEMANTICS: &str =
    "conditional_premise_finding_not_global_moral_verdict";

const FIXTURE_JSON: &str =
    include_str!("../../living-world-ethical-impact/v0.1/fixtures.json");
const SCHEMA_JSON: &str =
    include_str!("../../living-world-ethical-impact/v0.1/schema.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    corpus_id: String,
    claim_ceiling: String,
    profiles: Vec<Profile>,
    anti_derivation_controls: Vec<AntiDerivationControl>,
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    profile_id: String,
    description: String,
    premises: Vec<Premise>,
    status_semantics: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Premise {
    premise_id: String,
    statement: String,
    required_dimensions: Vec<EvidenceDimension>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AntiDerivationControl {
    control_id: String,
    prohibited_inference: String,
    required_behavior: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    fixture_id: String,
    factset_id: String,
    family: FixtureFamily,
    scenario: String,
    evidence_class: EvidenceClass,
    evidence: Vec<Evidence>,
    affected_parties: Vec<AffectedParty>,
    action_options: Vec<ActionOption>,
    thermodynamics: Thermodynamics,
    expected_findings: Vec<ExpectedFinding>,
    required_control_ids: Vec<String>,
    human_review_required: bool,
    no_action_authority: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
enum FixtureFamily {
    TumourParasiteCountermodel,
    StableExploitation,
    HighDissipationLowerWelfare,
    PredatorPreyConflict,
    InvasiveSpeciesManagement,
    ForestRestoration,
    FungalNetwork,
    WaterCommons,
    UncertainPossibleMoralPatient,
    ReversibleIrreversibleIntervention,
    EcologicalValueWithoutSentienceEvidence,
    SameFactsDifferentPremises,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    SyntheticKnownAnswer,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
enum EvidenceDimension {
    PhysicalFeasibility,
    EcologicalIntegrity,
    IndividualWelfare,
    MoralPatientUncertainty,
    HumanCommunityImpacts,
    IntergenerationalEffects,
    ConsentAndCustodianship,
    ThermodynamicObservables,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum EvidenceStatus {
    Observed,
    Modelled,
    Missing,
    NotTested,
    NotObservable,
    Contradictory,
    OutOfDomain,
    NotApplicable,
    Unqualified,
    NotRecorded,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    evidence_id: String,
    dimension: EvidenceDimension,
    status: EvidenceStatus,
    claim: String,
    unit: Option<String>,
    source_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AffectedParty {
    party_id: String,
    level: PartyLevel,
    impact: Impact,
    evidence_refs: Vec<String>,
    consent_status: ConsentStatus,
    time_horizon: TimeHorizon,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PartyLevel {
    Individual,
    Population,
    Ecosystem,
    Community,
    Institution,
    FutureGeneration,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Impact {
    Benefit,
    Harm,
    Mixed,
    Unknown,
    NoClaim,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ConsentStatus {
    Granted,
    Refused,
    NotRecorded,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum TimeHorizon {
    Immediate,
    ShortTerm,
    LongTerm,
    Intergenerational,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionOption {
    option_id: String,
    description: String,
    reversibility: Reversibility,
    monitoring: Monitoring,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Reversibility {
    Reversible,
    StagedReversible,
    PartlyReversible,
    Irreversible,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Monitoring {
    Continuous,
    Periodic,
    None,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Thermodynamics {
    status: ThermodynamicStatus,
    qualified_observables: Vec<ThermodynamicObservable>,
    notes: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ThermodynamicStatus {
    NotQualified,
    QualifiedWithinScope,
    Inconclusive,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ThermodynamicObservable {
    OpenSystemEnergyBalance,
    EntropyProduction,
    ExergyBalance,
    HeatFlow,
    WorkFlow,
    LandauerErasureBound,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedFinding {
    profile_id: String,
    premise_id: String,
    status: FindingStatus,
    evidence_refs: Vec<String>,
    action_option_ids: Vec<String>,
    rationale: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum FindingStatus {
    Supported,
    Contradicted,
    Undetermined,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ValidationSummary {
    pub fixture_count: usize,
    pub profile_count: usize,
    pub premise_count: usize,
    pub anti_derivation_control_count: usize,
    pub evidence_reference_count: usize,
    pub action_reference_count: usize,
}

/// Validate the checked-in v0.1 corpus and its closed schema contract.
pub fn validate_builtin_corpus() -> Result<ValidationSummary, Vec<String>> {
    let corpus: Corpus = serde_json::from_str(FIXTURE_JSON)
        .map_err(|error| vec![format!("fixture JSON parse failed: {error}")])?;
    let schema: serde_json::Value = serde_json::from_str(SCHEMA_JSON)
        .map_err(|error| vec![format!("schema JSON parse failed: {error}")])?;

    let mut errors = validate_schema_contract(&schema);
    match validate_corpus(&corpus) {
        Ok(summary) if errors.is_empty() => Ok(summary),
        Ok(_) => Err(errors),
        Err(mut corpus_errors) => {
            errors.append(&mut corpus_errors);
            Err(errors)
        }
    }
}

fn validate_schema_contract(schema: &serde_json::Value) -> Vec<String> {
    let mut errors = Vec::new();
    if schema["$schema"] != "https://json-schema.org/draft/2020-12/schema" {
        errors.push("schema must declare JSON Schema Draft 2020-12".into());
    }
    if schema["additionalProperties"] != false {
        errors.push("top-level schema must reject additional properties".into());
    }
    if schema["properties"]["fixtures"]["minItems"] != 12 {
        errors.push("schema fixture minimum must remain 12".into());
    }
    if schema["properties"]["fixtures"]["maxItems"] != 12 {
        errors.push("schema fixture maximum must remain 12".into());
    }

    let expected_required = schema["properties"]["fixtures"]["items"]["properties"]
        ["expected_findings"]["items"]["required"]
        .as_array();
    let has_action_scope = expected_required.is_some_and(|required| {
        required
            .iter()
            .any(|field| field.as_str() == Some("action_option_ids"))
    });
    if !has_action_scope {
        errors.push("expected findings must require action_option_ids".into());
    }

    let evidence_statuses = schema["properties"]["fixtures"]["items"]["properties"]["evidence"]
        ["items"]["properties"]["status"]["enum"]
        .as_array();
    let has_not_recorded = evidence_statuses.is_some_and(|statuses| {
        statuses
            .iter()
            .any(|status| status.as_str() == Some("not_recorded"))
    });
    if !has_not_recorded {
        errors.push("evidence status enum must preserve not_recorded".into());
    }
    errors
}

fn validate_corpus(corpus: &Corpus) -> Result<ValidationSummary, Vec<String>> {
    let mut errors = Vec::new();

    if corpus.schema_version != EXPECTED_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {EXPECTED_SCHEMA_VERSION}, got {}",
            corpus.schema_version
        ));
    }
    if corpus.corpus_id != EXPECTED_CORPUS_ID {
        errors.push(format!(
            "corpus_id must be {EXPECTED_CORPUS_ID}, got {}",
            corpus.corpus_id
        ));
    }
    require_nonempty("claim_ceiling", &corpus.claim_ceiling, &mut errors);
    if corpus.profiles.len() < 6 {
        errors.push("at least six premise profiles are required".into());
    }
    if corpus.anti_derivation_controls.len() < 12 {
        errors.push("at least twelve anti-derivation controls are required".into());
    }
    if corpus.fixtures.len() != 12 {
        errors.push(format!(
            "exactly twelve v0.1 fixtures are required, got {}",
            corpus.fixtures.len()
        ));
    }

    let mut profile_ids = BTreeSet::new();
    let mut premise_index: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut premise_count = 0;
    for profile in &corpus.profiles {
        require_nonempty("profile_id", &profile.profile_id, &mut errors);
        require_nonempty("profile description", &profile.description, &mut errors);
        insert_unique(
            &mut profile_ids,
            &profile.profile_id,
            "profile_id",
            &mut errors,
        );
        if profile.status_semantics != EXPECTED_PROFILE_STATUS_SEMANTICS {
            errors.push(format!(
                "profile {} must retain conditional, non-global status semantics",
                profile.profile_id
            ));
        }
        if profile.premises.is_empty() {
            errors.push(format!(
                "profile {} must declare at least one premise",
                profile.profile_id
            ));
        }
        let mut premise_ids = BTreeSet::new();
        for premise in &profile.premises {
            premise_count += 1;
            require_nonempty("premise_id", &premise.premise_id, &mut errors);
            require_nonempty("premise statement", &premise.statement, &mut errors);
            if premise.required_dimensions.is_empty() {
                errors.push(format!(
                    "premise {} in {} has no required evidence dimensions",
                    premise.premise_id, profile.profile_id
                ));
            }
            let mut dimensions = BTreeSet::new();
            for dimension in &premise.required_dimensions {
                if !dimensions.insert(*dimension) {
                    errors.push(format!(
                        "premise {} in {} repeats a required dimension",
                        premise.premise_id, profile.profile_id
                    ));
                }
            }
            insert_unique(
                &mut premise_ids,
                &premise.premise_id,
                "premise_id within profile",
                &mut errors,
            );
        }
        premise_index.insert(profile.profile_id.clone(), premise_ids);
    }

    let mut control_ids = BTreeSet::new();
    for control in &corpus.anti_derivation_controls {
        require_nonempty("control_id", &control.control_id, &mut errors);
        require_nonempty(
            "prohibited_inference",
            &control.prohibited_inference,
            &mut errors,
        );
        require_nonempty(
            "required_behavior",
            &control.required_behavior,
            &mut errors,
        );
        insert_unique(
            &mut control_ids,
            &control.control_id,
            "control_id",
            &mut errors,
        );
    }

    let mut fixture_ids = BTreeSet::new();
    let mut factset_ids = BTreeSet::new();
    let mut families = BTreeSet::new();
    let mut evidence_reference_count = 0;
    let mut action_reference_count = 0;

    for fixture in &corpus.fixtures {
        require_nonempty("fixture_id", &fixture.fixture_id, &mut errors);
        require_nonempty("factset_id", &fixture.factset_id, &mut errors);
        require_nonempty("scenario", &fixture.scenario, &mut errors);
        insert_unique(
            &mut fixture_ids,
            &fixture.fixture_id,
            "fixture_id",
            &mut errors,
        );
        insert_unique(
            &mut factset_ids,
            &fixture.factset_id,
            "factset_id",
            &mut errors,
        );
        if !families.insert(fixture.family) {
            errors.push(format!(
                "fixture family {:?} occurs more than once",
                fixture.family
            ));
        }
        if fixture.evidence_class != EvidenceClass::SyntheticKnownAnswer {
            errors.push(format!(
                "fixture {} must declare synthetic_known_answer evidence",
                fixture.fixture_id
            ));
        }
        if fixture.evidence.is_empty() {
            errors.push(format!(
                "fixture {} must include explicit evidence states",
                fixture.fixture_id
            ));
        }
        if fixture.expected_findings.is_empty() {
            errors.push(format!(
                "fixture {} must include at least one expected premise finding",
                fixture.fixture_id
            ));
        }
        if !fixture.human_review_required {
            errors.push(format!(
                "fixture {} must require human review",
                fixture.fixture_id
            ));
        }
        if !fixture.no_action_authority {
            errors.push(format!(
                "fixture {} must grant no action authority",
                fixture.fixture_id
            ));
        }
        if fixture.thermodynamics.status != ThermodynamicStatus::NotQualified
            || !fixture.thermodynamics.qualified_observables.is_empty()
        {
            errors.push(format!(
                "fixture {} must keep thermodynamic observables unqualified in v0.1",
                fixture.fixture_id
            ));
        }
        require_nonempty(
            "thermodynamics notes",
            &fixture.thermodynamics.notes,
            &mut errors,
        );

        let mut evidence_ids = BTreeSet::new();
        for evidence in &fixture.evidence {
            require_nonempty("evidence_id", &evidence.evidence_id, &mut errors);
            require_nonempty("evidence claim", &evidence.claim, &mut errors);
            if let Some(unit) = &evidence.unit {
                require_nonempty("evidence unit", unit, &mut errors);
            }
            if let Some(source_ref) = &evidence.source_ref {
                require_nonempty("evidence source_ref", source_ref, &mut errors);
            }
            insert_unique(
                &mut evidence_ids,
                &evidence.evidence_id,
                "evidence_id within fixture",
                &mut errors,
            );
            let _preserved_evidence_status = evidence.status;
            let _typed_dimension = evidence.dimension;
        }

        let mut party_ids = BTreeSet::new();
        for party in &fixture.affected_parties {
            require_nonempty("party_id", &party.party_id, &mut errors);
            insert_unique(
                &mut party_ids,
                &party.party_id,
                "party_id within fixture",
                &mut errors,
            );
            let _typed_party_facts = (
                party.level,
                party.impact,
                party.consent_status,
                party.time_horizon,
            );
            validate_references(
                &fixture.fixture_id,
                "affected party",
                &party.evidence_refs,
                &evidence_ids,
                &mut evidence_reference_count,
                &mut errors,
            );
        }

        let mut action_ids = BTreeSet::new();
        for action in &fixture.action_options {
            require_nonempty("option_id", &action.option_id, &mut errors);
            require_nonempty("action description", &action.description, &mut errors);
            let _typed_action_profile = (action.reversibility, action.monitoring);
            insert_unique(
                &mut action_ids,
                &action.option_id,
                "action option_id within fixture",
                &mut errors,
            );
        }

        let mut finding_ids = BTreeSet::new();
        for finding in &fixture.expected_findings {
            require_nonempty("finding profile_id", &finding.profile_id, &mut errors);
            require_nonempty("finding premise_id", &finding.premise_id, &mut errors);
            require_nonempty("finding rationale", &finding.rationale, &mut errors);
            let pair = format!("{}::{}", finding.profile_id, finding.premise_id);
            insert_unique(
                &mut finding_ids,
                &pair,
                "profile/premise finding pair within fixture",
                &mut errors,
            );
            match premise_index.get(&finding.profile_id) {
                Some(premises) if premises.contains(&finding.premise_id) => {}
                _ => errors.push(format!(
                    "fixture {} references unknown premise {} in profile {}",
                    fixture.fixture_id, finding.premise_id, finding.profile_id
                )),
            }
            let _conditional_status = finding.status;
            validate_references(
                &fixture.fixture_id,
                "expected finding",
                &finding.evidence_refs,
                &evidence_ids,
                &mut evidence_reference_count,
                &mut errors,
            );
            for action_id in &finding.action_option_ids {
                action_reference_count += 1;
                if !action_ids.contains(action_id) {
                    errors.push(format!(
                        "fixture {} finding references unknown action option {}",
                        fixture.fixture_id, action_id
                    ));
                }
            }
            let mut unique_actions = BTreeSet::new();
            for action_id in &finding.action_option_ids {
                if !unique_actions.insert(action_id) {
                    errors.push(format!(
                        "fixture {} finding repeats action option {}",
                        fixture.fixture_id, action_id
                    ));
                }
            }
        }

        let mut unique_required_controls = BTreeSet::new();
        for control_id in &fixture.required_control_ids {
            if !control_ids.contains(control_id) {
                errors.push(format!(
                    "fixture {} references unknown anti-derivation control {}",
                    fixture.fixture_id, control_id
                ));
            }
            if !unique_required_controls.insert(control_id) {
                errors.push(format!(
                    "fixture {} repeats anti-derivation control {}",
                    fixture.fixture_id, control_id
                ));
            }
        }
        if fixture.required_control_ids.is_empty() {
            errors.push(format!(
                "fixture {} must require at least one anti-derivation control",
                fixture.fixture_id
            ));
        }
    }

    if families.len() != 12 {
        errors.push(format!(
            "v0.1 must cover twelve distinct scenario families, got {}",
            families.len()
        ));
    }

    if errors.is_empty() {
        Ok(ValidationSummary {
            fixture_count: corpus.fixtures.len(),
            profile_count: corpus.profiles.len(),
            premise_count,
            anti_derivation_control_count: corpus.anti_derivation_controls.len(),
            evidence_reference_count,
            action_reference_count,
        })
    } else {
        Err(errors)
    }
}

fn validate_references(
    fixture_id: &str,
    owner: &str,
    references: &[String],
    known: &BTreeSet<String>,
    count: &mut usize,
    errors: &mut Vec<String>,
) {
    let mut unique = BTreeSet::new();
    for reference in references {
        *count += 1;
        if !unique.insert(reference) {
            errors.push(format!(
                "fixture {fixture_id} {owner} repeats evidence reference {reference}"
            ));
        }
        if !known.contains(reference) {
            errors.push(format!(
                "fixture {fixture_id} {owner} references unknown evidence {reference}"
            ));
        }
    }
}

fn require_nonempty(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
}

fn insert_unique(
    set: &mut BTreeSet<String>,
    value: &str,
    field: &str,
    errors: &mut Vec<String>,
) {
    if !set.insert(value.to_owned()) {
        errors.push(format!("duplicate {field}: {value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parse_corpus(value: Value) -> Result<Corpus, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE_JSON).expect("bundled fixture JSON must parse")
    }

    #[test]
    fn bundled_corpus_passes_strict_structural_conformance() {
        let summary = validate_builtin_corpus().expect("corpus should pass");
        assert_eq!(summary.fixture_count, 12);
        assert_eq!(summary.profile_count, 6);
        assert_eq!(summary.anti_derivation_control_count, 12);
        assert!(summary.premise_count >= 6);
        assert!(summary.evidence_reference_count > 0);
    }

    #[test]
    fn schema_is_closed_and_requires_action_option_scope() {
        let schema: Value =
            serde_json::from_str(SCHEMA_JSON).expect("schema JSON must parse");
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["additionalProperties"], false);
        let required = schema["properties"]["fixtures"]["items"]["properties"]
            ["expected_findings"]["items"]["required"]
            .as_array()
            .expect("expected findings must declare required fields");
        assert!(required
            .iter()
            .any(|field| field == "action_option_ids"));
    }

    #[test]
    fn unrecorded_consent_is_a_distinct_parseable_evidence_state() {
        let mut value = fixture_value();
        value["fixtures"][0]["evidence"][0]["status"] = Value::String("not_recorded".into());
        let corpus = parse_corpus(value).expect("not_recorded must remain a valid state");
        assert_eq!(
            corpus.fixtures[0].evidence[0].status,
            EvidenceStatus::NotRecorded
        );
    }

    #[test]
    fn unknown_fields_fail_closed() {
        let mut value = fixture_value();
        value["fixtures"][0]["unrecognized_authority"] = Value::Bool(true);
        assert!(parse_corpus(value).is_err());
    }

    #[test]
    fn unknown_evidence_states_fail_closed() {
        let mut value = fixture_value();
        value["fixtures"][0]["evidence"][0]["status"] = Value::String("absent".into());
        assert!(parse_corpus(value).is_err());
    }

    #[test]
    fn duplicate_fixture_ids_are_rejected() {
        let mut corpus = parse_corpus(fixture_value()).unwrap();
        let duplicate_id = corpus.fixtures[0].fixture_id.clone();
        corpus.fixtures[1].fixture_id = duplicate_id;
        assert!(validate_corpus(&corpus).is_err());
    }

    #[test]
    fn unknown_evidence_references_are_rejected() {
        let mut corpus = parse_corpus(fixture_value()).unwrap();
        corpus.fixtures[0].expected_findings[0]
            .evidence_refs
            .push("evidence-that-does-not-exist".into());
        assert!(validate_corpus(&corpus).is_err());
    }

    #[test]
    fn unknown_action_options_are_rejected() {
        let mut corpus = parse_corpus(fixture_value()).unwrap();
        let finding = corpus
            .fixtures
            .iter_mut()
            .flat_map(|fixture| fixture.expected_findings.iter_mut())
            .find(|finding| !finding.action_option_ids.is_empty())
            .expect("at least one finding must be action-scoped");
        finding.action_option_ids.push("unlisted-action".into());
        assert!(validate_corpus(&corpus).is_err());
    }

    #[test]
    fn action_authority_and_thermodynamic_promotion_are_rejected() {
        let mut corpus = parse_corpus(fixture_value()).unwrap();
        corpus.fixtures[0].no_action_authority = false;
        assert!(validate_corpus(&corpus).is_err());

        let mut corpus = parse_corpus(fixture_value()).unwrap();
        corpus.fixtures[0].thermodynamics.status =
            ThermodynamicStatus::QualifiedWithinScope;
        assert!(validate_corpus(&corpus).is_err());
    }

    #[test]
    fn same_facts_different_premises_remains_one_factset() {
        let corpus = parse_corpus(fixture_value()).unwrap();
        let fixture = corpus
            .fixtures
            .iter()
            .find(|fixture| fixture.family == FixtureFamily::SameFactsDifferentPremises)
            .expect("same-facts fixture must exist");
        assert_eq!(fixture.fixture_id, "LW-12-same-facts-different-premises");
        let distinct_profiles = fixture
            .expected_findings
            .iter()
            .map(|finding| finding.profile_id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(distinct_profiles.len() >= 3);
        assert_eq!(
            fixture.evidence.iter().filter(|evidence| {
                evidence.dimension == EvidenceDimension::ThermodynamicObservables
            }).count(),
            0
        );
    }


    #[test]
    fn unrecorded_or_unknown_consent_never_becomes_a_contradicted_consent_finding() {
        let corpus: Corpus = serde_json::from_str(FIXTURE_JSON).unwrap();
        for fixture in &corpus.fixtures {
            let consent_is_uncertain = fixture.affected_parties.iter().any(|party| {
                matches!(
                    party.consent_status,
                    ConsentStatus::NotRecorded | ConsentStatus::Unknown
                )
            });
            if consent_is_uncertain {
                for finding in fixture.expected_findings.iter().filter(|finding| {
                    finding.premise_id == "consent_or_authority"
                }) {
                    assert_eq!(
                        finding.status,
                        FindingStatus::Undetermined,
                        "fixture {} must not infer refusal or violation from an unrecorded/unknown consent state",
                        fixture.fixture_id
                    );
                }
            }
        }
    }

    #[test]
    fn persistence_and_energy_input_do_not_substitute_for_output_or_severe_welfare_thresholds() {
        let corpus: Corpus = serde_json::from_str(FIXTURE_JSON).unwrap();
        let tumour = corpus
            .fixtures
            .iter()
            .find(|fixture| fixture.fixture_id == "LW-01-tumour-parasite")
            .unwrap();
        let persistence_finding = tumour
            .expected_findings
            .iter()
            .find(|finding| finding.premise_id == "declared_output_floor")
            .unwrap();
        assert_eq!(persistence_finding.status, FindingStatus::Undetermined);

        let throughput = corpus
            .fixtures
            .iter()
            .find(|fixture| fixture.fixture_id == "LW-03-throughput-welfare")
            .unwrap();
        let output_finding = throughput
            .expected_findings
            .iter()
            .find(|finding| finding.premise_id == "declared_output_floor")
            .unwrap();
        assert_eq!(output_finding.status, FindingStatus::Supported);
        assert_eq!(
            output_finding.evidence_refs,
            vec![String::from("e4")]
        );

        let welfare_finding = throughput
            .expected_findings
            .iter()
            .find(|finding| finding.premise_id == "avoid_preventable_severe_harm")
            .unwrap();
        assert_eq!(welfare_finding.status, FindingStatus::Undetermined);

        assert!(throughput.evidence.iter().any(|evidence| {
            evidence.evidence_id == "e1"
                && evidence.dimension == EvidenceDimension::PhysicalFeasibility
                && evidence.unit.as_deref() == Some("kWh")
        }));
    }

    #[test]
    fn all_v01_cases_remain_unqualified_and_non_authorizing() {
        let corpus = parse_corpus(fixture_value()).unwrap();
        for fixture in &corpus.fixtures {
            assert_eq!(fixture.thermodynamics.status, ThermodynamicStatus::NotQualified);
            assert!(fixture.thermodynamics.qualified_observables.is_empty());
            assert!(fixture.human_review_required);
            assert!(fixture.no_action_authority);
        }
    }
}
