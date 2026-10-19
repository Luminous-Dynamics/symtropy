use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const INPUT_SCHEMA_VERSION: &str = "water-process-closure-001-inputs-v1";
pub const ORACLE_SCHEMA_VERSION: &str = "water-process-closure-001-oracle-v1";
pub const CLAIM_CEILING: &str = "WaterProcessAccountingContractOnly";
pub const EXPECTED_CASE_COUNT: usize = 16;

#[derive(Clone, Debug, Deserialize)]
pub struct InputCorpus {
    pub schema_version: String,
    pub qualification_status: String,
    pub claim_ceiling: String,
    pub cases: Vec<ProcessCase>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProcessCase {
    pub case_id: String,
    pub profile: String,
    pub unit_status: String,
    pub time_grid_aligned: bool,
    pub input_streams_complete: bool,
    pub internal_recycle_counted_as_external: bool,
    pub water_balance: WaterBalanceInput,
    pub mass_balance: MassBalanceInput,
    pub tracer_balance: TracerBalanceInput,
    pub residual: ResidualInput,
    pub cycle: CycleInput,
    pub utilities: UtilitiesInput,
    pub quality_gate: QualityGateInput,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WaterBalanceInput {
    pub unit_status: String,
    pub inflow_ml: i64,
    pub opening_storage_ml: i64,
    pub treated_effluent_ml: i64,
    pub reuse_ml: i64,
    pub backwash_ml: i64,
    pub sludge_water_ml: i64,
    pub reject_ml: i64,
    pub declared_loss_ml: i64,
    pub closing_storage_ml: i64,
    pub tolerance_ml: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Bounds {
    pub lower: i64,
    pub upper: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MassOutput {
    pub stream_id: String,
    pub lower_g: i64,
    pub upper_g: i64,
    pub stream_class: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MassBalanceInput {
    pub unit_status: String,
    pub streams_complete: bool,
    pub input_mass_g: Bounds,
    pub outputs: Vec<MassOutput>,
    pub tolerance_g: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TracerBalanceInput {
    pub applicable: bool,
    pub input_mass_mg: Option<i64>,
    pub output_mass_mg: Option<i64>,
    pub concentration_only_claim: bool,
    pub reaction_expected: bool,
    pub reaction_model_available: bool,
    pub closed_boundary: bool,
    pub tolerance_mg: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ResidualInput {
    pub required: bool,
    pub route_state: String,
    pub recovery_claimed: bool,
    pub recovery_qualified: bool,
    pub recovered_mass_g: i64,
    pub recovered_output_evidence: bool,
    pub permission_known: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CycleInput {
    pub complete: bool,
    pub consumables_renewed: bool,
    pub backwash_accounted: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UtilitiesInput {
    pub complete: bool,
    pub electricity_wh: Option<i64>,
    pub heat_wh: Option<i64>,
    pub process_minutes: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QualityGateInput {
    pub required: bool,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Oracle {
    pub schema_version: String,
    pub qualification_status: String,
    pub claim_ceiling: String,
    pub independently_hand_calculated: bool,
    pub results: Vec<CaseResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub water_balance_status: WaterStatus,
    pub material_mass_status: MassStatus,
    pub tracer_status: TracerStatus,
    pub lifecycle_status: LifecycleStatus,
    pub overall_disposition: OverallDisposition,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum WaterStatus {
    ExactBalanced,
    ExactUnbalanced,
    Invalid,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum MassStatus {
    ExactBalanced,
    ExactUnbalanced,
    PossibleWithUncertainty,
    ImpossibleWithinBounds,
    Incomplete,
    Invalid,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum TracerStatus {
    NotApplicable,
    ExactBalanced,
    Unbalanced,
    NotAssessable,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LifecycleStatus {
    ClosedForAccounting,
    Indeterminate,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum OverallDisposition {
    AccountingOnly,
    Unbalanced,
    Invalid,
    Indeterminate,
    Incomplete,
}

pub fn evaluate_corpus(corpus: &InputCorpus) -> Result<Vec<CaseResult>, String> {
    if corpus.schema_version != INPUT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported input schema version: {}",
            corpus.schema_version
        ));
    }
    if corpus.qualification_status != "NotRun" || corpus.claim_ceiling != CLAIM_CEILING {
        return Err("input corpus claim ceiling or qualification state is invalid".to_owned());
    }
    if corpus.cases.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} cases, got {}",
            corpus.cases.len()
        ));
    }
    let mut ids = HashSet::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err(format!("duplicate case ID: {}", case.case_id));
        }
    }
    let mut rows: Vec<CaseResult> = corpus.cases.iter().map(evaluate_case).collect();
    rows.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    Ok(rows)
}

pub fn evaluate_case(case: &ProcessCase) -> CaseResult {
    let water = evaluate_water(case);
    let mass = evaluate_mass(case);
    let tracer = evaluate_tracer(&case.tracer_balance);
    let lifecycle = evaluate_lifecycle(case);
    let overall = overall_disposition(water, mass, tracer, lifecycle);
    CaseResult {
        case_id: case.case_id.clone(),
        water_balance_status: water,
        material_mass_status: mass,
        tracer_status: tracer,
        lifecycle_status: lifecycle,
        overall_disposition: overall,
    }
}

fn evaluate_water(case: &ProcessCase) -> WaterStatus {
    let b = &case.water_balance;
    let quantities = [
        b.inflow_ml,
        b.opening_storage_ml,
        b.treated_effluent_ml,
        b.reuse_ml,
        b.backwash_ml,
        b.sludge_water_ml,
        b.reject_ml,
        b.declared_loss_ml,
        b.closing_storage_ml,
        b.tolerance_ml,
    ];
    if case.unit_status != "valid"
        || b.unit_status != "valid"
        || quantities.iter().any(|value| *value < 0)
    {
        return WaterStatus::Invalid;
    }
    if case.internal_recycle_counted_as_external {
        return WaterStatus::Invalid;
    }
    if !case.time_grid_aligned || !case.input_streams_complete {
        return WaterStatus::Incomplete;
    }
    let input = i128::from(b.inflow_ml) + i128::from(b.opening_storage_ml);
    let output = [
        b.treated_effluent_ml,
        b.reuse_ml,
        b.backwash_ml,
        b.sludge_water_ml,
        b.reject_ml,
        b.declared_loss_ml,
        b.closing_storage_ml,
    ]
    .iter()
    .map(|value| i128::from(*value))
    .sum::<i128>();
    if (input - output).abs() <= i128::from(b.tolerance_ml) {
        WaterStatus::ExactBalanced
    } else {
        WaterStatus::ExactUnbalanced
    }
}

fn evaluate_mass(case: &ProcessCase) -> MassStatus {
    let b = &case.mass_balance;
    if case.unit_status != "valid" || b.unit_status != "valid" {
        return MassStatus::Invalid;
    }
    if !case.input_streams_complete || !b.streams_complete {
        return MassStatus::Incomplete;
    }
    if b.tolerance_g < 0
        || b.input_mass_g.lower < 0
        || b.input_mass_g.upper < b.input_mass_g.lower
        || b.outputs
            .iter()
            .any(|o| o.stream_id.trim().is_empty() || o.lower_g < 0 || o.upper_g < o.lower_g)
    {
        return MassStatus::Invalid;
    }
    let mut stream_ids = HashSet::with_capacity(b.outputs.len());
    if b.outputs
        .iter()
        .any(|o| !stream_ids.insert(o.stream_id.as_str()))
    {
        return MassStatus::Invalid;
    }

    let output_lower = b
        .outputs
        .iter()
        .map(|o| i128::from(o.lower_g))
        .sum::<i128>();
    let output_upper = b
        .outputs
        .iter()
        .map(|o| i128::from(o.upper_g))
        .sum::<i128>();
    let input_lower = i128::from(b.input_mass_g.lower);
    let input_upper = i128::from(b.input_mass_g.upper);
    let tolerance = i128::from(b.tolerance_g);

    let input_is_point = input_lower == input_upper;
    let output_is_point = output_lower == output_upper;
    if input_is_point && output_is_point {
        return if (input_lower - output_lower).abs() <= tolerance {
            MassStatus::ExactBalanced
        } else {
            MassStatus::ExactUnbalanced
        };
    }

    let intervals_overlap_with_tolerance =
        input_lower <= output_upper + tolerance && output_lower <= input_upper + tolerance;
    if intervals_overlap_with_tolerance {
        MassStatus::PossibleWithUncertainty
    } else {
        MassStatus::ImpossibleWithinBounds
    }
}

fn evaluate_tracer(b: &TracerBalanceInput) -> TracerStatus {
    if !b.applicable {
        return TracerStatus::NotApplicable;
    }
    if b.tolerance_mg < 0
        || b.concentration_only_claim
        || !b.closed_boundary
        || (b.reaction_expected && !b.reaction_model_available)
    {
        return TracerStatus::NotAssessable;
    }
    let (Some(input), Some(output)) = (b.input_mass_mg, b.output_mass_mg) else {
        return TracerStatus::NotAssessable;
    };
    if input < 0 || output < 0 {
        return TracerStatus::NotAssessable;
    }
    if (i128::from(input) - i128::from(output)).abs() <= i128::from(b.tolerance_mg) {
        TracerStatus::ExactBalanced
    } else {
        TracerStatus::Unbalanced
    }
}

fn evaluate_lifecycle(case: &ProcessCase) -> LifecycleStatus {
    let cycle = &case.cycle;
    let utilities = &case.utilities;
    let residual = &case.residual;
    if !cycle.complete
        || !cycle.consumables_renewed
        || !cycle.backwash_accounted
        || !utilities.complete
        || utilities.electricity_wh.is_none()
        || utilities.heat_wh.is_none()
        || utilities.electricity_wh.is_some_and(|v| v < 0)
        || utilities.heat_wh.is_some_and(|v| v < 0)
        || utilities.process_minutes < 0
    {
        return LifecycleStatus::Incomplete;
    }
    if residual.recovered_mass_g < 0 {
        return LifecycleStatus::Indeterminate;
    }
    if residual.required {
        match residual.route_state.as_str() {
            "missing" | "not_applicable" => return LifecycleStatus::Incomplete,
            "unqualified" => return LifecycleStatus::Indeterminate,
            "qualified" => {}
            _ => return LifecycleStatus::Indeterminate,
        }
        if !residual.permission_known
            || (residual.recovery_claimed
                && (!residual.recovery_qualified || !residual.recovered_output_evidence))
            || (residual.recovery_qualified && !residual.recovered_output_evidence)
            || (residual.recovered_mass_g > 0 && !residual.recovered_output_evidence)
        {
            return LifecycleStatus::Indeterminate;
        }
    } else if residual.recovery_claimed && !residual.recovered_output_evidence {
        return LifecycleStatus::Indeterminate;
    }
    if case.quality_gate.required && case.quality_gate.status != "cleared" {
        return LifecycleStatus::Indeterminate;
    }
    LifecycleStatus::ClosedForAccounting
}

fn overall_disposition(
    water: WaterStatus,
    mass: MassStatus,
    tracer: TracerStatus,
    lifecycle: LifecycleStatus,
) -> OverallDisposition {
    if water == WaterStatus::Invalid || mass == MassStatus::Invalid {
        return OverallDisposition::Invalid;
    }
    if water == WaterStatus::ExactUnbalanced
        || mass == MassStatus::ExactUnbalanced
        || mass == MassStatus::ImpossibleWithinBounds
        || tracer == TracerStatus::Unbalanced
    {
        return OverallDisposition::Unbalanced;
    }
    if water == WaterStatus::Incomplete
        || mass == MassStatus::Incomplete
        || lifecycle == LifecycleStatus::Incomplete
    {
        return OverallDisposition::Incomplete;
    }
    if mass == MassStatus::PossibleWithUncertainty
        || tracer == TracerStatus::NotAssessable
        || lifecycle == LifecycleStatus::Indeterminate
    {
        return OverallDisposition::Indeterminate;
    }
    OverallDisposition::AccountingOnly
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_slice;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name)
    }

    fn load_corpus() -> InputCorpus {
        from_slice(&fs::read(fixture_path("water-process-closure-001-inputs-v1.json")).unwrap())
            .unwrap()
    }

    fn load_oracle() -> Oracle {
        from_slice(&fs::read(fixture_path("water-process-closure-001-oracle-v1.json")).unwrap())
            .unwrap()
    }

    #[test]
    fn frozen_sixteen_case_corpus_matches_independent_oracle() {
        let corpus = load_corpus();
        let oracle = load_oracle();
        let actual = evaluate_corpus(&corpus).unwrap();
        assert_eq!(oracle.schema_version, ORACLE_SCHEMA_VERSION);
        assert_eq!(oracle.qualification_status, "NotRun");
        assert_eq!(oracle.claim_ceiling, CLAIM_CEILING);
        assert!(oracle.independently_hand_calculated);
        assert_eq!(actual.len(), EXPECTED_CASE_COUNT);
        assert_eq!(actual, oracle.results);
    }

    #[test]
    fn fixture_order_does_not_change_sorted_results() {
        let mut corpus = load_corpus();
        let forward = evaluate_corpus(&corpus).unwrap();
        corpus.cases.reverse();
        assert_eq!(forward, evaluate_corpus(&corpus).unwrap());
    }

    #[test]
    fn water_imbalance_cannot_be_hidden_by_balanced_solids() {
        let mut corpus = load_corpus();
        corpus.cases[1].water_balance.treated_effluent_ml = 8_999;
        let row = evaluate_case(&corpus.cases[1]);
        assert_eq!(row.water_balance_status, WaterStatus::ExactUnbalanced);
        assert_eq!(row.material_mass_status, MassStatus::ExactBalanced);
        assert_eq!(row.overall_disposition, OverallDisposition::Unbalanced);
    }

    #[test]
    fn internal_recycle_misclassified_as_external_invalidates_water_balance() {
        let mut corpus = load_corpus();
        corpus.cases[0].internal_recycle_counted_as_external = true;
        let row = evaluate_case(&corpus.cases[0]);
        assert_eq!(row.water_balance_status, WaterStatus::Invalid);
        assert_eq!(row.overall_disposition, OverallDisposition::Invalid);
    }

    #[test]
    fn widening_mass_intervals_weakens_not_strengthens_claim() {
        let mut corpus = load_corpus();
        corpus.cases[0].mass_balance.input_mass_g.lower = 900;
        corpus.cases[0].mass_balance.input_mass_g.upper = 1_100;
        let row = evaluate_case(&corpus.cases[0]);
        assert_eq!(
            row.material_mass_status,
            MassStatus::PossibleWithUncertainty
        );
        assert_eq!(row.overall_disposition, OverallDisposition::Indeterminate);
    }

    #[test]
    fn disjoint_mass_intervals_are_impossible_within_bounds() {
        let mut corpus = load_corpus();
        let b = &mut corpus.cases[0].mass_balance;
        b.input_mass_g.lower = 900;
        b.input_mass_g.upper = 1_000;
        b.outputs[0].lower_g = 1_100;
        b.outputs[0].upper_g = 1_200;
        let row = evaluate_case(&corpus.cases[0]);
        assert_eq!(row.material_mass_status, MassStatus::ImpossibleWithinBounds);
        assert_eq!(row.overall_disposition, OverallDisposition::Unbalanced);
    }

    #[test]
    fn concentration_is_not_a_tracer_mass_balance() {
        let mut corpus = load_corpus();
        corpus.cases[11].tracer_balance.input_mass_mg = None;
        corpus.cases[11].tracer_balance.output_mass_mg = None;
        corpus.cases[11].tracer_balance.concentration_only_claim = true;
        let row = evaluate_case(&corpus.cases[11]);
        assert_eq!(row.tracer_status, TracerStatus::NotAssessable);
        assert_eq!(row.overall_disposition, OverallDisposition::Indeterminate);
    }

    #[test]
    fn missing_utility_inventory_is_incomplete_not_zero_burden() {
        let mut corpus = load_corpus();
        corpus.cases[0].utilities.complete = false;
        corpus.cases[0].utilities.electricity_wh = None;
        corpus.cases[0].utilities.heat_wh = None;
        let row = evaluate_case(&corpus.cases[0]);
        assert_eq!(row.lifecycle_status, LifecycleStatus::Incomplete);
        assert_eq!(row.overall_disposition, OverallDisposition::Incomplete);
    }

    #[test]
    fn duplicate_case_ids_fail_closed_at_corpus_boundary() {
        let mut corpus = load_corpus();
        corpus.cases[1].case_id = corpus.cases[0].case_id.clone();
        assert!(
            evaluate_corpus(&corpus)
                .unwrap_err()
                .contains("duplicate case ID")
        );
    }

    #[test]
    fn negative_mass_interval_is_invalid_not_balanced() {
        let mut corpus = load_corpus();
        corpus.cases[0].mass_balance.outputs[0].lower_g = -1;
        let row = evaluate_case(&corpus.cases[0]);
        assert_eq!(row.material_mass_status, MassStatus::Invalid);
        assert_eq!(row.overall_disposition, OverallDisposition::Invalid);
    }
}
