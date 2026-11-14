use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    accounting_tolerance_ppm: u32,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    expected_disposition: Disposition,
    expected_reason_codes: Vec<String>,
    rock_mass: RockMass,
    discontinuities: Vec<Discontinuity>,
    product_blocks: Vec<ProductBlock>,
    material_streams: MaterialStreams,
    expected_assertions: Vec<String>,
    negative_control: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RockMass {
    mass_id: String,
    dimensions: Dimensions,
    evidence_class: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dimensions {
    length_mm: u64,
    width_mm: u64,
    height_mm: u64,
}

impl Dimensions {
    fn axes(&self) -> [u64; 3] {
        [self.length_mm, self.width_mm, self.height_mm]
    }

    fn volume_mm3(&self) -> Option<u128> {
        (self.length_mm as u128)
            .checked_mul(self.width_mm as u128)?
            .checked_mul(self.height_mm as u128)
    }

    fn all_positive(&self) -> bool {
        self.length_mm > 0 && self.width_mm > 0 && self.height_mm > 0
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CrossingPolicy {
    Forbidden,
    Permitted,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Discontinuity {
    discontinuity_id: String,
    axis: Axis,
    offset_mm: u64,
    crossing_policy: CrossingPolicy,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Origin {
    x_mm: u64,
    y_mm: u64,
    z_mm: u64,
}

impl Origin {
    fn axes(&self) -> [u64; 3] {
        [self.x_mm, self.y_mm, self.z_mm]
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum IntendedUse {
    Decorative,
    NonStructural,
    LoadBearing,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum QualityEvidence {
    Qualified,
    Observed,
    Estimated,
    Unknown,
    Synthetic,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductBlock {
    block_id: String,
    origin: Origin,
    dimensions: Dimensions,
    intended_use: IntendedUse,
    quality_evidence: QualityEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialStreams {
    recoverable_offcuts_mm3: u64,
    kerf_loss_mm3: u64,
    fines_and_unclassified_mm3: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    Pass,
    Reject,
}

#[derive(Debug, Serialize)]
struct Receipt {
    result: &'static str,
    validator_version: &'static str,
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    scenario_count: usize,
    expected_controls_matched: usize,
    corpus_sha256: String,
    source_commit: String,
    scenarios: Vec<ScenarioReceipt>,
}

#[derive(Debug, Serialize)]
struct ScenarioReceipt {
    scenario_id: String,
    mass_id: String,
    expected_disposition: Disposition,
    actual_disposition: Disposition,
    expected_reason_codes: Vec<String>,
    actual_reason_codes: Vec<String>,
    rock_volume_mm3: Option<u128>,
    product_volume_mm3: u128,
    declared_non_product_streams_mm3: u128,
    accounting_difference_mm3: Option<u128>,
    assertions: Vec<String>,
    negative_control: String,
    expected_control_matched: bool,
}

fn product_bounds(block: &ProductBlock) -> Option<([u64; 3], [u64; 3])> {
    let start = block.origin.axes();
    let size = block.dimensions.axes();
    let end = [
        start[0].checked_add(size[0])?,
        start[1].checked_add(size[1])?,
        start[2].checked_add(size[2])?,
    ];
    Some((start, end))
}

fn boxes_overlap(a: &ProductBlock, b: &ProductBlock) -> bool {
    let (Some((a_start, a_end)), Some((b_start, b_end))) = (product_bounds(a), product_bounds(b))
    else {
        return false;
    };

    (0..3).all(|axis| a_start[axis] < b_end[axis] && b_start[axis] < a_end[axis])
}

fn add_product_volume(total: &mut u128, block: &ProductBlock) -> Check<()> {
    let volume = block
        .dimensions
        .volume_mm3()
        .ok_or_else(|| format!("{}: product volume overflow", block.block_id))?;
    *total = total
        .checked_add(volume)
        .ok_or_else(|| "product volume accumulator overflow".to_owned())?;
    Ok(())
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn evaluate_scenario(scenario: &Scenario, tolerance_ppm: u32) -> ScenarioReceipt {
    let mut reasons = BTreeSet::<String>::new();
    let dims = scenario.rock_mass.dimensions.axes();
    let rock_dims_valid = scenario.rock_mass.dimensions.all_positive();

    if scenario.rock_mass.evidence_class != "synthetic" {
        reasons.insert("ROCK_EVIDENCE_CLASS_UNSUPPORTED".to_owned());
    }
    if !rock_dims_valid {
        reasons.insert("ROCK_DIMENSION_INVALID".to_owned());
    }
    if scenario.rock_mass.mass_id.trim().is_empty() {
        reasons.insert("MASS_ID_MISSING".to_owned());
    }

    let rock_volume = scenario.rock_mass.dimensions.volume_mm3();
    if rock_volume.is_none() {
        reasons.insert("VOLUME_OVERFLOW".to_owned());
    }

    let mut discontinuity_ids = BTreeSet::new();
    for plane in &scenario.discontinuities {
        if plane.discontinuity_id.trim().is_empty()
            || !discontinuity_ids.insert(plane.discontinuity_id.clone())
        {
            reasons.insert("DUPLICATE_OR_EMPTY_DISCONTINUITY_ID".to_owned());
        }
        let axis_length = dims[plane.axis.index()];
        if plane.offset_mm == 0 || plane.offset_mm >= axis_length {
            reasons.insert("FRACTURE_OFFSET_INVALID".to_owned());
        }
    }

    let mut product_ids = BTreeSet::new();
    let mut product_volume = 0_u128;
    for block in &scenario.product_blocks {
        if block.block_id.trim().is_empty() || !product_ids.insert(block.block_id.clone()) {
            reasons.insert("DUPLICATE_OR_EMPTY_PRODUCT_ID".to_owned());
        }
        if !block.dimensions.all_positive() {
            reasons.insert("PRODUCT_DIMENSION_INVALID".to_owned());
        }
        if add_product_volume(&mut product_volume, block).is_err() {
            reasons.insert("VOLUME_OVERFLOW".to_owned());
        }

        match product_bounds(block) {
            None => {
                reasons.insert("PRODUCT_BOUNDS_OVERFLOW".to_owned());
            }
            Some((start, end)) => {
                if (0..3).any(|axis| end[axis] > dims[axis]) {
                    reasons.insert("PRODUCT_OUT_OF_BOUNDS".to_owned());
                }
                if block.intended_use == IntendedUse::LoadBearing
                    && block.quality_evidence != QualityEvidence::Qualified
                {
                    reasons.insert("PRODUCT_QUALIFICATION_UNRESOLVED".to_owned());
                }
                for plane in &scenario.discontinuities {
                    if plane.crossing_policy != CrossingPolicy::Forbidden {
                        continue;
                    }
                    let axis = plane.axis.index();
                    if start[axis] < plane.offset_mm && end[axis] > plane.offset_mm {
                        reasons.insert("PROHIBITED_FRACTURE_CROSSING".to_owned());
                    }
                }
            }
        }
    }

    for i in 0..scenario.product_blocks.len() {
        for j in (i + 1)..scenario.product_blocks.len() {
            if boxes_overlap(&scenario.product_blocks[i], &scenario.product_blocks[j]) {
                reasons.insert("PRODUCT_OVERLAP".to_owned());
            }
        }
    }

    let non_product_streams = scenario.material_streams.recoverable_offcuts_mm3 as u128
        + scenario.material_streams.kerf_loss_mm3 as u128
        + scenario.material_streams.fines_and_unclassified_mm3 as u128;
    let accounted = product_volume.checked_add(non_product_streams);
    let accounting_difference = match (rock_volume, accounted) {
        (Some(source), Some(output)) if rock_dims_valid => {
            let difference = source.abs_diff(output);
            let tolerance = (source / 1_000_000_u128) * tolerance_ppm as u128
                + ((source % 1_000_000_u128) * tolerance_ppm as u128) / 1_000_000_u128;
            if difference > tolerance {
                reasons.insert("MATERIAL_BALANCE_MISMATCH".to_owned());
            }
            Some(difference)
        }
        _ => None,
    };

    let actual_reason_codes: Vec<String> = reasons.into_iter().collect();
    let actual_disposition = if actual_reason_codes.is_empty() {
        Disposition::Pass
    } else {
        Disposition::Reject
    };

    let mut expected_reasons = scenario.expected_reason_codes.clone();
    sort_dedup(&mut expected_reasons);
    let expected_control_matched = actual_disposition == scenario.expected_disposition
        && actual_reason_codes == expected_reasons
        && !scenario.expected_assertions.is_empty()
        && !scenario.negative_control.trim().is_empty();

    ScenarioReceipt {
        scenario_id: scenario.scenario_id.clone(),
        mass_id: scenario.rock_mass.mass_id.clone(),
        expected_disposition: scenario.expected_disposition,
        actual_disposition,
        expected_reason_codes: expected_reasons,
        actual_reason_codes,
        rock_volume_mm3: rock_volume,
        product_volume_mm3: product_volume,
        declared_non_product_streams_mm3: non_product_streams,
        accounting_difference_mm3: accounting_difference,
        assertions: scenario.expected_assertions.clone(),
        negative_control: scenario.negative_control.clone(),
        expected_control_matched,
    }
}

fn validate_corpus(corpus: &Corpus, corpus_sha256: String, source_commit: String) -> Receipt {
    let mut scenarios = Vec::with_capacity(corpus.scenarios.len());
    let mut ids = BTreeSet::new();
    let mut duplicate_id = false;
    for scenario in &corpus.scenarios {
        if scenario.scenario_id.trim().is_empty() || !ids.insert(scenario.scenario_id.clone()) {
            duplicate_id = true;
        }
        scenarios.push(evaluate_scenario(scenario, corpus.accounting_tolerance_ppm));
    }

    let expected_controls_matched = scenarios
        .iter()
        .filter(|scenario| scenario.expected_control_matched)
        .count();
    let all_controls_matched = !duplicate_id
        && !scenarios.is_empty()
        && expected_controls_matched == scenarios.len()
        && corpus.suite_id == "STONE_RECOVERY_REFERENCE_BENCHMARK"
        && corpus.suite_version == "0.1.0"
        && corpus.evidence_class == "synthetic"
        && corpus.accounting_tolerance_ppm <= 100_000
        && !corpus.claim_ceiling.trim().is_empty();

    Receipt {
        result: if all_controls_matched {
            "validated"
        } else {
            "contract_mismatch"
        },
        validator_version: VERSION,
        suite_id: corpus.suite_id.clone(),
        suite_version: corpus.suite_version.clone(),
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        scenario_count: scenarios.len(),
        expected_controls_matched,
        corpus_sha256,
        source_commit,
        scenarios,
    }
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err(
            "usage: stone-recovery-bench <corpus.json> --source-commit <40-hex-commit>".to_owned(),
        );
    }
    let source_commit = args[2].to_ascii_lowercase();
    if source_commit.len() != 40
        || !source_commit
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err("--source-commit must be exactly 40 hexadecimal characters".to_owned());
    }

    let bytes = fs::read(&args[0]).map_err(|error| format!("cannot read corpus: {error}"))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    let receipt = validate_corpus(&corpus, digest, source_commit);
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize receipt: {error}"))?
    );

    if receipt.result != "validated" {
        return Err("one or more frozen benchmark controls did not match".to_owned());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
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
            "../../../docs/tech/stone-recovery-benchmark-corpus-v0.1.json"
        ))
        .expect("frozen synthetic corpus must parse")
    }

    #[test]
    fn frozen_corpus_matches_known_answers() {
        let corpus = corpus();
        let receipt = validate_corpus(&corpus, "0".repeat(64), "0".repeat(40));
        assert_eq!(receipt.result, "validated");
        assert_eq!(receipt.scenario_count, 4);
        assert_eq!(receipt.expected_controls_matched, 4);
        assert_eq!(receipt.scenarios[0].actual_disposition, Disposition::Pass);
        assert_eq!(
            receipt.scenarios[1].actual_reason_codes,
            vec!["PROHIBITED_FRACTURE_CROSSING".to_owned()]
        );
    }

    #[test]
    fn touching_a_fracture_plane_at_a_cut_face_is_not_crossing() {
        let mut corpus = corpus();
        let tolerance_ppm = corpus.accounting_tolerance_ppm;
        let scenario = &mut corpus.scenarios[0];
        scenario.discontinuities.push(Discontinuity {
            discontinuity_id: "joint-at-cut-face".to_owned(),
            axis: Axis::X,
            offset_mm: 450,
            crossing_policy: CrossingPolicy::Forbidden,
        });
        let report = evaluate_scenario(scenario, tolerance_ppm);
        assert_eq!(report.actual_disposition, Disposition::Pass);
    }

    #[test]
    fn overlapping_products_are_rejected() {
        let mut corpus = corpus();
        let tolerance_ppm = corpus.accounting_tolerance_ppm;
        let scenario = &mut corpus.scenarios[0];
        scenario.product_blocks[1].origin.x_mm = 400;
        let report = evaluate_scenario(scenario, tolerance_ppm);
        assert!(report
            .actual_reason_codes
            .contains(&"PRODUCT_OVERLAP".to_owned()));
    }
}
