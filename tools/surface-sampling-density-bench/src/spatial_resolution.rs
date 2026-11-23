use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "luminous.orbital-sampling-density.spatial-resolution-corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-sampling-density.spatial-resolution-receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-SPATIAL-RESOLUTION-001";
pub const CLAIM_CEILING: &str =
    "synthetic_spatial_resolution_density_matrix_with_visibility_fallback_only";
pub const CASE_COUNT: usize = 6;
const DENSITY_NUMERATOR_SCALE: u128 = 1_000_000_000_000_000_000;
const NORMALIZED_RATIO_SCALE: u128 = 1_000_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<SpatialResolutionCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpatialResolutionCase {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub candidate_view_ids: Vec<String>,
    pub selected_view_ids: Vec<String>,
    pub max_selected_views: usize,
    pub elements: Vec<SpatialResolutionElement>,
    pub expected_status: CaseStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpatialResolutionElement {
    pub element_id: String,
    pub mesh_revision_id: String,
    pub area_um2: u64,
    pub target_micro_sqrt_rays_per_mm: u64,
    pub ray_counts: Vec<ViewRayCount>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ViewRayCount {
    pub view_id: String,
    pub pixel_ray_intersections: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ElementMode {
    SpatialResolutionDensity,
    BinaryVisibilityFallback,
    Indeterminate,
    InfeasibleNoCandidateVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CaseStatus {
    Feasible,
    FeasibleVisibilityFallback,
    BudgetInfeasible,
    Infeasible,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "representation", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatrixEntry {
    DensityNormalized {
        lower_ratio_ppm: String,
        upper_ratio_ppm: String,
    },
    BinaryVisibility {
        visible: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewContribution {
    pub view_id: String,
    pub pixel_ray_intersections: u64,
    pub density_lower_micro: u64,
    pub density_upper_micro: u64,
    pub exact_density_value: bool,
    pub selected: bool,
    pub matrix_entry: MatrixEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ElementReceipt {
    pub element_id: String,
    pub mesh_revision_id: String,
    pub area_um2: u64,
    pub target_micro_sqrt_rays_per_mm: u64,
    pub mode: ElementMode,
    pub candidate_density_lower_micro: String,
    pub candidate_density_upper_micro: String,
    pub selected_density_lower_micro: String,
    pub selected_density_upper_micro: String,
    pub candidate_density_guaranteed: bool,
    pub selected_density_guaranteed: bool,
    pub candidate_visible: bool,
    pub selected_visible: bool,
    pub selected_requirement_met: bool,
    pub view_contributions: Vec<ViewContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseReceipt {
    pub case_id: String,
    pub status: CaseStatus,
    pub reason_codes: Vec<String>,
    pub candidate_view_count: usize,
    pub selected_view_count: usize,
    pub max_selected_views: usize,
    pub visibility_fallback_element_count: usize,
    pub density_mode_element_count: usize,
    pub density_targets_guaranteed_count: usize,
    pub selected_requirements_met: bool,
    pub expected_status: CaseStatus,
    pub expected_status_matches: bool,
    pub input_sha256: String,
    pub elements: Vec<ElementReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusReceipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub fixture_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseReceipt>,
}

#[derive(Debug, Clone, Copy)]
struct DensityInterval {
    lower: u64,
    upper: u64,
    exact: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut current = value;
    let mut next = (current + 1) / 2;
    while next < current {
        current = next;
        next = (current + value / current) / 2;
    }
    current
}

fn density_interval(pixel_rays: u64, area_um2: u64) -> Result<DensityInterval, String> {
    if area_um2 == 0 {
        return Err("SURFACE_ELEMENT_AREA_ZERO".to_owned());
    }
    let numerator = u128::from(pixel_rays)
        .checked_mul(DENSITY_NUMERATOR_SCALE)
        .ok_or_else(|| "SPATIAL_DENSITY_NUMERATOR_OVERFLOW".to_owned())?;
    let radicand_floor = numerator / u128::from(area_um2);
    let lower = integer_sqrt(radicand_floor);
    let square = lower
        .checked_mul(lower)
        .and_then(|value| value.checked_mul(u128::from(area_um2)))
        .ok_or_else(|| "SPATIAL_DENSITY_SQUARE_OVERFLOW".to_owned())?;
    let exact = square == numerator;
    let upper = if exact {
        lower
    } else {
        lower
            .checked_add(1)
            .ok_or_else(|| "SPATIAL_DENSITY_UPPER_BOUND_OVERFLOW".to_owned())?
    };
    Ok(DensityInterval {
        lower: u64::try_from(lower)
            .map_err(|_| "SPATIAL_DENSITY_LOWER_BOUND_OVERFLOW".to_owned())?,
        upper: u64::try_from(upper)
            .map_err(|_| "SPATIAL_DENSITY_UPPER_BOUND_OVERFLOW".to_owned())?,
        exact,
    })
}

fn ratio_ppm_bound(value: u128, target: u64, round_up: bool) -> Result<String, String> {
    let numerator = value
        .checked_mul(NORMALIZED_RATIO_SCALE)
        .ok_or_else(|| "SPATIAL_RATIO_NUMERATOR_OVERFLOW".to_owned())?;
    let denominator = u128::from(target);
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let result = if round_up && remainder != 0 {
        quotient
            .checked_add(1)
            .ok_or_else(|| "SPATIAL_RATIO_UPPER_OVERFLOW".to_owned())?
    } else {
        quotient
    };
    Ok(result.to_string())
}

fn validate_case(case: &SpatialResolutionCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.coordinate_frame_id.trim().is_empty()
        || case.candidate_view_ids.is_empty()
        || case.elements.is_empty()
    {
        return Err("SPATIAL_CASE_ID_FRAME_OR_COLLECTIONS_INVALID".to_owned());
    }

    let candidate_ids: BTreeSet<&str> =
        case.candidate_view_ids.iter().map(String::as_str).collect();
    if candidate_ids.len() != case.candidate_view_ids.len()
        || candidate_ids.iter().any(|view_id| view_id.trim().is_empty())
    {
        return Err("CANDIDATE_VIEW_IDS_DUPLICATE_OR_BLANK".to_owned());
    }
    let selected_ids: BTreeSet<&str> =
        case.selected_view_ids.iter().map(String::as_str).collect();
    if selected_ids.len() != case.selected_view_ids.len()
        || selected_ids.iter().any(|view_id| !candidate_ids.contains(view_id))
    {
        return Err("SELECTED_VIEW_IDS_DUPLICATE_OR_UNKNOWN".to_owned());
    }

    let mut element_ids = BTreeSet::new();
    for element in &case.elements {
        if element.element_id.trim().is_empty()
            || element.mesh_revision_id.trim().is_empty()
            || !element_ids.insert(element.element_id.as_str())
            || element.area_um2 == 0
            || element.target_micro_sqrt_rays_per_mm == 0
        {
            return Err("SPATIAL_ELEMENT_ID_AREA_OR_TARGET_INVALID".to_owned());
        }
        let ray_views: BTreeSet<&str> =
            element.ray_counts.iter().map(|row| row.view_id.as_str()).collect();
        if ray_views.len() != element.ray_counts.len() || ray_views != candidate_ids {
            return Err("SPATIAL_RAY_COUNT_MATRIX_MUST_MATCH_CANDIDATE_VIEWS".to_owned());
        }
    }
    Ok(())
}

fn evaluate_case(case: &SpatialResolutionCase) -> Result<CaseReceipt, String> {
    validate_case(case)?;
    let selected_ids: BTreeSet<&str> =
        case.selected_view_ids.iter().map(String::as_str).collect();
    let mut element_receipts = Vec::with_capacity(case.elements.len());
    let mut fallback_count = 0_usize;
    let mut density_mode_count = 0_usize;
    let mut guaranteed_count = 0_usize;
    let mut any_infeasible = false;
    let mut any_budget_infeasible = false;
    let mut any_indeterminate = false;
    let mut all_selected_requirements_met = true;
    let mut ordered_elements: Vec<&SpatialResolutionElement> = case.elements.iter().collect();
    ordered_elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));

    for element in ordered_elements {
        let mut ordered_counts: Vec<&ViewRayCount> = element.ray_counts.iter().collect();
        ordered_counts.sort_by(|left, right| left.view_id.cmp(&right.view_id));
        let mut intervals = BTreeMap::new();
        let mut candidate_lower = 0_u128;
        let mut candidate_upper = 0_u128;
        let mut selected_lower = 0_u128;
        let mut selected_upper = 0_u128;
        let mut candidate_visible = false;
        let mut selected_visible = false;

        for row in &ordered_counts {
            let interval = density_interval(row.pixel_ray_intersections, element.area_um2)?;
            candidate_lower = candidate_lower
                .checked_add(u128::from(interval.lower))
                .ok_or_else(|| "CANDIDATE_DENSITY_SUM_OVERFLOW".to_owned())?;
            candidate_upper = candidate_upper
                .checked_add(u128::from(interval.upper))
                .ok_or_else(|| "CANDIDATE_DENSITY_SUM_OVERFLOW".to_owned())?;
            candidate_visible |= row.pixel_ray_intersections > 0;
            if selected_ids.contains(row.view_id.as_str()) {
                selected_lower = selected_lower
                    .checked_add(u128::from(interval.lower))
                    .ok_or_else(|| "SELECTED_DENSITY_SUM_OVERFLOW".to_owned())?;
                selected_upper = selected_upper
                    .checked_add(u128::from(interval.upper))
                    .ok_or_else(|| "SELECTED_DENSITY_SUM_OVERFLOW".to_owned())?;
                selected_visible |= row.pixel_ray_intersections > 0;
            }
            intervals.insert(row.view_id.as_str(), interval);
        }

        let target = u128::from(element.target_micro_sqrt_rays_per_mm);
        let mode = if candidate_lower >= target {
            ElementMode::SpatialResolutionDensity
        } else if candidate_upper < target {
            if candidate_visible {
                ElementMode::BinaryVisibilityFallback
            } else {
                ElementMode::InfeasibleNoCandidateVisibility
            }
        } else {
            ElementMode::Indeterminate
        };

        let (selected_requirement_met, selected_density_guaranteed) = match mode {
            ElementMode::SpatialResolutionDensity => {
                if selected_lower >= target {
                    (true, true)
                } else if selected_upper < target {
                    (false, false)
                } else {
                    (false, false)
                }
            }
            ElementMode::BinaryVisibilityFallback => (selected_visible, false),
            ElementMode::InfeasibleNoCandidateVisibility => (false, false),
            ElementMode::Indeterminate => (false, false),
        };

        match mode {
            ElementMode::SpatialResolutionDensity => {
                density_mode_count += 1;
                if selected_density_guaranteed {
                    guaranteed_count += 1;
                } else if selected_upper >= target && selected_lower < target {
                    any_indeterminate = true;
                } else {
                    any_budget_infeasible = true;
                }
            }
            ElementMode::BinaryVisibilityFallback => {
                fallback_count += 1;
                if !selected_visible {
                    any_budget_infeasible = true;
                }
            }
            ElementMode::InfeasibleNoCandidateVisibility => any_infeasible = true,
            ElementMode::Indeterminate => any_indeterminate = true,
        }
        all_selected_requirements_met &= selected_requirement_met;

        let mut view_contributions = Vec::with_capacity(ordered_counts.len());
        for row in ordered_counts {
            let interval = intervals[row.view_id.as_str()];
            let selected = selected_ids.contains(row.view_id.as_str());
            let matrix_entry = match mode {
                ElementMode::SpatialResolutionDensity => MatrixEntry::DensityNormalized {
                    lower_ratio_ppm: ratio_ppm_bound(
                        u128::from(interval.lower),
                        element.target_micro_sqrt_rays_per_mm,
                        false,
                    )?,
                    upper_ratio_ppm: ratio_ppm_bound(
                        u128::from(interval.upper),
                        element.target_micro_sqrt_rays_per_mm,
                        true,
                    )?,
                },
                ElementMode::BinaryVisibilityFallback
                | ElementMode::InfeasibleNoCandidateVisibility => MatrixEntry::BinaryVisibility {
                    visible: row.pixel_ray_intersections > 0,
                },
                ElementMode::Indeterminate => MatrixEntry::BinaryVisibility {
                    visible: row.pixel_ray_intersections > 0,
                },
            };
            view_contributions.push(ViewContribution {
                view_id: row.view_id.clone(),
                pixel_ray_intersections: row.pixel_ray_intersections,
                density_lower_micro: interval.lower,
                density_upper_micro: interval.upper,
                exact_density_value: interval.exact,
                selected,
                matrix_entry,
            });
        }

        element_receipts.push(ElementReceipt {
            element_id: element.element_id.clone(),
            mesh_revision_id: element.mesh_revision_id.clone(),
            area_um2: element.area_um2,
            target_micro_sqrt_rays_per_mm: element.target_micro_sqrt_rays_per_mm,
            mode,
            candidate_density_lower_micro: candidate_lower.to_string(),
            candidate_density_upper_micro: candidate_upper.to_string(),
            selected_density_lower_micro: selected_lower.to_string(),
            selected_density_upper_micro: selected_upper.to_string(),
            candidate_density_guaranteed: candidate_lower >= target,
            selected_density_guaranteed,
            candidate_visible,
            selected_visible,
            selected_requirement_met,
            view_contributions,
        });
    }

    let selected_view_budget_met = case.selected_view_ids.len() <= case.max_selected_views;
    if !selected_view_budget_met {
        any_budget_infeasible = true;
    }
    let status = if any_infeasible {
        CaseStatus::Infeasible
    } else if any_indeterminate {
        CaseStatus::Indeterminate
    } else if any_budget_infeasible {
        CaseStatus::BudgetInfeasible
    } else if fallback_count > 0 {
        CaseStatus::FeasibleVisibilityFallback
    } else {
        CaseStatus::Feasible
    };

    let mut reason_codes = Vec::new();
    if fallback_count > 0 {
        reason_codes.push("CANDIDATE_POOL_BELOW_SPATIAL_DENSITY_TARGET_BINARY_VISIBILITY_FALLBACK".to_owned());
    }
    if any_budget_infeasible {
        reason_codes.push("SELECTED_VIEW_SET_DOES_NOT_MEET_DENSITY_OR_VISIBILITY_MATRIX".to_owned());
    }
    if any_indeterminate {
        reason_codes.push("FIXED_POINT_INTERVAL_STRADDLES_DENSITY_TARGET".to_owned());
    }
    if any_infeasible {
        reason_codes.push("NO_CANDIDATE_VIEW_HAS_ELEMENT_VISIBILITY".to_owned());
    }
    if !selected_view_budget_met {
        reason_codes.push("SELECTED_VIEW_BUDGET_EXCEEDED".to_owned());
    }

    let bytes = serde_json::to_vec(case)
        .map_err(|error| format!("SPATIAL_CASE_SERIALIZATION_FAILED:{error}"))?;
    Ok(CaseReceipt {
        case_id: case.case_id.clone(),
        status,
        reason_codes,
        candidate_view_count: case.candidate_view_ids.len(),
        selected_view_count: case.selected_view_ids.len(),
        max_selected_views: case.max_selected_views,
        visibility_fallback_element_count: fallback_count,
        density_mode_element_count: density_mode_count,
        density_targets_guaranteed_count: guaranteed_count,
        selected_requirements_met: all_selected_requirements_met && selected_view_budget_met,
        expected_status: case.expected_status,
        expected_status_matches: status == case.expected_status,
        input_sha256: sha256_hex(&bytes),
        elements: element_receipts,
    })
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<CorpusReceipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("SPATIAL_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.len() != CASE_COUNT
    {
        return Err("SPATIAL_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut matches = 0_usize;
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("DUPLICATE_SPATIAL_CASE_ID".to_owned());
        }
        let receipt = match evaluate_case(case) {
            Ok(receipt) => receipt,
            Err(reason) => {
                let bytes = serde_json::to_vec(case)
                    .map_err(|error| format!("SPATIAL_CASE_SERIALIZATION_FAILED:{error}"))?;
                CaseReceipt {
                    case_id: case.case_id.clone(),
                    status: CaseStatus::InvalidInput,
                    reason_codes: vec![reason],
                    candidate_view_count: case.candidate_view_ids.len(),
                    selected_view_count: case.selected_view_ids.len(),
                    max_selected_views: case.max_selected_views,
                    visibility_fallback_element_count: 0,
                    density_mode_element_count: 0,
                    density_targets_guaranteed_count: 0,
                    selected_requirements_met: false,
                    expected_status: case.expected_status,
                    expected_status_matches: false,
                    input_sha256: sha256_hex(&bytes),
                    elements: Vec::new(),
                }
            }
        };
        if receipt.status == case.expected_status {
            matches += 1;
        }
        cases.push(receipt);
    }
    Ok(CorpusReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        fixture_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_outcome_matches: matches,
        expected_outcome_mismatches: cases.len() - matches,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(view_id: &str, rays: u64) -> ViewRayCount {
        ViewRayCount {
            view_id: view_id.to_owned(),
            pixel_ray_intersections: rays,
        }
    }

    fn element(target: u64, counts: Vec<ViewRayCount>) -> SpatialResolutionElement {
        SpatialResolutionElement {
            element_id: "ELEMENT-A".to_owned(),
            mesh_revision_id: "MESH-R1".to_owned(),
            area_um2: 2_000_000_000_000,
            target_micro_sqrt_rays_per_mm: target,
            ray_counts: counts,
        }
    }

    fn case(
        id: &str,
        candidates: &[&str],
        selected: &[&str],
        max_views: usize,
        elements: Vec<SpatialResolutionElement>,
        expected_status: CaseStatus,
    ) -> SpatialResolutionCase {
        SpatialResolutionCase {
            case_id: id.to_owned(),
            coordinate_frame_id: "SYNTHETIC-WORLD".to_owned(),
            candidate_view_ids: candidates.iter().map(|value| (*value).to_owned()).collect(),
            selected_view_ids: selected.iter().map(|value| (*value).to_owned()).collect(),
            max_selected_views: max_views,
            elements,
            expected_status,
        }
    }

    #[test]
    fn one_view_density_above_target_uses_density_normalized_matrix() {
        let case = case(
            "KNOWN-ONE-VIEW",
            &["V1"],
            &["V1"],
            1,
            vec![element(1_500, vec![count("V1", 5)])],
            CaseStatus::Feasible,
        );
        let receipt = evaluate_case(&case).unwrap();
        let element = &receipt.elements[0];
        assert_eq!(element.mode, ElementMode::SpatialResolutionDensity);
        assert!(element.selected_density_guaranteed);
        assert_eq!(element.view_contributions[0].density_lower_micro, 1_581);
        assert_eq!(element.view_contributions[0].density_upper_micro, 1_582);
        assert_eq!(receipt.status, CaseStatus::Feasible);
    }

    #[test]
    fn insufficient_candidate_density_falls_back_to_binary_visibility() {
        let case = case(
            "KNOWN-FALLBACK",
            &["V1"],
            &["V1"],
            1,
            vec![element(2_000, vec![count("V1", 5)])],
            CaseStatus::FeasibleVisibilityFallback,
        );
        let receipt = evaluate_case(&case).unwrap();
        assert_eq!(receipt.elements[0].mode, ElementMode::BinaryVisibilityFallback);
        assert!(!receipt.elements[0].candidate_density_guaranteed);
        assert!(receipt.elements[0].selected_requirement_met);
        assert_eq!(receipt.status, CaseStatus::FeasibleVisibilityFallback);
    }

    #[test]
    fn density_accumulates_per_view_square_root_contributions() {
        let case = case(
            "KNOWN-MULTIVIEW",
            &["V1", "V2"],
            &["V1", "V2"],
            2,
            vec![element(3_000, vec![count("V1", 5), count("V2", 5)])],
            CaseStatus::Feasible,
        );
        let receipt = evaluate_case(&case).unwrap();
        assert_eq!(receipt.elements[0].mode, ElementMode::SpatialResolutionDensity);
        assert!(receipt.elements[0].selected_density_guaranteed);
        assert_eq!(receipt.elements[0].candidate_density_lower_micro, "3162");
    }

    #[test]
    fn threshold_interval_overlap_is_indeterminate_not_rounded_to_a_pass() {
        let case = case(
            "KNOWN-BOUNDARY",
            &["V1", "V2"],
            &["V1", "V2"],
            2,
            vec![element(3_163, vec![count("V1", 5), count("V2", 5)])],
            CaseStatus::Indeterminate,
        );
        let receipt = evaluate_case(&case).unwrap();
        assert_eq!(receipt.elements[0].mode, ElementMode::Indeterminate);
        assert_eq!(receipt.status, CaseStatus::Indeterminate);
    }

    #[test]
    fn selected_subset_below_density_target_is_budget_infeasible() {
        let case = case(
            "KNOWN-SELECTED-BUDGET",
            &["V1", "V2"],
            &["V1"],
            1,
            vec![element(3_000, vec![count("V1", 5), count("V2", 5)])],
            CaseStatus::BudgetInfeasible,
        );
        let receipt = evaluate_case(&case).unwrap();
        assert_eq!(receipt.status, CaseStatus::BudgetInfeasible);
        assert!(receipt.elements[0].candidate_density_guaranteed);
        assert!(!receipt.elements[0].selected_density_guaranteed);
    }

    #[test]
    fn zero_visibility_is_infeasible_not_visibility_fallback_pass() {
        let case = case(
            "KNOWN-NO-VIEW",
            &["V1"],
            &[],
            1,
            vec![element(2_000, vec![count("V1", 0)])],
            CaseStatus::Infeasible,
        );
        let receipt = evaluate_case(&case).unwrap();
        assert_eq!(receipt.elements[0].mode, ElementMode::InfeasibleNoCandidateVisibility);
        assert_eq!(receipt.status, CaseStatus::Infeasible);
    }
}
