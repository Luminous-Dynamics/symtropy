use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const SEARCH_NODE_LIMIT: u64 = 200_000;
const MAX_CAPACITY_TONNES: u32 = 100_000;
const PPM: u64 = 1_000_000;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AssayState {
    Quantified,
    BelowDetection,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PlanStatus {
    Feasible,
    NoFeasibleBlend,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum OracleStatus {
    Optimal,
    NoFeasibleBlend,
    IndeterminateSearchLimit,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    search_node_limit: u64,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    required_valuable_analyte_id: String,
    required_contaminant_analyte_id: String,
    feed_capacity_tonnes: u32,
    minimum_feed_tonnes: u32,
    minimum_grade_ppm: u64,
    maximum_contaminant_ppm: u64,
    maximum_energy_units: u64,
    maximum_water_units: u64,
    lots: Vec<Lot>,
    expected_lot_block_reasons: BTreeMap<String, Vec<String>>,
    expected_baseline_status: PlanStatus,
    expected_baseline_selection: Vec<LotTake>,
    expected_oracle_status: OracleStatus,
    expected_oracle_selection: Vec<LotTake>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Lot {
    lot_id: String,
    valuable_analyte_id: String,
    contaminant_analyte_id: String,
    available_tonnes: u32,
    sample_lot_id: String,
    assay_state: AssayState,
    detection_limit_ppm: u64,
    grade_lower_ppm: u64,
    grade_upper_ppm: u64,
    contaminant_lower_ppm: u64,
    contaminant_upper_ppm: u64,
    recovery_lower_ppm: u64,
    recovery_upper_ppm: u64,
    specific_energy_lower: u32,
    specific_energy_upper: u32,
    specific_water_lower: u32,
    specific_water_upper: u32,
    analytical_qc_pass: bool,
    dry_basis: bool,
    source_record_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LotTake {
    lot_id: String,
    tonnes: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ChoiceMetrics {
    total_feed_tonnes: u64,
    conservative_grade_ppm: u64,
    conservative_contaminant_ppm: u64,
    recovered_lower_score: u64,
    worst_case_energy_units: u64,
    worst_case_water_units: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Choice {
    selection: Vec<LotTake>,
    metrics: ChoiceMetrics,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    lot_block_reasons: BTreeMap<String, Vec<String>>,
    baseline_status: PlanStatus,
    baseline: Choice,
    baseline_independently_validated: bool,
    oracle_status: OracleStatus,
    oracle: Choice,
    oracle_explored_candidates: u64,
    oracle_search_limit: u64,
    oracle_independently_validated: bool,
    expected_control_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    result: String,
    validator_version: String,
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    source_commit: String,
    corpus_sha256: String,
    scenario_count: usize,
    expected_controls_matched: usize,
    search_node_limit: u64,
    scenarios: Vec<ScenarioResult>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}
fn option_nonempty(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|value| nonempty(value))
}

fn unique_nonempty(values: &[String], at: &str) -> Check<()> {
    if values.iter().any(|value| !nonempty(value)) {
        return Err(format!("{at}: identifiers must be non-empty"));
    }
    if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(format!("{at}: duplicate identifiers are not allowed"));
    }
    Ok(())
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.suite_id != "ORE_FEED_BLEND_ROBUSTNESS_BENCHMARK"
        || !nonempty(&corpus.suite_version)
        || corpus.evidence_class != "synthetic"
        || !nonempty(&corpus.claim_ceiling)
        || corpus.scenarios.is_empty()
        || corpus.search_node_limit == 0
        || corpus.search_node_limit > SEARCH_NODE_LIMIT
    {
        return Err("invalid corpus metadata or search limit".to_owned());
    }

    let scenario_ids: Vec<_> = corpus
        .scenarios
        .iter()
        .map(|s| s.scenario_id.clone())
        .collect();
    unique_nonempty(&scenario_ids, "corpus.scenario_id")?;
    for scenario in &corpus.scenarios {
        if !nonempty(&scenario.scenario_id)
            || !nonempty(&scenario.required_valuable_analyte_id)
            || !nonempty(&scenario.required_contaminant_analyte_id)
            || scenario.feed_capacity_tonnes == 0
            || scenario.feed_capacity_tonnes > MAX_CAPACITY_TONNES
            || scenario.minimum_feed_tonnes == 0
            || scenario.minimum_feed_tonnes > scenario.feed_capacity_tonnes
            || scenario.minimum_grade_ppm > PPM
            || scenario.maximum_contaminant_ppm > PPM
            || scenario.lots.is_empty()
            || scenario.lots.len() > 64
        {
            return Err(format!(
                "{}: invalid scenario envelope",
                scenario.scenario_id
            ));
        }

        let lot_ids: Vec<_> = scenario.lots.iter().map(|lot| lot.lot_id.clone()).collect();
        unique_nonempty(&lot_ids, &format!("{}.lot_id", scenario.scenario_id))?;
        for lot in &scenario.lots {
            if !nonempty(&lot.lot_id)
                || !nonempty(&lot.sample_lot_id)
                || lot.available_tonnes > MAX_CAPACITY_TONNES
            {
                return Err(format!(
                    "{}: invalid lot identity or availability",
                    scenario.scenario_id
                ));
            }
        }
        validate_expected_selection(
            scenario,
            &scenario.expected_baseline_selection,
            "expected_baseline_selection",
        )?;
        validate_expected_selection(
            scenario,
            &scenario.expected_oracle_selection,
            "expected_oracle_selection",
        )?;
        for (lot_id, reasons) in &scenario.expected_lot_block_reasons {
            if !lot_ids.contains(lot_id) || reasons.is_empty() {
                return Err(format!(
                    "{}: invalid expected lot-block record",
                    scenario.scenario_id
                ));
            }
            unique_nonempty(
                reasons,
                &format!(
                    "{}.expected_lot_block_reasons.{lot_id}",
                    scenario.scenario_id
                ),
            )?;
        }
    }
    Ok(())
}

fn validate_expected_selection(
    scenario: &Scenario,
    selection: &[LotTake],
    field: &str,
) -> Check<()> {
    let mut previous: Option<&str> = None;
    for take in selection {
        if take.tonnes == 0 || !scenario.lots.iter().any(|lot| lot.lot_id == take.lot_id) {
            return Err(format!(
                "{}.{}: invalid expected lot take",
                scenario.scenario_id, field
            ));
        }
        if previous.is_some_and(|id| id >= take.lot_id.as_str()) {
            return Err(format!(
                "{}.{}: IDs must be unique and sorted",
                scenario.scenario_id, field
            ));
        }
        previous = Some(&take.lot_id);
    }
    Ok(())
}

fn lot_block_reasons(lot: &Lot) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    if !lot.analytical_qc_pass {
        reasons.insert("ASSAY_QC_UNRESOLVED".to_owned());
    }
    if lot.sample_lot_id != lot.lot_id {
        reasons.insert("SAMPLE_LOT_MISMATCH".to_owned());
    }
    if !lot.dry_basis {
        reasons.insert("ASSAY_BASIS_NOT_DRY".to_owned());
    }
    if !option_nonempty(&lot.source_record_ref) {
        reasons.insert("ASSAY_SOURCE_MISSING".to_owned());
    }
    if lot.grade_lower_ppm > lot.grade_upper_ppm || lot.grade_upper_ppm > PPM {
        reasons.insert("GRADE_INTERVAL_INVALID".to_owned());
    }
    if lot.contaminant_lower_ppm > lot.contaminant_upper_ppm || lot.contaminant_upper_ppm > PPM {
        reasons.insert("CONTAMINANT_INTERVAL_INVALID".to_owned());
    }
    match lot.assay_state {
        AssayState::Quantified if lot.detection_limit_ppm != 0 => {
            reasons.insert("DETECTION_STATE_INVALID".to_owned());
        }
        AssayState::Quantified => {}
        AssayState::BelowDetection
            if lot.detection_limit_ppm == 0
                || lot.grade_lower_ppm != 0
                || lot.grade_upper_ppm == 0
                || lot.grade_upper_ppm > lot.detection_limit_ppm =>
        {
            reasons.insert("DETECTION_LIMIT_BOUNDS_INVALID".to_owned());
        }
        AssayState::BelowDetection => {}
    }
    if lot.recovery_lower_ppm > lot.recovery_upper_ppm || lot.recovery_upper_ppm > PPM {
        reasons.insert("RECOVERY_INTERVAL_INVALID".to_owned());
    }
    if lot.specific_energy_lower > lot.specific_energy_upper {
        reasons.insert("ENERGY_INTERVAL_INVALID".to_owned());
    }
    if lot.specific_water_lower > lot.specific_water_upper {
        reasons.insert("WATER_INTERVAL_INVALID".to_owned());
    }
    reasons.into_iter().collect()
}

fn lot_block_reasons_for(scenario: &Scenario, lot: &Lot) -> Vec<String> {
    let mut reasons: BTreeSet<String> = lot_block_reasons(lot).into_iter().collect();
    if lot.valuable_analyte_id != scenario.required_valuable_analyte_id {
        reasons.insert("VALUABLE_ANALYTE_MISMATCH".to_owned());
    }
    if lot.contaminant_analyte_id != scenario.required_contaminant_analyte_id {
        reasons.insert("CONTAMINANT_ANALYTE_MISMATCH".to_owned());
    }
    reasons.into_iter().collect()
}

fn all_lot_block_reasons(scenario: &Scenario) -> BTreeMap<String, Vec<String>> {
    scenario
        .lots
        .iter()
        .filter_map(|lot| {
            let reasons = lot_block_reasons_for(scenario, lot);
            (!reasons.is_empty()).then(|| (lot.lot_id.clone(), reasons))
        })
        .collect()
}

fn lot_index(scenario: &Scenario, lot_id: &str) -> Option<usize> {
    scenario.lots.iter().position(|lot| lot.lot_id == lot_id)
}

fn selection_from_quantities(scenario: &Scenario, quantities: &[u32]) -> Vec<LotTake> {
    let mut selection: Vec<_> = scenario
        .lots
        .iter()
        .zip(quantities)
        .filter_map(|(lot, tonnes)| {
            (*tonnes > 0).then(|| LotTake {
                lot_id: lot.lot_id.clone(),
                tonnes: *tonnes,
            })
        })
        .collect();
    selection.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
    selection
}

fn metrics_for_quantities(
    s: &Scenario,
    quantities: &[u32],
    enforce_minimum: bool,
) -> Option<ChoiceMetrics> {
    if quantities.len() != s.lots.len() {
        return None;
    }
    let blocked = all_lot_block_reasons(s);
    let mut total = 0_u64;
    let mut grade_sum = 0_u64;
    let mut contaminant_sum = 0_u64;
    let mut recovery_score = 0_u64;
    let mut energy = 0_u64;
    let mut water = 0_u64;

    for (lot, quantity) in s.lots.iter().zip(quantities) {
        if *quantity == 0 {
            continue;
        }
        if *quantity > lot.available_tonnes || blocked.contains_key(&lot.lot_id) {
            return None;
        }
        let q = u64::from(*quantity);
        total = total.checked_add(q)?;
        grade_sum = grade_sum.checked_add(q.checked_mul(lot.grade_lower_ppm)?)?;
        contaminant_sum = contaminant_sum.checked_add(q.checked_mul(lot.contaminant_upper_ppm)?)?;
        recovery_score = recovery_score.checked_add(
            q.checked_mul(lot.grade_lower_ppm)?
                .checked_mul(lot.recovery_lower_ppm)?,
        )?;
        energy = energy.checked_add(q.checked_mul(u64::from(lot.specific_energy_upper))?)?;
        water = water.checked_add(q.checked_mul(u64::from(lot.specific_water_upper))?)?;
    }

    if total == 0
        || total > u64::from(s.feed_capacity_tonnes)
        || (enforce_minimum && total < u64::from(s.minimum_feed_tonnes))
        || grade_sum < total.checked_mul(s.minimum_grade_ppm)?
        || contaminant_sum > total.checked_mul(s.maximum_contaminant_ppm)?
        || energy > s.maximum_energy_units
        || water > s.maximum_water_units
    {
        return None;
    }

    Some(ChoiceMetrics {
        total_feed_tonnes: total,
        conservative_grade_ppm: grade_sum / total,
        conservative_contaminant_ppm: contaminant_sum.checked_add(total - 1)?.checked_div(total)?,
        recovered_lower_score: recovery_score,
        worst_case_energy_units: energy,
        worst_case_water_units: water,
    })
}

fn choice_from_quantities(s: &Scenario, quantities: &[u32]) -> Option<Choice> {
    Some(Choice {
        selection: selection_from_quantities(s, quantities),
        metrics: metrics_for_quantities(s, quantities, true)?,
    })
}

fn partial_admissible(s: &Scenario, quantities: &[u32]) -> bool {
    metrics_for_quantities(s, quantities, false).is_some()
}

fn greedy_high_grade_baseline(s: &Scenario) -> Choice {
    let blocked = all_lot_block_reasons(s);
    let mut order: Vec<usize> = (0..s.lots.len())
        .filter(|i| !blocked.contains_key(&s.lots[*i].lot_id))
        .collect();
    order.sort_by(|a, b| {
        let x = &s.lots[*a];
        let y = &s.lots[*b];
        (y.grade_lower_ppm + y.grade_upper_ppm)
            .cmp(&(x.grade_lower_ppm + x.grade_upper_ppm))
            .then_with(|| x.lot_id.cmp(&y.lot_id))
    });
    let mut quantities = vec![0_u32; s.lots.len()];
    for index in order {
        let lot = &s.lots[index];
        loop {
            let total: u64 = quantities.iter().map(|q| u64::from(*q)).sum();
            if quantities[index] >= lot.available_tonnes
                || total >= u64::from(s.feed_capacity_tonnes)
            {
                break;
            }
            quantities[index] += 1;
            if !partial_admissible(s, &quantities) {
                quantities[index] -= 1;
                break;
            }
        }
    }
    choice_from_quantities(s, &quantities).unwrap_or_default()
}

fn signature(choice: &Choice) -> String {
    choice
        .selection
        .iter()
        .map(|take| format!("{}:{}", take.lot_id, take.tonnes))
        .collect::<Vec<_>>()
        .join("|")
}

fn compare_choices(a: &Choice, b: &Choice) -> Ordering {
    b.metrics
        .recovered_lower_score
        .cmp(&a.metrics.recovered_lower_score)
        .then_with(|| {
            a.metrics
                .worst_case_energy_units
                .cmp(&b.metrics.worst_case_energy_units)
        })
        .then_with(|| {
            a.metrics
                .worst_case_water_units
                .cmp(&b.metrics.worst_case_water_units)
        })
        .then_with(|| {
            a.metrics
                .total_feed_tonnes
                .cmp(&b.metrics.total_feed_tonnes)
        })
        .then_with(|| signature(a).cmp(&signature(b)))
}

struct Search<'a> {
    scenario: &'a Scenario,
    quantities: Vec<u32>,
    nodes: u64,
    limit: u64,
    exhausted: bool,
    best: Option<Choice>,
}

impl Search<'_> {
    fn visit(&mut self, index: usize) {
        if self.exhausted {
            return;
        }
        if index == self.scenario.lots.len() {
            if self.nodes >= self.limit {
                self.exhausted = true;
                return;
            }
            self.nodes += 1;
            if let Some(candidate) = choice_from_quantities(self.scenario, &self.quantities) {
                if self
                    .best
                    .as_ref()
                    .is_none_or(|best| compare_choices(&candidate, best) == Ordering::Less)
                {
                    self.best = Some(candidate);
                }
            }
            return;
        }
        let lot = &self.scenario.lots[index];
        let maximum = if lot_block_reasons_for(self.scenario, lot).is_empty() {
            lot.available_tonnes.min(self.scenario.feed_capacity_tonnes)
        } else {
            0
        };
        for tonnes in 0..=maximum {
            self.quantities[index] = tonnes;
            self.visit(index + 1);
            if self.exhausted {
                return;
            }
        }
        self.quantities[index] = 0;
    }
}

fn exhaustive_oracle(s: &Scenario, limit: u64) -> (Option<Choice>, u64, bool) {
    let mut search = Search {
        scenario: s,
        quantities: vec![0; s.lots.len()],
        nodes: 0,
        limit,
        exhausted: false,
        best: None,
    };
    search.visit(0);
    (search.best, search.nodes, search.exhausted)
}

fn actual_oracle_status(best: &Option<Choice>, exhausted: bool) -> OracleStatus {
    if exhausted {
        OracleStatus::IndeterminateSearchLimit
    } else if best.is_some() {
        OracleStatus::Optimal
    } else {
        OracleStatus::NoFeasibleBlend
    }
}

fn baseline_status(choice: &Choice) -> PlanStatus {
    if choice.selection.is_empty() {
        PlanStatus::NoFeasibleBlend
    } else {
        PlanStatus::Feasible
    }
}

fn independently_recompute(s: &Scenario, selection: &[LotTake]) -> Check<ChoiceMetrics> {
    let mut seen = BTreeSet::new();
    let mut total = 0_u64;
    let mut grade_sum = 0_u64;
    let mut contaminant_sum = 0_u64;
    let mut recovery_score = 0_u64;
    let mut energy = 0_u64;
    let mut water = 0_u64;

    for take in selection {
        if take.tonnes == 0 || !seen.insert(take.lot_id.clone()) {
            return Err("selection has zero tonnes or duplicate lot IDs".to_owned());
        }
        let index = lot_index(s, &take.lot_id)
            .ok_or_else(|| format!("unknown selected lot {}", take.lot_id))?;
        let lot = &s.lots[index];
        if take.tonnes > lot.available_tonnes || !lot_block_reasons_for(s, lot).is_empty() {
            return Err(format!(
                "selected lot is unavailable or inadmissible: {}",
                lot.lot_id
            ));
        }
        let mass = u64::from(take.tonnes);
        total = total
            .checked_add(mass)
            .ok_or_else(|| "mass sum overflow".to_owned())?;
        grade_sum = grade_sum
            .checked_add(
                mass.checked_mul(lot.grade_lower_ppm)
                    .ok_or_else(|| "grade numerator overflow".to_owned())?,
            )
            .ok_or_else(|| "grade sum overflow".to_owned())?;
        contaminant_sum = contaminant_sum
            .checked_add(
                mass.checked_mul(lot.contaminant_upper_ppm)
                    .ok_or_else(|| "contaminant numerator overflow".to_owned())?,
            )
            .ok_or_else(|| "contaminant sum overflow".to_owned())?;
        recovery_score = recovery_score
            .checked_add(
                mass.checked_mul(lot.grade_lower_ppm)
                    .and_then(|x| x.checked_mul(lot.recovery_lower_ppm))
                    .ok_or_else(|| "recovery score overflow".to_owned())?,
            )
            .ok_or_else(|| "recovery sum overflow".to_owned())?;
        energy = energy
            .checked_add(mass * u64::from(lot.specific_energy_upper))
            .ok_or_else(|| "energy sum overflow".to_owned())?;
        water = water
            .checked_add(mass * u64::from(lot.specific_water_upper))
            .ok_or_else(|| "water sum overflow".to_owned())?;
    }

    if total < u64::from(s.minimum_feed_tonnes)
        || total > u64::from(s.feed_capacity_tonnes)
        || grade_sum < total * s.minimum_grade_ppm
        || contaminant_sum > total * s.maximum_contaminant_ppm
        || energy > s.maximum_energy_units
        || water > s.maximum_water_units
    {
        return Err("independent reconstruction found a hard-constraint violation".to_owned());
    }

    Ok(ChoiceMetrics {
        total_feed_tonnes: total,
        conservative_grade_ppm: grade_sum / total,
        conservative_contaminant_ppm: (contaminant_sum + total - 1) / total,
        recovered_lower_score: recovery_score,
        worst_case_energy_units: energy,
        worst_case_water_units: water,
    })
}

fn independently_validate_choice(s: &Scenario, choice: &Choice) -> Check<()> {
    if choice.selection.is_empty() {
        return if choice.metrics == ChoiceMetrics::default() {
            Ok(())
        } else {
            Err("empty selection claims non-zero metrics".to_owned())
        };
    }
    let recomputed = independently_recompute(s, &choice.selection)?;
    if recomputed != choice.metrics {
        return Err("independently recomputed metrics differ from selector receipt".to_owned());
    }
    Ok(())
}

fn analyze_scenario(s: &Scenario, search_limit: u64) -> Check<ScenarioResult> {
    let lot_block_reasons = all_lot_block_reasons(s);
    let baseline = greedy_high_grade_baseline(s);
    independently_validate_choice(s, &baseline)?;
    let (best, nodes, exhausted) = exhaustive_oracle(s, search_limit);
    let oracle = best.clone().unwrap_or_default();
    independently_validate_choice(s, &oracle)?;
    let status = actual_oracle_status(&best, exhausted);
    let matched = lot_block_reasons == s.expected_lot_block_reasons
        && baseline_status(&baseline) == s.expected_baseline_status
        && baseline.selection == s.expected_baseline_selection
        && status == s.expected_oracle_status
        && oracle.selection == s.expected_oracle_selection;
    Ok(ScenarioResult {
        scenario_id: s.scenario_id.clone(),
        lot_block_reasons,
        baseline_status: baseline_status(&baseline),
        baseline,
        baseline_independently_validated: true,
        oracle_status: status,
        oracle,
        oracle_explored_candidates: nodes,
        oracle_search_limit: search_limit,
        oracle_independently_validated: true,
        expected_control_matched: matched,
    })
}

fn source_commit_from_args(args: &[String]) -> Check<String> {
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: ore-feed-blend-bench <corpus.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let value = args[3].to_ascii_lowercase();
    if value.len() != 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("--source-commit must be exactly 40 hexadecimal characters".to_owned());
    }
    Ok(value)
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: ore-feed-blend-bench <corpus.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = source_commit_from_args(&args)?;
    let bytes = fs::read(&args[1]).map_err(|e| format!("cannot read corpus: {e}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid corpus JSON: {e}"))?;
    validate_corpus(&corpus)?;
    let corpus_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let scenarios = corpus
        .scenarios
        .iter()
        .map(|scenario| analyze_scenario(scenario, corpus.search_node_limit))
        .collect::<Check<Vec<_>>>()?;
    let matched = scenarios
        .iter()
        .filter(|s| s.expected_control_matched)
        .count();
    let passed = matched == scenarios.len();
    let receipt = Receipt {
        result: if passed {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        validator_version: VERSION.to_owned(),
        suite_id: corpus.suite_id,
        suite_version: corpus.suite_version,
        evidence_class: corpus.evidence_class,
        source_commit,
        corpus_sha256,
        scenario_count: scenarios.len(),
        expected_controls_matched: matched,
        search_node_limit: corpus.search_node_limit,
        scenarios,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|e| format!("cannot serialize receipt: {e}"))?
    );
    Ok(if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn corpus() -> Corpus {
        serde_json::from_str(include_str!(
            "../../../docs/tech/ore-feed-blend-corpus-v0.1.json"
        ))
        .expect("frozen synthetic corpus must deserialize")
    }

    #[test]
    fn frozen_corpus_matches_all_known_answers() {
        let c = corpus();
        validate_corpus(&c).expect("corpus metadata must be valid");
        let results = c
            .scenarios
            .iter()
            .map(|s| analyze_scenario(s, c.search_node_limit))
            .collect::<Check<Vec<_>>>()
            .expect("all scenarios must be analysable");
        assert_eq!(12, results.len());
        assert!(results.iter().all(|r| r.expected_control_matched));
    }

    #[test]
    fn high_grade_first_can_miss_a_feasible_higher_recovery_blend() {
        let c = corpus();
        let s = c
            .scenarios
            .iter()
            .find(|s| s.scenario_id == "GRADE_FIRST_REGRESSION")
            .expect("regression fixture exists");
        let r = analyze_scenario(s, c.search_node_limit).expect("scenario must be analysable");
        assert_eq!(PlanStatus::NoFeasibleBlend, r.baseline_status);
        assert_eq!(OracleStatus::Optimal, r.oracle_status);
        assert_eq!(360_000_000_000, r.oracle.metrics.recovered_lower_score);
    }

    #[test]
    fn below_detection_is_an_interval_not_a_zero_measurement() {
        let c = corpus();
        let s = c
            .scenarios
            .iter()
            .find(|s| s.scenario_id == "BELOW_DETECTION_REMAINS_INTERVAL")
            .expect("below-detection fixture exists");
        let lot = s
            .lots
            .iter()
            .find(|lot| lot.lot_id == "B_BELOW_DETECTION_INTERVAL")
            .expect("below-detection lot exists");
        assert_eq!(AssayState::BelowDetection, lot.assay_state);
        assert_eq!(0, lot.grade_lower_ppm);
        assert_eq!(50_000, lot.grade_upper_ppm);
        assert!(!all_lot_block_reasons(s).contains_key(&lot.lot_id));
    }

    #[test]
    fn search_limit_never_mints_an_optimality_claim() {
        let c = corpus();
        let s = c
            .scenarios
            .iter()
            .find(|s| s.scenario_id == "GRADE_FIRST_REGRESSION")
            .expect("regression fixture exists");
        let (best, nodes, exhausted) = exhaustive_oracle(s, 1);
        assert_eq!(1, nodes);
        assert!(exhausted);
        assert_eq!(
            OracleStatus::IndeterminateSearchLimit,
            actual_oracle_status(&best, exhausted)
        );
    }

    #[test]
    fn independent_checker_rejects_double_counted_lots() {
        let c = corpus();
        let s = c
            .scenarios
            .iter()
            .find(|s| s.scenario_id == "SAMPLE_LOT_MISMATCH_EXCLUDED")
            .expect("lineage fixture exists");
        let duplicate = vec![
            LotTake {
                lot_id: "A_VALID".to_owned(),
                tonnes: 1,
            },
            LotTake {
                lot_id: "A_VALID".to_owned(),
                tonnes: 1,
            },
        ];
        assert!(independently_recompute(s, &duplicate).is_err());
    }

    #[test]
    fn unknown_json_fields_fail_closed() {
        let raw = include_str!("../../../docs/tech/ore-feed-blend-corpus-v0.1.json").replace(
            r#""suite_id":"#,
            r#""unexpected_authority":true,"suite_id":"#,
        );
        assert!(serde_json::from_str::<Corpus>(&raw).is_err());
    }
}
