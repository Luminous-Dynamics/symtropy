use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-sampling-density.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-sampling-density.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-SAMPLING-DENSITY-001";
pub const CLAIM_CEILING: &str =
    "synthetic_ray_intersection_density_accounting_on_declared_surface_elements_only";
pub const CASE_COUNT: usize = 13;
const DENSITY_SCALE: u128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Provenance {
    ExactSyntheticOracle,
    CallerSuppliedUnverified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DensityStatus {
    Feasible,
    Infeasible,
    BudgetInfeasible,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<SamplingCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingCase {
    pub case_id: String,
    pub coordinate_frame_id: String,
    pub area_unit: String,
    pub density_unit: String,
    pub candidate_views: Vec<String>,
    pub selected_view_ids: Vec<String>,
    pub max_selected_views: usize,
    pub elements: Vec<SurfaceElement>,
    pub expected_status: DensityStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceElement {
    pub element_id: String,
    pub mesh_revision_id: String,
    pub area_um2: u64,
    pub minimum_density_micro_rays_per_mm2: u64,
    pub minimum_contributing_views: usize,
    pub ray_counts: Vec<ViewRayCount>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ViewRayCount {
    pub view_id: String,
    pub intersection_count: u64,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ViewContribution {
    pub view_id: String,
    pub intersection_count: u64,
    pub selected: bool,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ElementDensityReport {
    pub element_id: String,
    pub mesh_revision_id: String,
    pub area_um2: u64,
    pub minimum_density_micro_rays_per_mm2: u64,
    pub minimum_contributing_views: usize,
    pub candidate_total_intersections: u64,
    pub selected_total_intersections: u64,
    pub candidate_contributing_views: usize,
    pub selected_contributing_views: usize,
    pub candidate_density_meets_target: bool,
    pub selected_density_meets_target: bool,
    pub candidate_view_count_meets_target: bool,
    pub selected_view_count_meets_target: bool,
    pub provenance_complete: bool,
    pub selected_density_rational_numerator_micro_mm2: String,
    pub density_rational_denominator_area_um2: u64,
    pub view_contributions: Vec<ViewContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseReceipt {
    pub case_id: String,
    pub status: DensityStatus,
    pub reason_codes: Vec<String>,
    pub candidate_pool_sufficient: bool,
    pub selected_plan_sufficient: bool,
    pub selected_view_budget_met: bool,
    pub provenance_complete: bool,
    pub candidate_view_count: usize,
    pub selected_view_count: usize,
    pub max_selected_views: usize,
    pub elements: Vec<ElementDensityReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn density_meets(hits: u64, area_um2: u64, target_micro: u64) -> Result<bool, String> {
    if area_um2 == 0 {
        return Err("SURFACE_ELEMENT_AREA_ZERO".to_owned());
    }
    let left = u128::from(hits)
        .checked_mul(DENSITY_SCALE * DENSITY_SCALE)
        .ok_or_else(|| "DENSITY_NUMERATOR_OVERFLOW".to_owned())?;
    let right = u128::from(target_micro)
        .checked_mul(u128::from(area_um2))
        .ok_or_else(|| "DENSITY_THRESHOLD_OVERFLOW".to_owned())?;
    Ok(left >= right)
}

fn validate_case(case: &SamplingCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.coordinate_frame_id.trim().is_empty()
        || case.area_unit != "SQUARE_MICROMETRE"
        || case.density_unit != "RAYS_PER_SQUARE_MILLIMETRE"
        || case.candidate_views.is_empty()
        || case.elements.is_empty()
    {
        return Err("CASE_ID_FRAME_UNITS_OR_COLLECTIONS_INVALID".to_owned());
    }
    let candidate_ids: BTreeSet<&str> = case.candidate_views.iter().map(String::as_str).collect();
    if candidate_ids.len() != case.candidate_views.len()
        || candidate_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err("CANDIDATE_VIEW_IDS_DUPLICATE_OR_BLANK".to_owned());
    }
    let selected_ids: BTreeSet<&str> = case.selected_view_ids.iter().map(String::as_str).collect();
    if selected_ids.len() != case.selected_view_ids.len()
        || selected_ids.iter().any(|id| !candidate_ids.contains(id))
    {
        return Err("SELECTED_VIEW_IDS_DUPLICATE_OR_UNKNOWN".to_owned());
    }

    let mut element_ids = BTreeSet::new();
    for element in &case.elements {
        if element.element_id.trim().is_empty()
            || element.mesh_revision_id.trim().is_empty()
            || !element_ids.insert(element.element_id.as_str())
            || element.area_um2 == 0
            || element.minimum_density_micro_rays_per_mm2 == 0
            || element.minimum_contributing_views == 0
        {
            return Err("SURFACE_ELEMENT_ID_AREA_OR_TARGET_INVALID".to_owned());
        }
        let ray_view_ids: BTreeSet<&str> = element
            .ray_counts
            .iter()
            .map(|entry| entry.view_id.as_str())
            .collect();
        if ray_view_ids.len() != element.ray_counts.len() || ray_view_ids != candidate_ids {
            return Err("RAY_COUNT_MATRIX_MUST_MATCH_CANDIDATE_VIEWS_EXACTLY".to_owned());
        }
    }
    Ok(())
}

pub(crate) fn evaluate_case(case: &SamplingCase) -> Result<CaseReceipt, String> {
    validate_case(case)?;
    let selected_ids: BTreeSet<&str> = case.selected_view_ids.iter().map(String::as_str).collect();

    let mut elements = Vec::with_capacity(case.elements.len());
    let mut candidate_pool_sufficient = true;
    let mut selected_plan_sufficient = true;
    let mut provenance_complete = true;
    let mut ordered_elements: Vec<&SurfaceElement> = case.elements.iter().collect();
    ordered_elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));

    for element in ordered_elements {
        let mut candidate_total = 0_u64;
        let mut selected_total = 0_u64;
        let mut candidate_view_count = 0_usize;
        let mut selected_view_count = 0_usize;
        let mut element_provenance_complete = true;
        let mut view_contributions = Vec::with_capacity(element.ray_counts.len());

        let mut ordered_counts: Vec<&ViewRayCount> = element.ray_counts.iter().collect();
        ordered_counts.sort_by(|left, right| left.view_id.cmp(&right.view_id));
        for ray_count in ordered_counts {
            if ray_count.provenance != Provenance::ExactSyntheticOracle {
                element_provenance_complete = false;
            }
            candidate_total = candidate_total
                .checked_add(ray_count.intersection_count)
                .ok_or_else(|| "RAY_INTERSECTION_SUM_OVERFLOW".to_owned())?;
            if ray_count.intersection_count > 0 {
                candidate_view_count += 1;
            }
            let is_selected = selected_ids.contains(ray_count.view_id.as_str());
            if is_selected {
                selected_total = selected_total
                    .checked_add(ray_count.intersection_count)
                    .ok_or_else(|| "SELECTED_RAY_INTERSECTION_SUM_OVERFLOW".to_owned())?;
                if ray_count.intersection_count > 0 {
                    selected_view_count += 1;
                }
            }
            view_contributions.push(ViewContribution {
                view_id: ray_count.view_id.clone(),
                intersection_count: ray_count.intersection_count,
                selected: is_selected,
                provenance: ray_count.provenance,
            });
        }

        let candidate_density_meets = density_meets(
            candidate_total,
            element.area_um2,
            element.minimum_density_micro_rays_per_mm2,
        )?;
        let selected_density_meets = density_meets(
            selected_total,
            element.area_um2,
            element.minimum_density_micro_rays_per_mm2,
        )?;
        let candidate_view_count_meets = candidate_view_count >= element.minimum_contributing_views;
        let selected_view_count_meets = selected_view_count >= element.minimum_contributing_views;

        candidate_pool_sufficient &= candidate_density_meets && candidate_view_count_meets;
        selected_plan_sufficient &= selected_density_meets && selected_view_count_meets;
        provenance_complete &= element_provenance_complete;

        let selected_density_numerator = u128::from(selected_total)
            .checked_mul(DENSITY_SCALE * DENSITY_SCALE)
            .ok_or_else(|| "DENSITY_RATIONAL_NUMERATOR_OVERFLOW".to_owned())?;
        elements.push(ElementDensityReport {
            element_id: element.element_id.clone(),
            mesh_revision_id: element.mesh_revision_id.clone(),
            area_um2: element.area_um2,
            minimum_density_micro_rays_per_mm2: element.minimum_density_micro_rays_per_mm2,
            minimum_contributing_views: element.minimum_contributing_views,
            candidate_total_intersections: candidate_total,
            selected_total_intersections: selected_total,
            candidate_contributing_views: candidate_view_count,
            selected_contributing_views: selected_view_count,
            candidate_density_meets_target: candidate_density_meets,
            selected_density_meets_target: selected_density_meets,
            candidate_view_count_meets_target: candidate_view_count_meets,
            selected_view_count_meets_target: selected_view_count_meets,
            provenance_complete: element_provenance_complete,
            selected_density_rational_numerator_micro_mm2: selected_density_numerator.to_string(),
            density_rational_denominator_area_um2: element.area_um2,
            view_contributions,
        });
    }

    let selected_view_budget_met = case.selected_view_ids.len() <= case.max_selected_views;
    let status = if !provenance_complete {
        DensityStatus::Indeterminate
    } else if !candidate_pool_sufficient {
        DensityStatus::Infeasible
    } else if !selected_plan_sufficient || !selected_view_budget_met {
        DensityStatus::BudgetInfeasible
    } else {
        DensityStatus::Feasible
    };

    let mut reason_codes = Vec::new();
    if !provenance_complete {
        reason_codes.push("RAY_COUNT_PROVENANCE_UNVERIFIED".to_owned());
    }
    if !candidate_pool_sufficient {
        reason_codes.push("CANDIDATE_POOL_DENSITY_OR_VIEW_SHORTFALL".to_owned());
    }
    if !selected_plan_sufficient {
        reason_codes.push("SELECTED_PLAN_DENSITY_OR_VIEW_SHORTFALL".to_owned());
    }
    if !selected_view_budget_met {
        reason_codes.push("SELECTED_VIEW_BUDGET_EXCEEDED".to_owned());
    }
    reason_codes.sort();
    reason_codes.dedup();

    Ok(CaseReceipt {
        case_id: case.case_id.clone(),
        status,
        reason_codes,
        candidate_pool_sufficient,
        selected_plan_sufficient,
        selected_view_budget_met,
        provenance_complete,
        candidate_view_count: case.candidate_views.len(),
        selected_view_count: case.selected_view_ids.len(),
        max_selected_views: case.max_selected_views,
        elements,
    })
}

fn invalid_case_receipt(case: &SamplingCase, reason: String) -> CaseReceipt {
    CaseReceipt {
        case_id: case.case_id.clone(),
        status: DensityStatus::InvalidInput,
        reason_codes: vec![reason],
        candidate_pool_sufficient: false,
        selected_plan_sufficient: false,
        selected_view_budget_met: false,
        provenance_complete: false,
        candidate_view_count: case.candidate_views.len(),
        selected_view_count: case.selected_view_ids.len(),
        max_selected_views: case.max_selected_views,
        elements: Vec::new(),
    }
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|value| value.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("SAMPLING_CORPUS_PARSE_FAILED:{error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.len() != CASE_COUNT
    {
        return Err("SAMPLING_CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut case_receipts = Vec::with_capacity(corpus.cases.len());
    let mut expected_outcome_matches = 0_usize;
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("DUPLICATE_SAMPLING_CASE_ID".to_owned());
        }
        let receipt = match evaluate_case(case) {
            Ok(receipt) => receipt,
            Err(reason) => invalid_case_receipt(case, reason),
        };
        if receipt.status == case.expected_status {
            expected_outcome_matches += 1;
        }
        case_receipts.push(receipt);
    }

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        fixture_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: case_receipts.len(),
        expected_outcome_matches,
        expected_outcome_mismatches: case_receipts.len() - expected_outcome_matches,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases: case_receipts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../docs/research/orbital-truss/fixtures/sampling-density-v0_1.json");

    #[test]
    fn frozen_sampling_density_corpus_matches_every_expected_status() {
        let receipt = audit_corpus(FIXTURE.as_bytes(), &"a".repeat(40)).unwrap();
        assert_eq!(receipt.case_count, CASE_COUNT);
        assert_eq!(
            receipt.expected_outcome_matches,
            CASE_COUNT,
            "case statuses: {:?}",
            receipt
                .cases
                .iter()
                .map(|case| (&case.case_id, case.status))
                .collect::<Vec<_>>()
        );
        assert_eq!(receipt.expected_outcome_mismatches, 0);
        assert!(!receipt.authority_granted);
        assert!(
            receipt
                .cases
                .iter()
                .all(|case| case.status != DensityStatus::Feasible || case.provenance_complete)
        );
    }

    #[test]
    fn tenfold_area_reduces_density_by_ten_at_equal_hit_count() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let a = evaluate_case(&corpus.cases[0]).unwrap().elements[0].clone();
        let b = evaluate_case(&corpus.cases[1]).unwrap().elements[0].clone();
        assert_eq!(
            a.selected_total_intersections,
            b.selected_total_intersections
        );
        assert_eq!(b.area_um2, a.area_um2 * 10);
    }

    #[test]
    fn density_rational_numerator_uses_micro_rays_per_square_millimetre_units() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let report = evaluate_case(&corpus.cases[0]).unwrap().elements[0].clone();
        assert_eq!(report.selected_total_intersections, 1);
        assert_eq!(
            report.selected_density_rational_numerator_micro_mm2,
            "1000000000000"
        );
        assert_eq!(report.density_rational_denominator_area_um2, 1_000_000);
    }

    #[test]
    fn requested_contributing_views_above_candidate_pool_is_infeasible_not_invalid() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let case = &corpus.cases[12];
        let receipt = evaluate_case(case).unwrap();
        assert_eq!(receipt.status, DensityStatus::Infeasible);
        assert!(!receipt.candidate_pool_sufficient);
        assert!(receipt.selected_view_budget_met);
        assert_eq!(receipt.candidate_view_count, 3);
        assert_eq!(receipt.max_selected_views, 9);
        assert_eq!(receipt.elements[0].candidate_contributing_views, 1);
        assert_eq!(receipt.elements[0].minimum_contributing_views, 4);
    }

    #[test]
    fn element_order_and_view_row_order_do_not_change_density_receipt() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).unwrap();
        let original = evaluate_case(&corpus.cases[11]).unwrap();
        let mut permuted = corpus.cases[11].clone();
        permuted.elements.reverse();
        for element in &mut permuted.elements {
            element.ray_counts.reverse();
        }
        let actual = evaluate_case(&permuted).unwrap();
        assert_eq!(actual, original);
    }
}

pub mod ray_geometry;
