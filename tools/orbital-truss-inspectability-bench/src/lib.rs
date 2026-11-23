use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-truss-inspectability.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-truss-inspectability.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-TRUSS-INSPECTABILITY-001";
pub const CLAIM_CEILING: &str = "synthetic_2d_geometric_observability_only";
const EPS: f64 = 1.0e-9;
const UNIT_EPS: f64 = 1.0e-6;
const MAX_POSE_PERTURBATIONS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaseClass {
    PositiveControl,
    NegativeControl,
    Metamorphic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeatureStatus {
    RobustlyCovered,
    NotNominallyVisible,
    NominalDiversityInsufficient,
    RobustDiversityInsufficient,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub case_class: CaseClass,
    pub expected_disposition: Disposition,
    pub features: Vec<Feature>,
    pub views: Vec<View>,
    pub occluders: Vec<Occluder>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub feature_id: String,
    pub position_m: [f64; 2],
    pub outward_normal: [f64; 2],
    pub minimum_distinct_failure_domains: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub view_id: String,
    pub sensor_id: String,
    pub failure_domain_id: String,
    pub origin_m: [f64; 2],
    pub heading_rad: f64,
    pub half_fov_deg: f64,
    pub min_standoff_m: f64,
    pub max_standoff_m: f64,
    pub max_incidence_deg: f64,
    pub pose_perturbations: Vec<PosePerturbation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PosePerturbation {
    pub dx_m: f64,
    pub dy_m: f64,
    pub heading_delta_rad: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Occluder {
    pub occluder_id: String,
    pub start_m: [f64; 2],
    pub end_m: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewResult {
    pub view_id: String,
    pub failure_domain_id: String,
    pub nominal_visible: bool,
    pub nominal_reason_codes: Vec<String>,
    pub robust_visible: bool,
    pub perturbations_evaluated: usize,
    pub failed_perturbation_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureResult {
    pub feature_id: String,
    pub status: FeatureStatus,
    pub nominal_view_ids: Vec<String>,
    pub robust_view_ids: Vec<String>,
    pub nominal_failure_domain_count: usize,
    pub robust_failure_domain_count: usize,
    pub required_failure_domain_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub disposition: Disposition,
    pub reason_codes: Vec<String>,
    pub features: Vec<FeatureResult>,
    pub views: Vec<ViewResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub structural_acceptance_authorized: bool,
    pub flight_authorized: bool,
    pub physical_validation: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub cases: Vec<CaseResult>,
}

fn is_finite_point(point: [f64; 2]) -> bool {
    point[0].is_finite() && point[1].is_finite()
}

fn norm(vector: [f64; 2]) -> f64 {
    vector[0].hypot(vector[1])
}

fn subtract(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn dot(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[0] + left[1] * right[1]
}

fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    cross(a, b, p).abs() <= EPS
        && p[0] >= a[0].min(b[0]) - EPS
        && p[0] <= a[0].max(b[0]) + EPS
        && p[1] >= a[1].min(b[1]) - EPS
        && p[1] <= a[1].max(b[1]) + EPS
}

// Closed-segment intersection is conservative: touching an occluder endpoint
// is treated as blocked, not as a barely-open sight line.
fn segments_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let o1 = cross(a, b, c);
    let o2 = cross(a, b, d);
    let o3 = cross(c, d, a);
    let o4 = cross(c, d, b);

    if o1.abs() <= EPS && on_segment(a, b, c) {
        return true;
    }
    if o2.abs() <= EPS && on_segment(a, b, d) {
        return true;
    }
    if o3.abs() <= EPS && on_segment(c, d, a) {
        return true;
    }
    if o4.abs() <= EPS && on_segment(c, d, b) {
        return true;
    }

    ((o1 > EPS && o2 < -EPS) || (o1 < -EPS && o2 > EPS))
        && ((o3 > EPS && o4 < -EPS) || (o3 < -EPS && o4 > EPS))
}

fn validate_case(case: &Case) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.coordinate_frame_id.trim().is_empty()
        || case.features.is_empty()
        || case.views.is_empty()
    {
        return Err("CASE_ID_FRAME_OR_REQUIRED_COLLECTION_INVALID".to_owned());
    }

    let mut feature_ids = BTreeSet::new();
    for feature in &case.features {
        let normal_length = norm(feature.outward_normal);
        if feature.feature_id.trim().is_empty()
            || !feature_ids.insert(feature.feature_id.as_str())
            || !is_finite_point(feature.position_m)
            || !is_finite_point(feature.outward_normal)
            || !normal_length.is_finite()
            || (normal_length - 1.0).abs() > UNIT_EPS
            || feature.minimum_distinct_failure_domains == 0
        {
            return Err("FEATURE_CONTRACT_INVALID".to_owned());
        }
    }

    let mut view_ids = BTreeSet::new();
    for view in &case.views {
        if view.view_id.trim().is_empty()
            || !view_ids.insert(view.view_id.as_str())
            || view.sensor_id.trim().is_empty()
            || view.failure_domain_id.trim().is_empty()
            || !is_finite_point(view.origin_m)
            || !view.heading_rad.is_finite()
            || !view.half_fov_deg.is_finite()
            || !(0.0..=180.0).contains(&view.half_fov_deg)
            || !view.min_standoff_m.is_finite()
            || view.min_standoff_m < 0.0
            || !view.max_standoff_m.is_finite()
            || view.max_standoff_m <= view.min_standoff_m
            || !view.max_incidence_deg.is_finite()
            || !(0.0..=180.0).contains(&view.max_incidence_deg)
            || view.pose_perturbations.is_empty()
            || view.pose_perturbations.len() > MAX_POSE_PERTURBATIONS
        {
            return Err("VIEW_CONTRACT_INVALID".to_owned());
        }
        for perturbation in &view.pose_perturbations {
            if !perturbation.dx_m.is_finite()
                || !perturbation.dy_m.is_finite()
                || !perturbation.heading_delta_rad.is_finite()
            {
                return Err("POSE_PERTURBATION_INVALID".to_owned());
            }
        }
    }

    let mut occluder_ids = BTreeSet::new();
    for occluder in &case.occluders {
        if occluder.occluder_id.trim().is_empty()
            || !occluder_ids.insert(occluder.occluder_id.as_str())
            || !is_finite_point(occluder.start_m)
            || !is_finite_point(occluder.end_m)
            || norm(subtract(occluder.start_m, occluder.end_m)) <= EPS
        {
            return Err("OCCLUDER_CONTRACT_INVALID".to_owned());
        }
    }
    Ok(())
}

fn view_reason_codes(
    feature: &Feature,
    view: &View,
    origin: [f64; 2],
    heading_rad: f64,
    occluders: &[Occluder],
) -> Vec<String> {
    let sensor_to_target = subtract(feature.position_m, origin);
    let distance = norm(sensor_to_target);
    let mut reasons = Vec::new();

    if distance <= EPS {
        reasons.push("ZERO_STANDOFF".to_owned());
        return reasons;
    }

    if distance < view.min_standoff_m - EPS {
        reasons.push("STANDOFF_TOO_SHORT".to_owned());
    }
    if distance > view.max_standoff_m + EPS {
        reasons.push("STANDOFF_TOO_LONG".to_owned());
    }

    let direction = [
        sensor_to_target[0] / distance,
        sensor_to_target[1] / distance,
    ];
    let forward = [heading_rad.cos(), heading_rad.sin()];
    let fov_angle_deg = dot(direction, forward).clamp(-1.0, 1.0).acos().to_degrees();
    if fov_angle_deg > view.half_fov_deg + EPS {
        reasons.push("FIELD_OF_VIEW_EXCEEDED".to_owned());
    }

    let target_to_sensor = [-direction[0], -direction[1]];
    let incidence_deg = dot(feature.outward_normal, target_to_sensor)
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    if incidence_deg > view.max_incidence_deg + EPS {
        reasons.push("INCIDENCE_ANGLE_EXCEEDED".to_owned());
    }

    if occluders.iter().any(|occluder| {
        segments_intersect(origin, feature.position_m, occluder.start_m, occluder.end_m)
    }) {
        reasons.push("LINE_OF_SIGHT_OCCLUDED".to_owned());
    }

    reasons
}

fn evaluate_view(feature: &Feature, view: &View, occluders: &[Occluder]) -> ViewResult {
    let nominal_reasons =
        view_reason_codes(feature, view, view.origin_m, view.heading_rad, occluders);
    let mut failed_perturbations = Vec::new();

    for (index, perturbation) in view.pose_perturbations.iter().enumerate() {
        let perturbed_origin = [
            view.origin_m[0] + perturbation.dx_m,
            view.origin_m[1] + perturbation.dy_m,
        ];
        let perturbed_heading = view.heading_rad + perturbation.heading_delta_rad;
        if !view_reason_codes(
            feature,
            view,
            perturbed_origin,
            perturbed_heading,
            occluders,
        )
        .is_empty()
        {
            failed_perturbations.push(index);
        }
    }

    ViewResult {
        view_id: view.view_id.clone(),
        failure_domain_id: view.failure_domain_id.clone(),
        nominal_visible: nominal_reasons.is_empty(),
        nominal_reason_codes: nominal_reasons,
        robust_visible: failed_perturbations.is_empty()
            && view_reason_codes(feature, view, view.origin_m, view.heading_rad, occluders)
                .is_empty(),
        perturbations_evaluated: view.pose_perturbations.len(),
        failed_perturbation_indices: failed_perturbations,
    }
}

pub fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    validate_case(case)?;
    let mut feature_results = Vec::with_capacity(case.features.len());
    let mut case_reasons = Vec::new();
    let mut has_nominal_gap = false;
    let mut has_robust_gap = false;
    let mut all_view_results = Vec::new();

    for feature in &case.features {
        let view_results: Vec<ViewResult> = case
            .views
            .iter()
            .map(|view| evaluate_view(feature, view, &case.occluders))
            .collect();
        all_view_results.extend(view_results.iter().cloned());

        let nominal_view_ids: Vec<String> = view_results
            .iter()
            .filter(|result| result.nominal_visible)
            .map(|result| result.view_id.clone())
            .collect();
        let robust_view_ids: Vec<String> = view_results
            .iter()
            .filter(|result| result.robust_visible)
            .map(|result| result.view_id.clone())
            .collect();

        let nominal_domains: BTreeSet<&str> = view_results
            .iter()
            .filter(|result| result.nominal_visible)
            .map(|result| result.failure_domain_id.as_str())
            .collect();
        let robust_domains: BTreeSet<&str> = view_results
            .iter()
            .filter(|result| result.robust_visible)
            .map(|result| result.failure_domain_id.as_str())
            .collect();

        let status = if nominal_view_ids.is_empty() {
            has_nominal_gap = true;
            case_reasons.push(format!(
                "FEATURE_NOT_NOMINALLY_VISIBLE:{}",
                feature.feature_id
            ));
            FeatureStatus::NotNominallyVisible
        } else if nominal_domains.len() < feature.minimum_distinct_failure_domains {
            has_nominal_gap = true;
            case_reasons.push(format!(
                "NOMINAL_FAILURE_DOMAIN_DIVERSITY_INSUFFICIENT:{}",
                feature.feature_id
            ));
            FeatureStatus::NominalDiversityInsufficient
        } else if robust_domains.len() < feature.minimum_distinct_failure_domains {
            has_robust_gap = true;
            case_reasons.push(format!(
                "ROBUST_FAILURE_DOMAIN_DIVERSITY_NOT_ESTABLISHED:{}",
                feature.feature_id
            ));
            FeatureStatus::RobustDiversityInsufficient
        } else {
            FeatureStatus::RobustlyCovered
        };

        feature_results.push(FeatureResult {
            feature_id: feature.feature_id.clone(),
            status,
            nominal_view_ids,
            robust_view_ids,
            nominal_failure_domain_count: nominal_domains.len(),
            robust_failure_domain_count: robust_domains.len(),
            required_failure_domain_count: feature.minimum_distinct_failure_domains,
        });
    }

    let disposition = if has_nominal_gap {
        Disposition::Infeasible
    } else if has_robust_gap {
        Disposition::Indeterminate
    } else {
        Disposition::Feasible
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        coordinate_frame_id: case.coordinate_frame_id.clone(),
        disposition,
        reason_codes: case_reasons,
        features: feature_results,
        views: all_view_results,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }

    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.len() != 12
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(corpus.cases.len());
    let mut expected_matches = 0usize;
    let mut expected_mismatches = 0usize;

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("DUPLICATE_CASE_ID".to_owned());
        }
        let result = evaluate_case(case)?;
        if result.disposition == case.expected_disposition {
            expected_matches += 1;
        } else {
            expected_mismatches += 1;
        }
        results.push(result);
    }

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        authority_granted: false,
        structural_acceptance_authorized: false,
        flight_authorized: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        case_count: results.len(),
        expected_outcome_matches: expected_matches,
        expected_outcome_mismatches: expected_mismatches,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../docs/research/orbital-truss/fixtures/inspectability-v0_1.json");

    fn rotate_translate(point: [f64; 2], angle: f64, translation: [f64; 2]) -> [f64; 2] {
        let (sin, cos) = angle.sin_cos();
        [
            cos * point[0] - sin * point[1] + translation[0],
            sin * point[0] + cos * point[1] + translation[1],
        ]
    }

    fn reparameterize(case: &mut Case, angle: f64, translation: [f64; 2]) {
        let (sin, cos) = angle.sin_cos();
        for feature in &mut case.features {
            feature.position_m = rotate_translate(feature.position_m, angle, translation);
            feature.outward_normal = [
                cos * feature.outward_normal[0] - sin * feature.outward_normal[1],
                sin * feature.outward_normal[0] + cos * feature.outward_normal[1],
            ];
        }
        for view in &mut case.views {
            view.origin_m = rotate_translate(view.origin_m, angle, translation);
            view.heading_rad += angle;
            for perturbation in &mut view.pose_perturbations {
                let dx = perturbation.dx_m;
                let dy = perturbation.dy_m;
                perturbation.dx_m = cos * dx - sin * dy;
                perturbation.dy_m = sin * dx + cos * dy;
            }
        }
        for occluder in &mut case.occluders {
            occluder.start_m = rotate_translate(occluder.start_m, angle, translation);
            occluder.end_m = rotate_translate(occluder.end_m, angle, translation);
        }
    }

    #[test]
    fn frozen_corpus_matches_all_adjudicated_outcomes() {
        let receipt = audit_corpus(FIXTURE.as_bytes(), &"a".repeat(40)).unwrap();
        assert_eq!(receipt.case_count, 12);
        assert_eq!(receipt.expected_outcome_matches, 12);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
        assert!(!receipt.authority_granted);
        assert!(!receipt.structural_acceptance_authorized);
        assert!(!receipt.flight_authorized);
        assert_eq!(receipt.physical_validation, "NOT_ASSESSED");
    }

    #[test]
    fn rigid_frame_reparameterization_preserves_every_disposition() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        for case in &corpus.cases {
            let baseline = evaluate_case(case).unwrap();
            for index in 0..128 {
                let angle = (index as f64) * 0.071;
                let translation = [(index as f64) * 0.17 - 4.0, 3.0 - (index as f64) * 0.11];
                let mut transformed = case.clone();
                reparameterize(&mut transformed, angle, translation);
                assert_eq!(
                    evaluate_case(&transformed).unwrap(),
                    baseline,
                    "{} at transform {}",
                    case.case_id,
                    index
                );
            }
        }
    }

    #[test]
    fn endpoint_touching_occluder_is_conservatively_blocked() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OTI-010-OCCLUDER-ENDPOINT-TOUCH")
            .unwrap();
        let result = evaluate_case(case).unwrap();
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert!(
            result.views[0]
                .nominal_reason_codes
                .contains(&"LINE_OF_SIGHT_OCCLUDED".to_owned())
        );
    }

    #[test]
    fn same_declared_failure_domain_does_not_count_as_two_paths() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OTI-008-DUPLICATE-FAILURE-DOMAIN")
            .unwrap();
        let result = evaluate_case(case).unwrap();
        assert_eq!(result.disposition, Disposition::Infeasible);
        assert_eq!(result.features[0].nominal_failure_domain_count, 1);
    }

    #[test]
    fn failed_pose_challenge_abstains_instead_of_claiming_robust_coverage() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OTI-007-POSE-FRAGILITY")
            .unwrap();
        let result = evaluate_case(case).unwrap();
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert_eq!(result.views[0].failed_perturbation_indices, vec![1]);
    }

    #[test]
    fn invalid_unit_normal_is_rejected() {
        let mut corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        corpus.cases[0].features[0].outward_normal = [2.0, 0.0];
        assert_eq!(
            evaluate_case(&corpus.cases[0]),
            Err("FEATURE_CONTRACT_INVALID".to_owned())
        );
    }

    #[test]
    fn blank_coordinate_frame_is_rejected() {
        let mut corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        corpus.cases[0].coordinate_frame_id.clear();
        assert_eq!(
            evaluate_case(&corpus.cases[0]),
            Err("CASE_ID_FRAME_OR_REQUIRED_COLLECTION_INVALID".to_owned())
        );
    }

    #[test]
    fn excessive_pose_perturbations_are_rejected() {
        let mut corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let perturbation = corpus.cases[0].views[0].pose_perturbations[0].clone();
        corpus.cases[0].views[0].pose_perturbations =
            vec![perturbation; MAX_POSE_PERTURBATIONS + 1];
        assert_eq!(
            evaluate_case(&corpus.cases[0]),
            Err("VIEW_CONTRACT_INVALID".to_owned())
        );
    }

    #[test]
    fn non_frozen_case_count_is_rejected() {
        let mut corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        corpus.cases.pop();
        let bytes = serde_json::to_vec(&corpus).unwrap();
        assert_eq!(
            audit_corpus(&bytes, &"a".repeat(40)),
            Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned())
        );
    }

    #[test]
    fn duplicate_view_identity_is_rejected() {
        let mut corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let duplicate_view = corpus.cases[0].views[0].clone();
        corpus.cases[0].views.push(duplicate_view);
        assert_eq!(
            evaluate_case(&corpus.cases[0]),
            Err("VIEW_CONTRACT_INVALID".to_owned())
        );
    }

    #[test]
    fn malformed_source_commit_is_rejected() {
        assert_eq!(
            audit_corpus(FIXTURE.as_bytes(), "not-a-commit"),
            Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned())
        );
    }
}

pub mod planning;
