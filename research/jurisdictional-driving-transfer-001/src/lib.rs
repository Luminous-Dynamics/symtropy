//! Source-qualified jurisdiction profiles and left/right convention fixtures.
//!
//! This crate validates profile structure and metamorphic fixture consistency.
//! It does not decide whether a traffic rule is legally correct, verify source
//! contents against their URLs, infer vehicle configuration or control a car.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VehicleHandedness {
    LeftHandDrive,
    RightHandDrive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundaboutCirculation {
    Clockwise,
    Counterclockwise,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    Draft,
    Reviewed,
    PolicyEligible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    GovernmentGuidance,
    Statute,
    TrafficSignManual,
    Dataset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    NotReviewed,
    CrossChecked,
    LegalReviewed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSourceEvidence {
    pub source_id: String,
    pub authority: String,
    pub title: String,
    pub url: String,
    pub source_kind: SourceKind,
    pub edition_or_revision: Option<String>,
    pub published_date: Option<String>,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub content_sha256: Option<String>,
    pub verification_status: VerificationStatus,
    pub supported_rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JurisdictionProfile {
    pub schema_version: u32,
    pub profile_id: String,
    /// ISO 3166-1 alpha-2 code. The validator checks syntax, not political recognition.
    pub jurisdiction_code: String,
    pub profile_status: ProfileStatus,
    pub traffic_side: TrafficSide,
    /// Independent of traffic side: imported/converted vehicles are possible.
    pub steering_wheel_side: VehicleHandedness,
    /// Explicit and independently sourced; never inferred from traffic_side.
    pub roundabout_circulation: RoundaboutCirculation,
    pub sign_standard: String,
    pub effective_from: Option<String>,
    pub effective_to: Option<String>,
    pub rule_bundle_sha256: Option<String>,
    pub sources: Vec<RuleSourceEvidence>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Maneuver {
    LeftTurn,
    RightTurn,
    Straight,
    UTurn,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LateralSceneGeometry {
    pub ego_lateral_mm: i64,
    pub lane_center_offsets_mm: Vec<i64>,
    pub actor_lateral_offsets_mm: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConventionMetamorphicFixture {
    pub schema_version: u32,
    pub fixture_id: String,
    pub base_jurisdiction: String,
    pub alternate_jurisdiction: String,
    pub base_traffic_side: TrafficSide,
    pub alternate_traffic_side: TrafficSide,
    pub task_intent_id: String,
    pub base_task_intent_id: String,
    pub alternate_task_intent_id: String,
    pub base_rule_bundle_sha256: String,
    pub alternate_rule_bundle_sha256: String,
    pub base_maneuver: Maneuver,
    pub alternate_maneuver: Maneuver,
    pub base_geometry: LateralSceneGeometry,
    pub alternate_geometry: LateralSceneGeometry,
    pub transformation: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileAuditReport {
    pub schema_version: u32,
    pub profile_id: String,
    pub jurisdiction_code: String,
    pub disposition: String,
    pub policy_eligible: bool,
    pub traffic_side: TrafficSide,
    pub steering_wheel_side: VehicleHandedness,
    pub roundabout_circulation: RoundaboutCirculation,
    pub source_count: usize,
    pub legal_reviewed_source_count: usize,
    pub rule_bundle_digest_present: bool,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MetamorphicAuditReport {
    pub schema_version: u32,
    pub fixture_id: String,
    pub disposition: String,
    pub same_task_intent: bool,
    pub traffic_side_changed: bool,
    pub maneuver_mirrored: bool,
    pub geometry_mirrored: bool,
    pub rule_bundle_changed: bool,
    pub findings: Vec<String>,
    pub claim_ceiling: String,
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_iso_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
    {
        return false;
    }
    let Ok(year) = parts[0].parse::<u32>() else {
        return false;
    };
    let Ok(month) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return false;
    };
    if year < 1900 || !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    day <= max_day
}

fn validate_date_pair(
    start: Option<&str>,
    end: Option<&str>,
    label: &str,
    findings: &mut BTreeSet<String>,
) {
    for (kind, date) in [("start", start), ("end", end)] {
        if date.is_some_and(|value| !valid_iso_date(value)) {
            findings.insert(format!("{label}_{kind}_date_invalid"));
        }
    }
    if let (Some(start), Some(end)) = (start, end) {
        if valid_iso_date(start) && valid_iso_date(end) && start > end {
            findings.insert(format!("{label}_date_range_inverted"));
        }
    }
}

fn unique_non_empty(values: &[String]) -> bool {
    if values.is_empty() {
        return true;
    }
    if values.iter().any(|value| value.trim().is_empty()) {
        return false;
    }
    let unique = values.iter().collect::<BTreeSet<_>>();
    unique.len() == values.len()
}

pub fn audit_profile(profile: &JurisdictionProfile) -> ProfileAuditReport {
    let mut findings = BTreeSet::new();
    if profile.schema_version != 1 {
        findings.insert("schema_version_mismatch".to_string());
    }
    if profile.profile_id.trim().is_empty() {
        findings.insert("profile_id_empty".to_string());
    }
    if profile.jurisdiction_code.len() != 2
        || !profile
            .jurisdiction_code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase())
    {
        findings.insert("jurisdiction_code_invalid".to_string());
    }
    if profile.sign_standard.trim().is_empty() {
        findings.insert("sign_standard_empty".to_string());
    }
    if profile.sources.is_empty() {
        findings.insert("sources_missing".to_string());
    }
    validate_date_pair(
        profile.effective_from.as_deref(),
        profile.effective_to.as_deref(),
        "profile",
        &mut findings,
    );

    let mut source_ids = BTreeSet::new();
    let mut has_legal_statute = false;
    let mut has_legal_sign_manual = false;
    let mut cross_checked_source_count = 0_usize;
    let mut legal_reviewed_source_count = 0_usize;

    for source in &profile.sources {
        if source.source_id.trim().is_empty()
            || source.authority.trim().is_empty()
            || source.title.trim().is_empty()
        {
            findings.insert("source_identity_incomplete".to_string());
        }
        if !source_ids.insert(source.source_id.as_str()) {
            findings.insert("duplicate_source_id".to_string());
        }
        if !source.url.starts_with("https://") {
            findings.insert("source_url_must_use_https".to_string());
        }
        if source
            .content_sha256
            .as_deref()
            .is_some_and(|digest| !is_sha256_hex(digest))
        {
            findings.insert("source_content_digest_invalid".to_string());
        }
        validate_date_pair(
            source.effective_from.as_deref(),
            source.effective_to.as_deref(),
            "source_effective",
            &mut findings,
        );
        if source
            .published_date
            .as_deref()
            .is_some_and(|date| !valid_iso_date(date))
        {
            findings.insert("source_published_date_invalid".to_string());
        }
        if source.verification_status == VerificationStatus::CrossChecked {
            cross_checked_source_count += 1;
            if source.edition_or_revision.as_deref().is_none_or(str::is_empty)
                || source.content_sha256.is_none()
                || source.content_sha256.as_deref().is_some_and(|digest| !is_sha256_hex(digest))
            {
                findings.insert("cross_checked_source_lacks_revision_or_digest".to_string());
            }
        }
        if source.verification_status == VerificationStatus::LegalReviewed {
            legal_reviewed_source_count += 1;
            if source.edition_or_revision.as_deref().is_none_or(str::is_empty)
                || source.effective_from.as_deref().is_none_or(|date| !valid_iso_date(date))
                || source.content_sha256.is_none()
                || source.content_sha256.as_deref().is_some_and(|digest| !is_sha256_hex(digest))
            {
                findings.insert("legal_reviewed_source_lacks_revision_date_or_digest".to_string());
            }
            match source.source_kind {
                SourceKind::Statute => has_legal_statute = true,
                SourceKind::TrafficSignManual => has_legal_sign_manual = true,
                SourceKind::GovernmentGuidance | SourceKind::Dataset => {}
            }
        }
        if !unique_non_empty(&source.supported_rule_ids) {
            findings.insert("source_rule_ids_empty_or_duplicate".to_string());
        }
    }

    let rule_bundle_digest_present = profile
        .rule_bundle_sha256
        .as_deref()
        .is_some_and(is_sha256_hex);
    if profile.rule_bundle_sha256.is_some() && !rule_bundle_digest_present {
        findings.insert("rule_bundle_digest_invalid".to_string());
    }

    if profile.profile_status == ProfileStatus::Reviewed
        && cross_checked_source_count + legal_reviewed_source_count == 0
    {
        findings.insert("reviewed_profile_requires_cross_checked_source".to_string());
    }

    if profile.profile_status == ProfileStatus::PolicyEligible {
        if !rule_bundle_digest_present {
            findings.insert("policy_eligible_profile_requires_rule_bundle_digest".to_string());
        }
        if profile.effective_from.as_deref().is_none_or(|date| !valid_iso_date(date)) {
            findings.insert("policy_eligible_profile_requires_effective_date".to_string());
        }
        if !has_legal_statute {
            findings.insert("policy_eligible_profile_requires_legal_reviewed_statute".to_string());
        }
        if !has_legal_sign_manual {
            findings.insert("policy_eligible_profile_requires_legal_reviewed_sign_manual".to_string());
        }
        if legal_reviewed_source_count < 2 {
            findings.insert("policy_eligible_profile_requires_multiple_legal_sources".to_string());
        }
    }

    let policy_eligible = profile.profile_status == ProfileStatus::PolicyEligible
        && findings.is_empty()
        && rule_bundle_digest_present;
    let disposition = if findings.is_empty() {
        match profile.profile_status {
            ProfileStatus::Draft => "valid_draft",
            ProfileStatus::Reviewed => "valid_reviewed_not_policy_eligible",
            ProfileStatus::PolicyEligible => "policy_eligible",
        }
    } else {
        "rejected"
    };

    ProfileAuditReport {
        schema_version: 1,
        profile_id: profile.profile_id.clone(),
        jurisdiction_code: profile.jurisdiction_code.clone(),
        disposition: disposition.to_string(),
        policy_eligible,
        traffic_side: profile.traffic_side,
        steering_wheel_side: profile.steering_wheel_side,
        roundabout_circulation: profile.roundabout_circulation,
        source_count: profile.sources.len(),
        legal_reviewed_source_count,
        rule_bundle_digest_present,
        findings: findings.into_iter().collect(),
        claim_ceiling: "Structural source-manifest consistency only. The auditor does not fetch or authenticate URLs, verify legal interpretation, infer traffic law, or establish safe driving behaviour.".to_string(),
    }
}

pub fn mirror_lateral_coordinate(value_mm: i64) -> Result<i64, String> {
    value_mm
        .checked_neg()
        .ok_or_else(|| "cannot mirror i64::MIN lateral coordinate".to_string())
}

fn mirror_maneuver(maneuver: Maneuver) -> Maneuver {
    match maneuver {
        Maneuver::LeftTurn => Maneuver::RightTurn,
        Maneuver::RightTurn => Maneuver::LeftTurn,
        Maneuver::Straight => Maneuver::Straight,
        Maneuver::UTurn => Maneuver::UTurn,
    }
}

fn geometry_mirrors(
    base: &LateralSceneGeometry,
    alternate: &LateralSceneGeometry,
) -> bool {
    if base.lane_center_offsets_mm.len() != alternate.lane_center_offsets_mm.len()
        || base.actor_lateral_offsets_mm.len() != alternate.actor_lateral_offsets_mm.len()
    {
        return false;
    }
    if mirror_lateral_coordinate(base.ego_lateral_mm).ok() != Some(alternate.ego_lateral_mm) {
        return false;
    }
    base.lane_center_offsets_mm
        .iter()
        .zip(&alternate.lane_center_offsets_mm)
        .all(|(base, alternate)| mirror_lateral_coordinate(*base).ok() == Some(*alternate))
        && base
            .actor_lateral_offsets_mm
            .iter()
            .zip(&alternate.actor_lateral_offsets_mm)
            .all(|(base, alternate)| {
                mirror_lateral_coordinate(*base).ok() == Some(*alternate)
            })
}

pub fn audit_metamorphic_fixture(
    fixture: &ConventionMetamorphicFixture,
) -> MetamorphicAuditReport {
    let mut findings = BTreeSet::new();
    if fixture.schema_version != 1 {
        findings.insert("schema_version_mismatch".to_string());
    }
    if fixture.fixture_id.trim().is_empty()
        || fixture.base_jurisdiction.trim().is_empty()
        || fixture.alternate_jurisdiction.trim().is_empty()
        || fixture.task_intent_id.trim().is_empty()
    {
        findings.insert("fixture_identity_incomplete".to_string());
    }
    if fixture.base_jurisdiction == fixture.alternate_jurisdiction {
        findings.insert("jurisdictions_must_differ".to_string());
    }
    let traffic_side_changed = fixture.base_traffic_side != fixture.alternate_traffic_side;
    if !traffic_side_changed {
        findings.insert("traffic_side_not_changed".to_string());
    }
    let same_task_intent = fixture.task_intent_id == fixture.base_task_intent_id
        && fixture.task_intent_id == fixture.alternate_task_intent_id;
    if !same_task_intent {
        findings.insert("task_intent_not_preserved".to_string());
    }
    let rule_bundle_changed = is_sha256_hex(&fixture.base_rule_bundle_sha256)
        && is_sha256_hex(&fixture.alternate_rule_bundle_sha256)
        && fixture.base_rule_bundle_sha256 != fixture.alternate_rule_bundle_sha256;
    if !rule_bundle_changed {
        findings.insert("rule_bundle_digests_must_be_valid_and_distinct".to_string());
    }
    let maneuver_mirrored = mirror_maneuver(fixture.base_maneuver)
        == fixture.alternate_maneuver;
    if !maneuver_mirrored {
        findings.insert("maneuver_not_mirrored".to_string());
    }
    let geometry_mirrored =
        geometry_mirrors(&fixture.base_geometry, &fixture.alternate_geometry);
    if !geometry_mirrored {
        findings.insert("lateral_geometry_not_mirrored".to_string());
    }
    if fixture.transformation != "mirror_about_road_centerline" {
        findings.insert("unsupported_metamorphic_transformation".to_string());
    }

    MetamorphicAuditReport {
        schema_version: 1,
        fixture_id: fixture.fixture_id.clone(),
        disposition: if findings.is_empty() { "valid" } else { "rejected" }.to_string(),
        same_task_intent,
        traffic_side_changed,
        maneuver_mirrored,
        geometry_mirrored,
        rule_bundle_changed,
        findings: findings.into_iter().collect(),
        claim_ceiling: "Deterministic lateral-geometry metamorphic consistency only. This does not transform or validate local law, traffic-sign semantics, actor policy, physical dynamics, or driving performance.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn south_africa_draft() -> JurisdictionProfile {
        JurisdictionProfile {
            schema_version: 1,
            profile_id: "ZA-source-skeleton-v0-1".to_string(),
            jurisdiction_code: "ZA".to_string(),
            profile_status: ProfileStatus::Draft,
            traffic_side: TrafficSide::Left,
            steering_wheel_side: VehicleHandedness::Unknown,
            roundabout_circulation: RoundaboutCirculation::Unknown,
            sign_standard: "SADC/SA Road Traffic Signs Manual — exact applicable edition pending review".to_string(),
            effective_from: None,
            effective_to: None,
            rule_bundle_sha256: None,
            sources: vec![
                RuleSourceEvidence {
                    source_id: "za-government-left-traffic-guidance".to_string(),
                    authority: "South African Department of Tourism".to_string(),
                    title: "Safety and Travel Tips — Exploring Mzansi by Road".to_string(),
                    url: "https://www.tourism.gov.za/Pages/Safety--Travel-Tips-Info.aspx".to_string(),
                    source_kind: SourceKind::GovernmentGuidance,
                    edition_or_revision: None,
                    published_date: None,
                    effective_from: None,
                    effective_to: None,
                    content_sha256: None,
                    verification_status: VerificationStatus::NotReviewed,
                    supported_rule_ids: vec!["traffic-side-left".to_string()],
                },
                RuleSourceEvidence {
                    source_id: "za-traffic-signs-manual-v4c1".to_string(),
                    authority: "South African Department of Transport".to_string(),
                    title: "SADC Road Traffic Signs Manual, Volume 4, Chapter 1".to_string(),
                    url: "https://www.transport.gov.za/wp-content/uploads/2023/02/V4C1.pdf".to_string(),
                    source_kind: SourceKind::TrafficSignManual,
                    edition_or_revision: Some("Third Edition; exact local adoption/revision pending review".to_string()),
                    published_date: None,
                    effective_from: None,
                    effective_to: None,
                    content_sha256: None,
                    verification_status: VerificationStatus::NotReviewed,
                    supported_rule_ids: vec!["road-sign-standard-reference".to_string()],
                },
                RuleSourceEvidence {
                    source_id: "za-national-road-traffic-act-1996".to_string(),
                    authority: "South African Government".to_string(),
                    title: "National Road Traffic Act 93 of 1996 — current consolidated applicability pending review".to_string(),
                    url: "https://www.gov.za/documents/national-road-traffic-act".to_string(),
                    source_kind: SourceKind::Statute,
                    edition_or_revision: Some("Act 93 of 1996; amendments and commencement status require legal review".to_string()),
                    published_date: None,
                    effective_from: None,
                    effective_to: None,
                    content_sha256: None,
                    verification_status: VerificationStatus::NotReviewed,
                    supported_rule_ids: vec![],
                }
            ],
            notes: vec![
                "Draft source skeleton only; not policy eligible.".to_string(),
                "Left-hand traffic is supported by the cited government guidance; this is not a detailed maneuver rule pack.".to_string(),
                "The official traffic-sign manual and National Road Traffic Act source pages require exact revision, effective-date, digest and legal applicability review before policy use.".to_string(),
                "Vehicle steering-wheel position and roundabout circulation are explicitly unknown; do not infer either from traffic side.".to_string()
            ],
        }
    }

    fn mirrored_fixture() -> ConventionMetamorphicFixture {
        ConventionMetamorphicFixture {
            schema_version: 1,
            fixture_id: "ZA-GB-left-right-road-mirror-v0-1".to_string(),
            base_jurisdiction: "ZA".to_string(),
            alternate_jurisdiction: "US".to_string(),
            base_traffic_side: TrafficSide::Left,
            alternate_traffic_side: TrafficSide::Right,
            task_intent_id: "approach-junction-and-take-corresponding-mirrored-exit-v1".to_string(),
            base_task_intent_id: "approach-junction-and-take-corresponding-mirrored-exit-v1".to_string(),
            alternate_task_intent_id: "approach-junction-and-take-corresponding-mirrored-exit-v1".to_string(),
            base_rule_bundle_sha256: "a".repeat(64),
            alternate_rule_bundle_sha256: "b".repeat(64),
            base_maneuver: Maneuver::LeftTurn,
            alternate_maneuver: Maneuver::RightTurn,
            base_geometry: LateralSceneGeometry {
                ego_lateral_mm: 1_000,
                lane_center_offsets_mm: vec![1_750, 5_250],
                actor_lateral_offsets_mm: vec![-900, 2_100],
            },
            alternate_geometry: LateralSceneGeometry {
                ego_lateral_mm: -1_000,
                lane_center_offsets_mm: vec![-1_750, -5_250],
                actor_lateral_offsets_mm: vec![900, -2_100],
            },
            transformation: "mirror_about_road_centerline".to_string(),
            note: "Geometry/intent fixture only; local sign/rule bundles are independently versioned.".to_string(),
        }
    }

    #[test]
    fn validates_gregorian_dates_and_leap_years() {
        assert!(valid_iso_date("2024-02-29"));
        assert!(valid_iso_date("2026-11-11"));
        assert!(!valid_iso_date("2025-02-29"));
        assert!(!valid_iso_date("2026-04-31"));
        assert!(!valid_iso_date("2026-13-01"));
        assert!(!valid_iso_date("2026-00-01"));
        assert!(!valid_iso_date("2026-11-00"));
    }

    #[test]
    fn south_africa_profile_is_valid_draft_not_policy_eligible() {
        let report = audit_profile(&south_africa_draft());
        assert_eq!(report.disposition, "valid_draft");
        assert!(!report.policy_eligible);
        assert_eq!(report.traffic_side, TrafficSide::Left);
        assert_eq!(report.steering_wheel_side, VehicleHandedness::Unknown);
        assert_eq!(report.legal_reviewed_source_count, 0);
    }

    #[test]
    fn left_hand_traffic_does_not_infer_steering_wheel_position() {
        let mut profile = south_africa_draft();
        profile.steering_wheel_side = VehicleHandedness::LeftHandDrive;
        assert_eq!(audit_profile(&profile).disposition, "valid_draft");
        profile.steering_wheel_side = VehicleHandedness::Unknown;
        assert_eq!(audit_profile(&profile).disposition, "valid_draft");
    }

    #[test]
    fn reviewed_profile_requires_at_least_one_cross_checked_source() {
        let mut profile = south_africa_draft();
        profile.profile_status = ProfileStatus::Reviewed;
        let report = audit_profile(&profile);
        assert!(!report.policy_eligible);
        assert!(report.findings.contains(
            &"reviewed_profile_requires_cross_checked_source".to_string()
        ));
        profile.sources[0].verification_status = VerificationStatus::CrossChecked;
        profile.sources[0].edition_or_revision = Some("government-guidance-current-page".into());
        profile.sources[0].content_sha256 = Some("e".repeat(64));
        assert_eq!(audit_profile(&profile).disposition, "valid_reviewed_not_policy_eligible");
    }

    #[test]
    fn policy_eligible_profile_requires_legal_reviewed_statute_and_sign_manual() {
        let mut profile = south_africa_draft();
        profile.profile_status = ProfileStatus::PolicyEligible;
        profile.effective_from = Some("2026-01-01".to_string());
        profile.rule_bundle_sha256 = Some("c".repeat(64));
        let report = audit_profile(&profile);
        assert!(!report.policy_eligible);
        assert!(report.findings.contains(&"policy_eligible_profile_requires_legal_reviewed_statute".to_string()));
        assert!(report.findings.contains(&"policy_eligible_profile_requires_legal_reviewed_sign_manual".to_string()));
    }

    #[test]
    fn policy_eligible_profile_accepts_exact_legal_source_records() {
        let mut profile = south_africa_draft();
        profile.profile_status = ProfileStatus::PolicyEligible;
        profile.effective_from = Some("2026-01-01".to_string());
        profile.rule_bundle_sha256 = Some("c".repeat(64));
        for source in &mut profile.sources {
            if matches!(source.source_kind, SourceKind::Statute | SourceKind::TrafficSignManual) {
                source.edition_or_revision = Some("reviewed-revision-1".to_string());
                source.effective_from = Some("2026-01-01".to_string());
                source.content_sha256 = Some("d".repeat(64));
                source.verification_status = VerificationStatus::LegalReviewed;
            }
        }
        let report = audit_profile(&profile);
        assert_eq!(report.disposition, "policy_eligible");
        assert!(report.policy_eligible);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn rejects_non_https_source_and_invalid_source_digest() {
        let mut profile = south_africa_draft();
        profile.sources[0].url = "http://example.org/rules".to_string();
        profile.sources[1].content_sha256 = Some("not-a-digest".to_string());
        let report = audit_profile(&profile);
        assert!(report.findings.contains(&"source_url_must_use_https".to_string()));
        assert!(report.findings.contains(&"source_content_digest_invalid".to_string()));
    }

    #[test]
    fn rejects_duplicate_source_ids_and_malformed_jurisdiction_code() {
        let mut profile = south_africa_draft();
        profile.jurisdiction_code = "ZAF".to_string();
        profile.sources[1].source_id = profile.sources[0].source_id.clone();
        let report = audit_profile(&profile);
        assert!(report.findings.contains(&"jurisdiction_code_invalid".to_string()));
        assert!(report.findings.contains(&"duplicate_source_id".to_string()));
    }

    #[test]
    fn rejects_legal_reviewed_sources_without_revision_effective_date_and_digest() {
        let mut profile = south_africa_draft();
        profile.sources[2].verification_status = VerificationStatus::LegalReviewed;
        let report = audit_profile(&profile);
        assert!(report.findings.contains(&"legal_reviewed_source_lacks_revision_date_or_digest".to_string()));
    }

    #[test]
    fn accepts_left_right_geometry_metamorphic_pair() {
        let report = audit_metamorphic_fixture(&mirrored_fixture());
        assert_eq!(report.disposition, "valid");
        assert!(report.same_task_intent);
        assert!(report.traffic_side_changed);
        assert!(report.maneuver_mirrored);
        assert!(report.geometry_mirrored);
        assert!(report.rule_bundle_changed);
    }

    #[test]
    fn rejects_metamorphic_pair_with_wrong_lateral_geometry() {
        let mut fixture = mirrored_fixture();
        fixture.alternate_geometry.lane_center_offsets_mm[0] = 1_750;
        let report = audit_metamorphic_fixture(&fixture);
        assert_eq!(report.disposition, "rejected");
        assert!(!report.geometry_mirrored);
    }

    #[test]
    fn rejects_metamorphic_pair_when_task_intent_changes() {
        let mut fixture = mirrored_fixture();
        fixture.alternate_task_intent_id = "approach-junction-and-go-straight-v1".to_string();
        let report = audit_metamorphic_fixture(&fixture);
        assert_eq!(report.disposition, "rejected");
        assert!(!report.same_task_intent);
    }

    #[test]
    fn rejects_metamorphic_pair_when_traffic_side_does_not_change() {
        let mut fixture = mirrored_fixture();
        fixture.alternate_traffic_side = TrafficSide::Left;
        let report = audit_metamorphic_fixture(&fixture);
        assert_eq!(report.disposition, "rejected");
        assert!(!report.traffic_side_changed);
    }

    #[test]
    fn rejects_invalid_rule_bundle_digests() {
        let mut fixture = mirrored_fixture();
        fixture.alternate_rule_bundle_sha256 = "wrong".to_string();
        let report = audit_metamorphic_fixture(&fixture);
        assert!(!report.rule_bundle_changed);
    }

    #[test]
    fn lateral_coordinate_mirror_rejects_signed_minimum_overflow() {
        assert!(mirror_lateral_coordinate(i64::MIN).is_err());
        assert_eq!(mirror_lateral_coordinate(1_234).unwrap(), -1_234);
    }
}
