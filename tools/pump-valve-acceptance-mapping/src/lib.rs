//! Product-family standard applicability classifier.
//!
//! This module checks only frozen public-scope metadata and explicitly declared
//! contract/profile fields. It does not encode normative tolerances, authenticate
//! standards documents, establish conformity, certify equipment, authorize tests,
//! define pressure-test safety, or release products.

use std::collections::{BTreeMap, BTreeSet};

pub const PROFILES: &str = include_str!("../fixtures/profiles-v0.1.tsv");
pub const SCENARIOS: &str = include_str!("../fixtures/scenarios-v0.1.tsv");
pub const EXPECTED: &str = include_str!("../fixtures/expected-v0.1.tsv");

const PROFILE_HEADER: &str = "profile_id\tstandard_id\tedition\tpublication_state\tproduct_families\ttest_families\tseat_families\tfluid_scope\tfacility_scope\trequires_product_standard\tallow_other_family_by_agreement\trequires_agreement_for_fittings\trequires_acceptance_grade\texcludes_pressure_test_safety\tsource_ref";
const SCENARIO_HEADER: &str = "scenario_id\tprofile_id\tproduct_family\ttest_family\tseat_family\tfluid_profile\tfacility_profile\tincludes_fittings\tfitting_agreement\tproduct_standard_link\tpurchaser_agreement\tacceptance_grade\tevidence_status\tclaim_kind";
const EXPECTED_HEADER: &str = "scenario_id\texpected_disposition";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StandardProfile {
    pub profile_id: String,
    pub standard_id: String,
    pub edition: String,
    pub publication_state: String,
    pub product_families: Vec<String>,
    pub test_families: Vec<String>,
    pub seat_families: Vec<String>,
    pub fluid_scope: String,
    pub facility_scope: String,
    pub requires_product_standard: bool,
    pub allow_other_family_by_agreement: bool,
    pub requires_agreement_for_fittings: bool,
    pub requires_acceptance_grade: bool,
    pub excludes_pressure_test_safety: bool,
    pub source_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scenario {
    pub scenario_id: String,
    pub profile_id: String,
    pub product_family: String,
    pub test_family: String,
    pub seat_family: String,
    pub fluid_profile: String,
    pub facility_profile: String,
    pub includes_fittings: bool,
    pub fitting_agreement: String,
    pub product_standard_link: String,
    pub purchaser_agreement: bool,
    pub acceptance_grade: String,
    pub evidence_status: String,
    pub claim_kind: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    Applicable,
    NotApplicable,
    ProfileIncomplete,
    EvidenceIncomplete,
    StaleOrSuperseded,
    UnsupportedClaim,
}

impl Disposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applicable => "APPLICABLE",
            Self::NotApplicable => "NOT_APPLICABLE",
            Self::ProfileIncomplete => "PROFILE_INCOMPLETE",
            Self::EvidenceIncomplete => "EVIDENCE_INCOMPLETE",
            Self::StaleOrSuperseded => "STALE_OR_SUPERSEDED",
            Self::UnsupportedClaim => "UNSUPPORTED_CLAIM",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "APPLICABLE" => Ok(Self::Applicable),
            "NOT_APPLICABLE" => Ok(Self::NotApplicable),
            "PROFILE_INCOMPLETE" => Ok(Self::ProfileIncomplete),
            "EVIDENCE_INCOMPLETE" => Ok(Self::EvidenceIncomplete),
            "STALE_OR_SUPERSEDED" => Ok(Self::StaleOrSuperseded),
            "UNSUPPORTED_CLAIM" => Ok(Self::UnsupportedClaim),
            _ => Err(format!("unknown expected disposition: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedRow {
    pub scenario_id: String,
    pub disposition: Disposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    pub scenario_id: String,
    pub profile_id: String,
    pub disposition: Disposition,
    pub reason: String,
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value != "-"
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || b":/_-.@".contains(&byte)
        })
}

fn parse_bool(value: &str, source: &str, line_no: usize) -> Result<bool, String> {
    match value {
        "YES" => Ok(true),
        "NO" => Ok(false),
        _ => Err(format!("{source}:{line_no}: expected YES or NO, found {value}")),
    }
}

fn split_row<'a>(
    line: &'a str,
    expected: usize,
    source: &str,
    line_no: usize,
) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != expected {
        return Err(format!(
            "{source}:{line_no}: expected {expected} fields, found {}",
            fields.len()
        ));
    }
    if fields.iter().any(|field| field.trim() != *field || field.is_empty()) {
        return Err(format!(
            "{source}:{line_no}: empty or padded fields are forbidden"
        ));
    }
    Ok(fields)
}

fn parse_list(value: &str) -> Result<Vec<String>, String> {
    let values: Vec<String> = value.split(',').map(str::to_owned).collect();
    if values.is_empty() || values.iter().any(|item| !valid_ref(item)) {
        return Err(format!("invalid comma-separated profile set: {value}"));
    }
    Ok(values)
}

pub fn parse_profiles(input: &str) -> Result<Vec<StandardProfile>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == PROFILE_HEADER => {}
        Some(_) => return Err("profiles: header mismatch".into()),
        None => return Err("profiles: empty input".into()),
    }

    let mut profiles = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 15, "profiles", index + 2)?;
        for (label, value) in [
            ("profile_id", row[0]),
            ("standard_id", row[1]),
            ("edition", row[2]),
            ("publication_state", row[3]),
            ("fluid_scope", row[7]),
            ("facility_scope", row[8]),
            ("source_ref", row[14]),
        ] {
            if !valid_ref(value) && label != "standard_id" {
                return Err(format!("profiles:{}: invalid {label}", index + 2));
            }
        }
        if row[1].trim().is_empty() || !row[14].starts_with("https://") {
            return Err(format!("profiles:{}: invalid standard/source reference", index + 2));
        }
        if !matches!(
            row[3],
            "PUBLISHED_UNDER_REVIEW"
                | "PUBLISHED_CURRENT"
                | "PUBLISHED_CONFIRMED_2025"
                | "DRAFT_UNDER_DEVELOPMENT"
                | "WITHDRAWN"
        ) {
            return Err(format!(
                "profiles:{}: unsupported publication state {}",
                index + 2,
                row[3]
            ));
        }
        if !seen.insert(row[0].to_owned()) {
            return Err(format!(
                "profiles:{}: duplicate profile_id {}",
                index + 2,
                row[0]
            ));
        }

        profiles.push(StandardProfile {
            profile_id: row[0].to_owned(),
            standard_id: row[1].to_owned(),
            edition: row[2].to_owned(),
            publication_state: row[3].to_owned(),
            product_families: parse_list(row[4])?,
            test_families: parse_list(row[5])?,
            seat_families: parse_list(row[6])?,
            fluid_scope: row[7].to_owned(),
            facility_scope: row[8].to_owned(),
            requires_product_standard: parse_bool(row[9], "profiles", index + 2)?,
            allow_other_family_by_agreement: parse_bool(row[10], "profiles", index + 2)?,
            requires_agreement_for_fittings: parse_bool(row[11], "profiles", index + 2)?,
            requires_acceptance_grade: parse_bool(row[12], "profiles", index + 2)?,
            excludes_pressure_test_safety: parse_bool(row[13], "profiles", index + 2)?,
            source_ref: row[14].to_owned(),
        });
    }
    if profiles.is_empty() {
        return Err("profiles: no profile rows".into());
    }
    Ok(profiles)
}

pub fn parse_scenarios(input: &str) -> Result<Vec<Scenario>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == SCENARIO_HEADER => {}
        Some(_) => return Err("scenarios: header mismatch".into()),
        None => return Err("scenarios: empty input".into()),
    }

    let mut scenarios = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 14, "scenarios", index + 2)?;
        for (label, value) in [
            ("scenario_id", row[0]),
            ("profile_id", row[1]),
            ("product_family", row[2]),
            ("test_family", row[3]),
            ("seat_family", row[4]),
            ("fluid_profile", row[5]),
            ("facility_profile", row[6]),
            ("product_standard_link", row[9]),
            ("acceptance_grade", row[11]),
            ("evidence_status", row[12]),
            ("claim_kind", row[13]),
        ] {
            if !valid_ref(value) && value != "-" {
                return Err(format!("scenarios:{}: invalid {label}", index + 2));
            }
        }
        if !seen.insert(row[0].to_owned()) {
            return Err(format!(
                "scenarios:{}: duplicate scenario_id {}",
                index + 2,
                row[0]
            ));
        }
        if !matches!(row[8], "YES" | "NO" | "NA") {
            return Err(format!(
                "scenarios:{}: invalid fitting_agreement {}",
                index + 2,
                row[8]
            ));
        }
        if !matches!(
            row[9],
            "REFERENCED" | "NOT_REFERENCED" | "NOT_REQUIRED"
        ) {
            return Err(format!(
                "scenarios:{}: invalid product_standard_link {}",
                index + 2,
                row[9]
            ));
        }
        if !matches!(
            row[12],
            "COMPLETE" | "MISSING_CALIBRATION" | "MISSING_TEST_RECORD" | "STALE_EVIDENCE"
        ) {
            return Err(format!(
                "scenarios:{}: unsupported evidence_status {}",
                index + 2,
                row[12]
            ));
        }

        scenarios.push(Scenario {
            scenario_id: row[0].to_owned(),
            profile_id: row[1].to_owned(),
            product_family: row[2].to_owned(),
            test_family: row[3].to_owned(),
            seat_family: row[4].to_owned(),
            fluid_profile: row[5].to_owned(),
            facility_profile: row[6].to_owned(),
            includes_fittings: parse_bool(row[7], "scenarios", index + 2)?,
            fitting_agreement: row[8].to_owned(),
            product_standard_link: row[9].to_owned(),
            purchaser_agreement: parse_bool(row[10], "scenarios", index + 2)?,
            acceptance_grade: row[11].to_owned(),
            evidence_status: row[12].to_owned(),
            claim_kind: row[13].to_owned(),
        });
    }
    if scenarios.is_empty() {
        return Err("scenarios: no scenario rows".into());
    }
    Ok(scenarios)
}

pub fn parse_expected(input: &str) -> Result<Vec<ExpectedRow>, String> {
    let mut lines = input.lines();
    match lines.next() {
        Some(header) if header == EXPECTED_HEADER => {}
        Some(_) => return Err("expected: header mismatch".into()),
        None => return Err("expected: empty input".into()),
    }

    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let row = split_row(line, 2, "expected", index + 2)?;
        if !valid_ref(row[0]) || !seen.insert(row[0].to_owned()) {
            return Err(format!("expected:{}: invalid or duplicate scenario id", index + 2));
        }
        rows.push(ExpectedRow {
            scenario_id: row[0].to_owned(),
            disposition: Disposition::parse(row[1])?,
        });
    }
    Ok(rows)
}

pub fn evaluate(
    profiles: &[StandardProfile],
    scenarios: &[Scenario],
) -> Result<Vec<Evaluation>, String> {
    if profiles.is_empty() || scenarios.is_empty() {
        return Err("profiles and scenarios must both be non-empty".into());
    }

    let mut profiles_by_id = BTreeMap::new();
    for profile in profiles {
        if profiles_by_id.insert(profile.profile_id.as_str(), profile).is_some() {
            return Err(format!("duplicate profile_id: {}", profile.profile_id));
        }
    }

    let mut seen_scenarios = BTreeSet::new();
    let mut results = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        if !seen_scenarios.insert(scenario.scenario_id.as_str()) {
            return Err(format!("duplicate scenario_id: {}", scenario.scenario_id));
        }
        let profile = profiles_by_id
            .get(scenario.profile_id.as_str())
            .ok_or_else(|| format!("unknown profile_id: {}", scenario.profile_id))?;

        let (disposition, reason) = evaluate_one(profile, scenario);
        results.push(Evaluation {
            scenario_id: scenario.scenario_id.clone(),
            profile_id: scenario.profile_id.clone(),
            disposition,
            reason: reason.to_owned(),
        });
    }
    Ok(results)
}

fn evaluate_one(profile: &StandardProfile, scenario: &Scenario) -> (Disposition, &'static str) {
    if matches!(
        profile.publication_state.as_str(),
        "DRAFT_UNDER_DEVELOPMENT" | "WITHDRAWN"
    ) {
        return (
            Disposition::StaleOrSuperseded,
            "selected standard profile is not a published usable edition",
        );
    }

    if scenario.claim_kind == "PRESSURE_TEST_SAFETY" {
        return (
            Disposition::UnsupportedClaim,
            if profile.excludes_pressure_test_safety {
                "the selected standard profile explicitly excludes pressure-test safety"
            } else {
                "the selected test standard does not qualify a pressure-test safety case"
            },
        );
    }
    if scenario.claim_kind != "TEST_APPLICABILITY" {
        return (
            Disposition::UnsupportedClaim,
            "applicability metadata does not establish product release or safety",
        );
    }
    if scenario.evidence_status != "COMPLETE" {
        return (
            Disposition::EvidenceIncomplete,
            "required evidence is missing or stale",
        );
    }

    let family_is_listed = profile
        .product_families
        .iter()
        .any(|family| family == &scenario.product_family);
    if !family_is_listed
        && !(profile.allow_other_family_by_agreement && scenario.purchaser_agreement)
    {
        return (
            Disposition::NotApplicable,
            "product family is outside the declared standard profile scope",
        );
    }

    if !profile
        .test_families
        .iter()
        .any(|family| family == &scenario.test_family)
    {
        return (
            Disposition::NotApplicable,
            "test family is outside the declared standard profile scope",
        );
    }

    if scenario.seat_family == "UNKNOWN" || scenario.seat_family == "-" {
        return (
            Disposition::ProfileIncomplete,
            "valve seat family must be explicitly declared",
        );
    }
    if !profile.seat_families.iter().any(|seat| seat == "ANY_DECLARED")
        && !profile
            .seat_families
            .iter()
            .any(|seat| seat == &scenario.seat_family)
    {
        return (
            Disposition::NotApplicable,
            "seat family is outside the declared standard profile scope",
        );
    }

    if profile.fluid_scope == "CLEAN_COLD_WATER"
        && scenario.fluid_profile != "CLEAN_COLD_WATER"
    {
        return (
            Disposition::NotApplicable,
            "fluid is outside the ISO 9906 clean/cold-water behavior profile",
        );
    }
    if profile.fluid_scope == "CLEAR_WATER" && scenario.fluid_profile != "CLEAR_WATER" {
        return (
            Disposition::NotApplicable,
            "fluid is outside the ANSI/HI 14.6 clear-water profile",
        );
    }
    if profile.fluid_scope == "ANY_DECLARED" && scenario.fluid_profile == "UNKNOWN" {
        return (
            Disposition::ProfileIncomplete,
            "fluid/service profile must be explicitly declared",
        );
    }

    match profile.facility_scope.as_str() {
        "FACTORY_LAB" if scenario.facility_profile != "FACTORY_LAB" => {
            return (
                Disposition::NotApplicable,
                "profile is scoped to a manufacturer or laboratory test facility",
            );
        }
        "FACTORY_LAB_OR_AGREED_FIELD" => {
            if scenario.facility_profile == "FIELD" && !scenario.purchaser_agreement {
                return (
                    Disposition::ProfileIncomplete,
                    "field test requires an explicit purchaser/manufacturer agreement",
                );
            }
            if !matches!(scenario.facility_profile.as_str(), "FACTORY_LAB" | "FIELD") {
                return (
                    Disposition::ProfileIncomplete,
                    "facility type is unknown or unsupported",
                );
            }
        }
        "QUALIFIED_TEST_FACILITY" if scenario.facility_profile != "QUALIFIED_TEST_FACILITY" => {
            return (
                Disposition::ProfileIncomplete,
                "a qualified valve test facility has not been declared",
            );
        }
        _ => {}
    }

    if profile.requires_product_standard
        && scenario.product_standard_link != "REFERENCED"
    {
        return (
            Disposition::ProfileIncomplete,
            "the required governing valve product standard is not linked",
        );
    }
    if profile.requires_agreement_for_fittings
        && scenario.includes_fittings
        && scenario.fitting_agreement != "YES"
    {
        return (
            Disposition::ProfileIncomplete,
            "testing with fittings requires prior agreement and an exact test boundary",
        );
    }
    if scenario.includes_fittings && scenario.fitting_agreement != "YES" {
        return (
            Disposition::ProfileIncomplete,
            "the pump-plus-fittings test boundary is not explicitly agreed",
        );
    }
    if profile.requires_acceptance_grade && scenario.acceptance_grade == "-" {
        return (
            Disposition::ProfileIncomplete,
            "purchaser-selected acceptance grade is missing",
        );
    }
    if profile.requires_acceptance_grade && scenario.acceptance_grade == "UNKNOWN" {
        return (
            Disposition::ProfileIncomplete,
            "acceptance grade is unknown",
        );
    }
    if profile.profile_id == "api-598-2023"
        && !family_is_listed
        && !scenario.purchaser_agreement
    {
        return (
            Disposition::NotApplicable,
            "API 598 does not cover this valve type without purchaser/manufacturer agreement",
        );
    }
    if profile.profile_id == "api-598-2023"
        && scenario.product_standard_link != "REFERENCED"
        && !scenario.purchaser_agreement
    {
        return (
            Disposition::ProfileIncomplete,
            "API 598 use lacks a normative reference or purchaser/manufacturer agreement",
        );
    }

    (
        Disposition::Applicable,
        "declared product, test, fluid, facility, contract and evidence fields fit the frozen profile",
    )
}

pub fn verify_expected(
    actual: &[Evaluation],
    expected: &[ExpectedRow],
) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "expected {} evaluations, got {}",
            expected.len(),
            actual.len()
        ));
    }
    let actual_by_id: BTreeMap<&str, &Evaluation> = actual
        .iter()
        .map(|row| (row.scenario_id.as_str(), row))
        .collect();
    for expected_row in expected {
        let actual_row = actual_by_id
            .get(expected_row.scenario_id.as_str())
            .ok_or_else(|| format!("missing actual scenario: {}", expected_row.scenario_id))?;
        if actual_row.disposition != expected_row.disposition {
            return Err(format!(
                "oracle mismatch for {}: expected {}, got {} ({})",
                expected_row.scenario_id,
                expected_row.disposition.as_str(),
                actual_row.disposition.as_str(),
                actual_row.reason
            ));
        }
    }
    Ok(())
}

pub fn evaluate_corpus(
    profiles_tsv: &str,
    scenarios_tsv: &str,
    expected_tsv: &str,
) -> Result<Vec<Evaluation>, String> {
    let profiles = parse_profiles(profiles_tsv)?;
    let scenarios = parse_scenarios(scenarios_tsv)?;
    let expected = parse_expected(expected_tsv)?;
    let results = evaluate(&profiles, &scenarios)?;
    verify_expected(&results, &expected)?;
    Ok(results)
}

pub fn render_report(results: &[Evaluation]) -> String {
    let mut report = String::from("scenario_id\tprofile_id\tdisposition\treason\n");
    for row in results {
        report.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            row.scenario_id,
            row.profile_id,
            row.disposition.as_str(),
            row.reason
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed() -> (Vec<StandardProfile>, Vec<Scenario>) {
        (
            parse_profiles(PROFILES).unwrap(),
            parse_scenarios(SCENARIOS).unwrap(),
        )
    }

    #[test]
    fn frozen_twenty_seven_case_oracle_matches() {
        let result = evaluate_corpus(PROFILES, SCENARIOS, EXPECTED).unwrap();
        assert_eq!(result.len(), 30);
        assert_eq!(
            result
                .iter()
                .filter(|row| row.disposition == Disposition::Applicable)
                .count(),
            11
        );
    }

    #[test]
    fn iso_9906_published_under_review_is_not_stale() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "pump_iso9906_valid")
            .unwrap();
        assert_eq!(row.disposition, Disposition::Applicable);
    }

    #[test]
    fn positive_displacement_pump_is_not_in_iso_9906_scope() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "pump_iso9906_positive_displacement")
            .unwrap();
        assert_eq!(row.disposition, Disposition::NotApplicable);
    }

    #[test]
    fn clear_water_pump_profile_does_not_cover_abrasive_slurry() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "pump_iso9906_abrasive_slurry")
            .unwrap();
        assert_eq!(row.disposition, Disposition::NotApplicable);
    }

    #[test]
    fn pump_fittings_require_explicit_boundary() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "pump_hi146_fittings_no_agreement")
            .unwrap();
        assert_eq!(row.disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn acceptance_grade_is_not_invented() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "pump_iso9906_missing_grade")
            .unwrap();
        assert_eq!(row.disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn submersible_mechanical_and_electrical_tests_have_a_separate_profile() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "submersible_hi116_integrity")
            .unwrap();
        assert_eq!(row.disposition, Disposition::Applicable);
    }

    #[test]
    fn submersible_hydraulic_test_is_not_claimed_under_hi_11_6() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "submersible_hi116_hydraulic_misuse")
            .unwrap();
        assert_eq!(row.disposition, Disposition::NotApplicable);
    }

    #[test]
    fn agreed_field_submersible_integrity_test_is_profile_applicable() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "submersible_hi116_field_agreed")
            .unwrap();
        assert_eq!(row.disposition, Disposition::Applicable);
    }

    #[test]
    fn unqualified_valve_test_facility_does_not_qualify() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_unqualified_facility")
            .unwrap();
        assert_eq!(row.disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn iso_5208_requires_governing_valve_product_standard() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_missing_product_standard")
            .unwrap();
        assert_eq!(row.disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn iso_5208_does_not_support_pressure_test_safety_claims() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_pressure_safety_claim")
            .unwrap();
        assert_eq!(row.disposition, Disposition::UnsupportedClaim);
    }

    #[test]
    fn draft_amendment_cannot_be_used_as_published_requirement() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_draft_amendment")
            .unwrap();
        assert_eq!(row.disposition, Disposition::StaleOrSuperseded);
    }

    #[test]
    fn api_598_pressure_test_does_not_establish_test_safety() {
        let (profiles, mut scenarios) = parsed();
        let row = scenarios
            .iter_mut()
            .find(|row| row.scenario_id == "valve_api598_gate_referenced")
            .unwrap();
        row.claim_kind = "PRESSURE_TEST_SAFETY".into();
        let result = evaluate(&profiles, &scenarios).unwrap();
        let evaluated = result
            .iter()
            .find(|row| row.scenario_id == "valve_api598_gate_referenced")
            .unwrap();
        assert_eq!(evaluated.disposition, Disposition::UnsupportedClaim);
    }

    #[test]
    fn api_598_other_valve_type_requires_agreement() {
        let (profiles, scenarios) = parsed();
        let result = evaluate(&profiles, &scenarios).unwrap();
        let agreed = result
            .iter()
            .find(|row| row.scenario_id == "valve_api598_other_type_agreed")
            .unwrap();
        let not_agreed = result
            .iter()
            .find(|row| row.scenario_id == "valve_api598_other_type_no_agreement")
            .unwrap();
        assert_eq!(agreed.disposition, Disposition::Applicable);
        assert_eq!(not_agreed.disposition, Disposition::NotApplicable);
    }

    #[test]
    fn unknown_api_598_seat_family_is_incomplete() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_api598_unknown_seat_family")
            .unwrap();
        assert_eq!(row.disposition, Disposition::ProfileIncomplete);
    }

    #[test]
    fn noncovered_api_598_seat_family_is_not_applicable() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_api598_noncovered_seat_family")
            .unwrap();
        assert_eq!(row.disposition, Disposition::NotApplicable);
    }

    #[test]
    fn missing_calibration_does_not_become_applicable() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_evidence_incomplete")
            .unwrap();
        assert_eq!(row.disposition, Disposition::EvidenceIncomplete);
    }

    #[test]
    fn test_applicability_is_not_product_release() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_product_release_claim")
            .unwrap();
        assert_eq!(row.disposition, Disposition::UnsupportedClaim);
    }

    #[test]
    fn nonmetallic_valve_is_not_in_metallic_iso_5208_scope() {
        let (profiles, scenarios) = parsed();
        let row = evaluate(&profiles, &scenarios)
            .unwrap()
            .into_iter()
            .find(|row| row.scenario_id == "valve_iso5208_nonmetallic")
            .unwrap();
        assert_eq!(row.disposition, Disposition::NotApplicable);
    }

    #[test]
    fn profile_and_scenario_order_do_not_change_dispositions() {
        let (mut profiles, mut scenarios) = parsed();
        let forward = evaluate(&profiles, &scenarios).unwrap();
        profiles.reverse();
        scenarios.reverse();
        let reverse = evaluate(&profiles, &scenarios).unwrap();
        let mut forward_pairs: Vec<_> = forward
            .iter()
            .map(|row| (row.scenario_id.as_str(), row.disposition))
            .collect();
        let mut reverse_pairs: Vec<_> = reverse
            .iter()
            .map(|row| (row.scenario_id.as_str(), row.disposition))
            .collect();
        forward_pairs.sort();
        reverse_pairs.sort();
        assert_eq!(forward_pairs, reverse_pairs);
    }
}
