#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const RECEIPT_SCHEMA: &str = "luminous.spacecraft-unit-conformal-calibration.receipt.v0.1";
pub const NOMINAL_LEVELS_BPS: [u16; 5] = [5_000, 8_000, 9_000, 9_500, 9_900];
pub const MIN_HELDOUT_UNITS: u64 = 30;
pub const WILSON_Z_95: f64 = 1.959_963_984_540_054;
pub const Q32_INFINITY_SENTINEL: u64 = u64::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnitKind {
    Target,
    Session,
    Sequence,
}

impl UnitKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "TARGET" => Ok(Self::Target),
            "SESSION" => Ok(Self::Session),
            "SEQUENCE" => Ok(Self::Sequence),
            _ => Err(format!("unsupported unit_kind: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Target => "TARGET",
            Self::Session => "SESSION",
            Self::Sequence => "SEQUENCE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partition {
    Calibration,
    Test,
}

impl Partition {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "CALIBRATION" => Ok(Self::Calibration),
            "TEST" => Ok(Self::Test),
            _ => Err(format!("unsupported split: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Calibration => "CALIBRATION",
            Self::Test => "TEST",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    Scored,
    Abstained,
    Missing,
    Invalid,
}

impl ObservationStatus {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "SCORED" => Ok(Self::Scored),
            "ABSTAINED" => Ok(Self::Abstained),
            "MISSING" => Ok(Self::Missing),
            "INVALID_INPUT" => Ok(Self::Invalid),
            _ => Err(format!("unsupported observation status: {value}")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scored => "SCORED",
            Self::Abstained => "ABSTAINED",
            Self::Missing => "MISSING",
            Self::Invalid => "INVALID_INPUT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub sample_id: String,
    pub target_id: String,
    pub session_id: String,
    pub sequence_id: String,
    pub unit_kind: UnitKind,
    pub unit_id: String,
    pub partition: Partition,
    pub domain_id: String,
    pub condition_id: String,
    pub crop_stratum: String,
    pub modality: String,
    pub ood_group: String,
    pub calibration_revision: String,
    pub unit_frame_budget: u32,
    pub status: ObservationStatus,
    /// None is treated as +infinity in both frame-wise and unit-wise calibration.
    pub score_q32_32: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ScopeKey {
    domain_id: String,
    condition_id: String,
    crop_stratum: String,
    modality: String,
    ood_group: String,
    calibration_revision: String,
    unit_frame_budget: u32,
}

impl ScopeKey {
    fn from_observation(row: &Observation) -> Self {
        Self {
            domain_id: row.domain_id.clone(),
            condition_id: row.condition_id.clone(),
            crop_stratum: row.crop_stratum.clone(),
            modality: row.modality.clone(),
            ood_group: row.ood_group.clone(),
            calibration_revision: row.calibration_revision.clone(),
            unit_frame_budget: row.unit_frame_budget,
        }
    }

    pub fn domain_id(&self) -> &str {
        &self.domain_id
    }

    pub fn condition_id(&self) -> &str {
        &self.condition_id
    }

    pub fn crop_stratum(&self) -> &str {
        &self.crop_stratum
    }

    pub fn modality(&self) -> &str {
        &self.modality
    }

    pub fn ood_group(&self) -> &str {
        &self.ood_group
    }

    pub fn calibration_revision(&self) -> &str {
        &self.calibration_revision
    }

    pub const fn unit_frame_budget(&self) -> u32 {
        self.unit_frame_budget
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UnitKey {
    scope: ScopeKey,
    unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnitRows {
    partition: Partition,
    scores: Vec<Option<u64>>,
    scored_count: u64,
    abstained_count: u64,
    missing_count: u64,
    invalid_count: u64,
}

impl UnitRows {
    fn new(partition: Partition) -> Self {
        Self {
            partition,
            scores: Vec::new(),
            scored_count: 0,
            abstained_count: 0,
            missing_count: 0,
            invalid_count: 0,
        }
    }

    fn push(&mut self, row: &Observation) {
        self.scores.push(row.score_q32_32);
        match row.status {
            ObservationStatus::Scored => self.scored_count += 1,
            ObservationStatus::Abstained => self.abstained_count += 1,
            ObservationStatus::Missing => self.missing_count += 1,
            ObservationStatus::Invalid => self.invalid_count += 1,
        }
    }

    fn max_score(&self) -> Option<u64> {
        if self.scores.iter().any(Option::is_none) {
            return None;
        }
        self.scores.iter().filter_map(|score| *score).max()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WilsonIntervalPpm {
    pub lower_ppm: u32,
    pub upper_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdResult {
    pub nominal_level_bps: u16,
    pub calibration_frame_count: u64,
    pub calibration_unit_count: u64,
    pub framewise_order_statistic_rank: u64,
    pub unitwise_order_statistic_rank: u64,
    pub framewise_threshold_q32_32: Option<u64>,
    pub unitwise_threshold_q32_32: Option<u64>,
    pub framewise_finite: bool,
    pub unitwise_finite: bool,
    pub test_frame_count: u64,
    pub test_unit_count: u64,
    pub test_scored_frame_count: u64,
    pub test_abstained_frame_count: u64,
    pub test_missing_frame_count: u64,
    pub test_invalid_frame_count: u64,
    pub framewise_covered_frame_count: Option<u64>,
    pub framewise_conditional_coverage_ppm: Option<u32>,
    pub framewise_operational_success_ppm: Option<u32>,
    pub framewise_complete_unit_success_count: Option<u64>,
    pub framewise_complete_unit_wilson_95: Option<WilsonIntervalPpm>,
    pub framewise_silent_miss_unit_count: Option<u64>,
    pub framewise_silent_miss_wilson_95: Option<WilsonIntervalPpm>,
    pub unitwise_covered_frame_count: Option<u64>,
    pub unitwise_conditional_coverage_ppm: Option<u32>,
    pub unitwise_operational_success_ppm: Option<u32>,
    pub unitwise_complete_unit_success_count: Option<u64>,
    pub unitwise_complete_unit_wilson_95: Option<WilsonIntervalPpm>,
    pub unitwise_silent_miss_unit_count: Option<u64>,
    pub unitwise_silent_miss_wilson_95: Option<WilsonIntervalPpm>,
    pub unitwise_units_with_any_unscored_frame: Option<u64>,
    pub promotion_eligible_for_declared_scope: bool,
    pub promotion_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeResult {
    pub domain_id: String,
    pub condition_id: String,
    pub crop_stratum: String,
    pub modality: String,
    pub ood_group: String,
    pub calibration_revision: String,
    pub unit_frame_budget: u32,
    pub unit_kind: String,
    pub calibration_unscored_frame_count: u64,
    pub calibration_units_with_unscored_frames: u64,
    pub levels: Vec<ThresholdResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationReceipt {
    pub schema: String,
    pub unit_kind: String,
    pub observation_count: u64,
    pub scope_count: u64,
    pub results: Vec<ScopeResult>,
    pub claim_ready: bool,
    pub dataset_source_bytes_authenticated: bool,
    pub calibration_exchangeability_proven: bool,
    pub physical_calibration_validated: bool,
    pub deterministic_safety_bound: bool,
    pub maneuver_authority_granted: bool,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value != "NONE"
}

fn unit_identity(row: &Observation) -> &str {
    match row.unit_kind {
        UnitKind::Target => &row.target_id,
        UnitKind::Session => &row.session_id,
        UnitKind::Sequence => &row.sequence_id,
    }
}

fn validate_observation(row: &Observation) -> Result<(), String> {
    for (name, value) in [
        ("sample_id", row.sample_id.as_str()),
        ("target_id", row.target_id.as_str()),
        ("session_id", row.session_id.as_str()),
        ("sequence_id", row.sequence_id.as_str()),
        ("unit_id", row.unit_id.as_str()),
        ("domain_id", row.domain_id.as_str()),
        ("condition_id", row.condition_id.as_str()),
        ("crop_stratum", row.crop_stratum.as_str()),
        ("modality", row.modality.as_str()),
        ("ood_group", row.ood_group.as_str()),
        ("calibration_revision", row.calibration_revision.as_str()),
    ] {
        if !valid_text(value) {
            return Err(format!("{name} is empty or reserved"));
        }
    }

    if row.unit_frame_budget == 0 {
        return Err("unit_frame_budget must be positive".to_owned());
    }
    if row.unit_id != unit_identity(row) {
        return Err(format!(
            "unit_id does not match declared {} identity for sample {}",
            row.unit_kind.as_str(),
            row.sample_id
        ));
    }

    match (row.status, row.score_q32_32) {
        (ObservationStatus::Scored, Some(score)) if score != Q32_INFINITY_SENTINEL => Ok(()),
        (ObservationStatus::Scored, _) => {
            Err("SCORED rows require a finite Q32.32 score".to_owned())
        }
        (_, None) => Ok(()),
        (_, Some(_)) => Err("unscored rows must carry score 0 / no score".to_owned()),
    }
}

fn validate_manifest(rows: &[Observation]) -> Result<UnitKind, String> {
    if rows.is_empty() {
        return Err("manifest has no observations".to_owned());
    }

    let unit_kind = rows[0].unit_kind;
    let mut sample_ids = BTreeSet::<&str>::new();
    let mut unit_partitions = BTreeMap::<(UnitKind, &str), Partition>::new();

    for row in rows {
        validate_observation(row)?;
        if row.unit_kind != unit_kind {
            return Err("unit_kind changes within manifest".to_owned());
        }
        if !sample_ids.insert(row.sample_id.as_str()) {
            return Err(format!("duplicate sample_id: {}", row.sample_id));
        }

        let identity = (row.unit_kind, row.unit_id.as_str());
        if let Some(previous) = unit_partitions.insert(identity, row.partition) {
            if previous != row.partition {
                return Err(format!(
                    "independent unit {} appears in both {} and {}",
                    row.unit_id,
                    previous.as_str(),
                    row.partition.as_str()
                ));
            }
        }
    }

    Ok(unit_kind)
}

fn conformal_rank(unit_count: u64, nominal_level_bps: u16) -> Option<u64> {
    if nominal_level_bps == 0 || nominal_level_bps > 10_000 {
        return None;
    }
    let numerator = (u128::from(unit_count) + 1) * u128::from(nominal_level_bps);
    let rank = numerator.div_ceil(10_000);
    u64::try_from(rank).ok()
}

/// Returns the exact one-based conformal order statistic. None means the rank exceeds n
/// or the selected order statistic is +infinity because the calibration unit was unscored.
fn conformal_quantile(scores: &[Option<u64>], nominal_level_bps: u16) -> Option<(u64, Option<u64>)> {
    let sample_count = u64::try_from(scores.len()).ok()?;
    let rank = conformal_rank(sample_count, nominal_level_bps)?;
    if rank > sample_count {
        return Some((rank, None));
    }

    let mut ordered = scores.to_vec();
    ordered.sort_by(|left, right| match (left, right) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    let selected = ordered.get(usize::try_from(rank - 1).ok()?)?;
    Some((rank, *selected))
}

fn ratio_ppm(numerator: u64, denominator: u64) -> Option<u32> {
    if denominator == 0 {
        return None;
    }
    let scaled = u128::from(numerator)
        .checked_mul(1_000_000)?
        .checked_div(u128::from(denominator))?;
    u32::try_from(scaled).ok()
}

fn ppm_from_probability(value: f64, lower: bool) -> Option<u32> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return None;
    }
    let scaled = value * 1_000_000.0;
    if !scaled.is_finite() || scaled < 0.0 || scaled > 1_000_000.0 {
        return None;
    }
    let rounded = if lower { scaled.floor() } else { scaled.ceil() };
    u32::try_from(rounded as u64).ok()
}

/// Two-sided 95% Wilson score interval. Independent units, never frames, are the trials.
pub fn wilson_interval_95(successes: u64, trials: u64) -> Option<WilsonIntervalPpm> {
    if trials == 0 || successes > trials {
        return None;
    }

    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let variance = (p * (1.0 - p) / n) + (z2 / (4.0 * n * n));
    let half = WILSON_Z_95 * variance.sqrt() / denominator;
    let lower = (center - half).clamp(0.0, 1.0);
    let upper = (center + half).clamp(0.0, 1.0);

    Some(WilsonIntervalPpm {
        lower_ppm: ppm_from_probability(lower, true)?,
        upper_ppm: ppm_from_probability(upper, false)?,
    })
}

#[derive(Default)]
struct ThresholdCounts {
    covered_frames: u64,
    complete_unit_successes: u64,
    silent_miss_units: u64,
    units_with_unscored_frames: u64,
}

fn evaluate_threshold(
    units: &BTreeMap<String, UnitRows>,
    threshold: Option<u64>,
) -> ThresholdCounts {
    let mut result = ThresholdCounts::default();
    let Some(threshold) = threshold else {
        return result;
    };

    for unit in units.values() {
        let has_unscored = unit.scores.iter().any(Option::is_none);
        let has_silent_miss = unit
            .scores
            .iter()
            .flatten()
            .any(|score| *score > threshold);
        if has_unscored {
            result.units_with_unscored_frames += 1;
        }
        if has_silent_miss {
            result.silent_miss_units += 1;
        }

        let mut complete_success = true;
        for score in &unit.scores {
            match score {
                Some(value) if *value <= threshold => result.covered_frames += 1,
                _ => complete_success = false,
            }
        }
        if complete_success {
            result.complete_unit_successes += 1;
        }
    }

    result
}

fn count_status(units: &BTreeMap<String, UnitRows>) -> (u64, u64, u64, u64, u64) {
    let mut frames = 0_u64;
    let mut scored = 0_u64;
    let mut abstained = 0_u64;
    let mut missing = 0_u64;
    let mut invalid = 0_u64;
    for unit in units.values() {
        frames += u64::try_from(unit.scores.len()).unwrap_or(u64::MAX);
        scored += unit.scored_count;
        abstained += unit.abstained_count;
        missing += unit.missing_count;
        invalid += unit.invalid_count;
    }
    (frames, scored, abstained, missing, invalid)
}

fn threshold_result(
    nominal_level_bps: u16,
    calibration_frame_scores: &[Option<u64>],
    calibration_unit_scores: &[Option<u64>],
    test_units: &BTreeMap<String, UnitRows>,
) -> ThresholdResult {
    let (frame_rank, frame_threshold) =
        conformal_quantile(calibration_frame_scores, nominal_level_bps)
            .unwrap_or((0, None));
    let (unit_rank, unit_threshold) =
        conformal_quantile(calibration_unit_scores, nominal_level_bps)
            .unwrap_or((0, None));

    let (test_frame_count, test_scored, test_abstained, test_missing, test_invalid) =
        count_status(test_units);
    let test_unit_count = u64::try_from(test_units.len()).unwrap_or(u64::MAX);

    let frame_counts = frame_threshold.map(|value| evaluate_threshold(test_units, Some(value)));
    let unit_counts = unit_threshold.map(|value| evaluate_threshold(test_units, Some(value)));

    let frame_covered = frame_counts.as_ref().map(|counts| counts.covered_frames);
    let unit_covered = unit_counts.as_ref().map(|counts| counts.covered_frames);
    let frame_complete = frame_counts
        .as_ref()
        .map(|counts| counts.complete_unit_successes);
    let unit_complete = unit_counts
        .as_ref()
        .map(|counts| counts.complete_unit_successes);
    let frame_silent = frame_counts.as_ref().map(|counts| counts.silent_miss_units);
    let unit_silent = unit_counts.as_ref().map(|counts| counts.silent_miss_units);

    let frame_conditional = frame_covered.and_then(|count| ratio_ppm(count, test_scored));
    let unit_conditional = unit_covered.and_then(|count| ratio_ppm(count, test_scored));
    let frame_operational = frame_covered.and_then(|count| ratio_ppm(count, test_frame_count));
    let unit_operational = unit_covered.and_then(|count| ratio_ppm(count, test_frame_count));

    let frame_unit_ci = frame_complete.and_then(|count| wilson_interval_95(count, test_unit_count));
    let unit_unit_ci = unit_complete.and_then(|count| wilson_interval_95(count, test_unit_count));
    let frame_silent_ci = frame_silent.and_then(|count| wilson_interval_95(count, test_unit_count));
    let unit_silent_ci = unit_silent.and_then(|count| wilson_interval_95(count, test_unit_count));

    let nominal_ppm = u32::from(nominal_level_bps) * 100;
    let promotion_reason = if unit_threshold.is_none() {
        "NO_FINITE_UNIT_CONFORMAL_THRESHOLD"
    } else if test_unit_count < MIN_HELDOUT_UNITS {
        "INSUFFICIENT_INDEPENDENT_TEST_UNITS"
    } else if unit_unit_ci
        .as_ref()
        .is_none_or(|interval| interval.lower_ppm < nominal_ppm)
    {
        "HELDOUT_UNIT_COVERAGE_LOWER_BOUND_BELOW_NOMINAL"
    } else {
        "UNIT_LEVEL_HOLDOUT_TARGET_MET_FOR_DECLARED_SCOPE"
    };

    ThresholdResult {
        nominal_level_bps,
        calibration_frame_count: u64::try_from(calibration_frame_scores.len()).unwrap_or(u64::MAX),
        calibration_unit_count: u64::try_from(calibration_unit_scores.len()).unwrap_or(u64::MAX),
        framewise_order_statistic_rank: frame_rank,
        unitwise_order_statistic_rank: unit_rank,
        framewise_threshold_q32_32: frame_threshold,
        unitwise_threshold_q32_32: unit_threshold,
        framewise_finite: frame_threshold.flatten().is_some(),
        unitwise_finite: unit_threshold.flatten().is_some(),
        test_frame_count,
        test_unit_count,
        test_scored_frame_count: test_scored,
        test_abstained_frame_count: test_abstained,
        test_missing_frame_count: test_missing,
        test_invalid_frame_count: test_invalid,
        framewise_covered_frame_count: frame_covered,
        framewise_conditional_coverage_ppm: frame_conditional,
        framewise_operational_success_ppm: frame_operational,
        framewise_complete_unit_success_count: frame_complete,
        framewise_complete_unit_wilson_95: frame_unit_ci,
        framewise_silent_miss_unit_count: frame_silent,
        framewise_silent_miss_wilson_95: frame_silent_ci,
        unitwise_covered_frame_count: unit_covered,
        unitwise_conditional_coverage_ppm: unit_conditional,
        unitwise_operational_success_ppm: unit_operational,
        unitwise_complete_unit_success_count: unit_complete,
        unitwise_complete_unit_wilson_95: unit_unit_ci,
        unitwise_silent_miss_unit_count: unit_silent,
        unitwise_silent_miss_wilson_95: unit_silent_ci,
        unitwise_units_with_any_unscored_frame: unit_counts
            .as_ref()
            .map(|counts| counts.units_with_unscored_frames),
        promotion_eligible_for_declared_scope: promotion_reason
            == "UNIT_LEVEL_HOLDOUT_TARGET_MET_FOR_DECLARED_SCOPE",
        promotion_reason: promotion_reason.to_owned(),
    }
}

pub fn evaluate_manifest(rows: &[Observation]) -> Result<EvaluationReceipt, String> {
    let unit_kind = validate_manifest(rows)?;
    let mut scopes = BTreeMap::<ScopeKey, BTreeMap<Partition, BTreeMap<String, UnitRows>>>::new();

    for row in rows {
        let scope = ScopeKey::from_observation(row);
        let unit = scopes
            .entry(scope)
            .or_default()
            .entry(row.partition)
            .or_default()
            .entry(row.unit_id.clone())
            .or_insert_with(|| UnitRows::new(row.partition));
        if unit.partition != row.partition {
            return Err(format!("unit {} crosses partitions", row.unit_id));
        }
        unit.push(row);
    }

    let mut results = Vec::with_capacity(scopes.len());
    for (scope, partitions) in scopes {
        let calibration_units = partitions
            .get(&Partition::Calibration)
            .cloned()
            .unwrap_or_default();
        let test_units = partitions
            .get(&Partition::Test)
            .cloned()
            .unwrap_or_default();

        for unit in calibration_units.values().chain(test_units.values()) {
            if unit.scores.len() != usize::try_from(scope.unit_frame_budget).unwrap_or(usize::MAX) {
                return Err(format!(
                    "unit frame count {} differs from budget {} in domain {}",
                    unit.scores.len(),
                    scope.unit_frame_budget,
                    scope.domain_id
                ));
            }
        }

        let calibration_frame_scores = calibration_units
            .values()
            .flat_map(|unit| unit.scores.iter().copied())
            .collect::<Vec<_>>();
        let calibration_unit_scores = calibration_units
            .values()
            .map(UnitRows::max_score)
            .collect::<Vec<_>>();

        let calibration_unscored_frames = calibration_units
            .values()
            .map(|unit| unit.scores.iter().filter(|score| score.is_none()).count() as u64)
            .sum();
        let calibration_units_with_unscored = calibration_units
            .values()
            .filter(|unit| unit.scores.iter().any(Option::is_none))
            .count() as u64;

        let levels = NOMINAL_LEVELS_BPS
            .iter()
            .map(|level| {
                threshold_result(
                    *level,
                    &calibration_frame_scores,
                    &calibration_unit_scores,
                    &test_units,
                )
            })
            .collect();

        results.push(ScopeResult {
            domain_id: scope.domain_id,
            condition_id: scope.condition_id,
            crop_stratum: scope.crop_stratum,
            modality: scope.modality,
            ood_group: scope.ood_group,
            calibration_revision: scope.calibration_revision,
            unit_frame_budget: scope.unit_frame_budget,
            unit_kind: unit_kind.as_str().to_owned(),
            calibration_unscored_frame_count: calibration_unscored_frames,
            calibration_units_with_unscored_frames: calibration_units_with_unscored,
            levels,
        });
    }

    let observation_count = u64::try_from(rows.len()).unwrap_or(u64::MAX);
    Ok(EvaluationReceipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        unit_kind: unit_kind.as_str().to_owned(),
        observation_count,
        scope_count: u64::try_from(results.len()).unwrap_or(u64::MAX),
        results,
        claim_ready: false,
        dataset_source_bytes_authenticated: false,
        calibration_exchangeability_proven: false,
        physical_calibration_validated: false,
        deterministic_safety_bound: false,
        maneuver_authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        NOMINAL_LEVELS_BPS, Observation, ObservationStatus, Partition, UnitKind,
        conformal_quantile, evaluate_manifest, wilson_interval_95,
    };

    fn row(
        sample: &str,
        unit: &str,
        split: Partition,
        frame: u32,
        budget: u32,
        status: ObservationStatus,
        score: Option<u64>,
        domain: &str,
    ) -> Observation {
        Observation {
            sample_id: sample.to_owned(),
            target_id: "TANGO".to_owned(),
            session_id: format!("session-{unit}"),
            sequence_id: unit.to_owned(),
            unit_kind: UnitKind::Sequence,
            unit_id: unit.to_owned(),
            partition: split,
            domain_id: domain.to_owned(),
            condition_id: "COND-1".to_owned(),
            crop_stratum: "FULL".to_owned(),
            modality: "RGB".to_owned(),
            ood_group: "IN_DOMAIN".to_owned(),
            calibration_revision: "cal-r1".to_owned(),
            unit_frame_budget: budget,
            status,
            score_q32_32: score,
        }
    }

    fn clean_manifest(calibration_units: u32, test_units: u32, frame_budget: u32) -> Vec<Observation> {
        let mut rows = Vec::new();
        for unit_index in 0..calibration_units {
            for frame in 0..frame_budget {
                let score = if frame == 0 { 1_000 } else { 100 };
                rows.push(row(
                    &format!("cal-{unit_index}-{frame}"),
                    &format!("cal-seq-{unit_index}"),
                    Partition::Calibration,
                    frame,
                    frame_budget,
                    ObservationStatus::Scored,
                    Some(score),
                    "LIGHTBOX",
                ));
            }
        }
        for unit_index in 0..test_units {
            for frame in 0..frame_budget {
                let score = if frame == 0 { 500 } else { 100 };
                rows.push(row(
                    &format!("test-{unit_index}-{frame}"),
                    &format!("test-seq-{unit_index}"),
                    Partition::Test,
                    frame,
                    frame_budget,
                    ObservationStatus::Scored,
                    Some(score),
                    "LIGHTBOX",
                ));
            }
        }
        rows
    }

    #[test]
    fn exact_finite_sample_ranks_use_calibration_units() {
        let scores = vec![Some(1), Some(2), Some(3), Some(4)];
        assert_eq!(conformal_quantile(&scores, 5_000), Some((3, Some(3))));
        assert_eq!(conformal_quantile(&scores, 9_900), Some((5, None)));
        assert_eq!(NOMINAL_LEVELS_BPS, [5_000, 8_000, 9_000, 9_500, 9_900]);
    }

    #[test]
    fn unit_max_quantile_detects_rare_within_sequence_extremes() {
        let rows = clean_manifest(30, 35, 11);
        let receipt = evaluate_manifest(&rows).expect("valid manifest");
        let scope = &receipt.results[0];
        let at_90 = &scope.levels[2];
        assert_eq!(at_90.framewise_threshold_q32_32, Some(100));
        assert_eq!(at_90.unitwise_threshold_q32_32, Some(1_000));
        assert_eq!(at_90.framewise_complete_unit_success_count, Some(0));
        assert_eq!(at_90.unitwise_complete_unit_success_count, Some(35));
        assert_eq!(at_90.unitwise_silent_miss_unit_count, Some(0));
        assert!(at_90.promotion_eligible_for_declared_scope);
    }

    #[test]
    fn unitwise_conformal_profile_has_no_finite_99_percent_rank_with_30_units() {
        let rows = clean_manifest(30, 35, 11);
        let receipt = evaluate_manifest(&rows).expect("valid manifest");
        let at_99 = &receipt.results[0].levels[4];
        assert_eq!(at_99.unitwise_order_statistic_rank, 31);
        assert!(!at_99.unitwise_finite);
        assert!(!at_99.promotion_eligible_for_declared_scope);
    }

    #[test]
    fn unscored_calibration_frame_is_infinite_unit_score_not_dropped() {
        let mut rows = clean_manifest(30, 35, 11);
        let first = rows
            .iter_mut()
            .find(|row| row.sample_id == "cal-0-0")
            .expect("calibration row");
        first.status = ObservationStatus::Abstained;
        first.score_q32_32 = None;
        let receipt = evaluate_manifest(&rows).expect("valid manifest");
        let scope = &receipt.results[0];
        assert_eq!(scope.calibration_unscored_frame_count, 1);
        assert_eq!(scope.calibration_units_with_unscored_frames, 1);
        assert_eq!(scope.levels[2].unitwise_threshold_q32_32, Some(1_000));
    }

    #[test]
    fn unscored_test_frame_prevents_complete_unit_success() {
        let mut rows = clean_manifest(30, 35, 11);
        let first = rows
            .iter_mut()
            .find(|row| row.sample_id == "test-0-0")
            .expect("test row");
        first.status = ObservationStatus::Missing;
        first.score_q32_32 = None;
        let receipt = evaluate_manifest(&rows).expect("valid manifest");
        let at_90 = &receipt.results[0].levels[2];
        assert_eq!(at_90.unitwise_complete_unit_success_count, Some(34));
        assert_eq!(at_90.unitwise_units_with_any_unscored_frame, Some(1));
        assert!(!at_90.promotion_eligible_for_declared_scope);
    }

    #[test]
    fn independent_unit_cannot_cross_calibration_and_test() {
        let mut rows = clean_manifest(2, 2, 2);
        let duplicate_unit = rows
            .iter_mut()
            .find(|row| row.sample_id == "test-0-0")
            .expect("test row");
        duplicate_unit.unit_id = "cal-seq-0".to_owned();
        duplicate_unit.sequence_id = "cal-seq-0".to_owned();
        assert!(evaluate_manifest(&rows).is_err());
    }

    #[test]
    fn duplicate_sample_identity_is_rejected() {
        let mut rows = clean_manifest(2, 2, 2);
        let mut duplicate = rows[0].clone();
        duplicate.partition = Partition::Calibration;
        rows.push(duplicate);
        assert!(evaluate_manifest(&rows).is_err());
    }

    #[test]
    fn incomplete_unit_length_is_rejected() {
        let mut rows = clean_manifest(2, 2, 2);
        rows.pop();
        assert!(evaluate_manifest(&rows).is_err());
    }

    #[test]
    fn scope_domain_changes_are_never_pooled() {
        let mut rows = clean_manifest(30, 35, 11);
        let mut other = clean_manifest(30, 35, 11);
        for row in &mut other {
            row.domain_id = "SUNLAMP".to_owned();
            row.sample_id.push_str("-sun");
            row.unit_id.push_str("-sun");
            row.sequence_id = row.unit_id.clone();
            row.session_id = format!("session-{}", row.unit_id);
        }
        rows.extend(other);
        let receipt = evaluate_manifest(&rows).expect("valid separate domains");
        assert_eq!(receipt.scope_count, 2);
        assert!(receipt
            .results
            .iter()
            .all(|scope| scope.levels[2].unitwise_threshold_q32_32 == Some(1_000)));
    }

    #[test]
    fn test_only_domain_has_no_calibration_profile() {
        let mut rows = clean_manifest(0, 35, 11);
        for row in &mut rows {
            row.domain_id = "SUNLAMP".to_owned();
        }
        let receipt = evaluate_manifest(&rows).expect("test-only scope is reportable");
        let at_90 = &receipt.results[0].levels[2];
        assert!(at_90.unitwise_threshold_q32_32.is_none());
        assert_eq!(at_90.promotion_reason, "NO_FINITE_UNIT_CONFORMAL_THRESHOLD");
    }

    #[test]
    fn unscored_rows_must_not_carry_a_finite_score() {
        let mut rows = clean_manifest(2, 2, 2);
        let first = rows
            .iter_mut()
            .find(|row| row.sample_id == "cal-0-0")
            .expect("calibration row");
        first.status = ObservationStatus::Abstained;
        assert!(evaluate_manifest(&rows).is_err());
    }

    #[test]
    fn wilson_interval_uses_independent_unit_denominator() {
        let interval = wilson_interval_95(35, 35).expect("nonempty interval");
        assert!(interval.lower_ppm > 900_000);
        assert_eq!(wilson_interval_95(0, 0), None);
    }

    #[test]
    fn target_and_session_unit_kinds_validate_the_declared_id_field() {
        let mut rows = clean_manifest(2, 2, 2);
        for row in &mut rows {
            row.unit_kind = UnitKind::Target;
            row.unit_id = row.target_id.clone();
        }
        assert!(evaluate_manifest(&rows).is_err());
        let mut rows = clean_manifest(2, 2, 2);
        for row in &mut rows {
            row.unit_kind = UnitKind::Session;
            row.unit_id = row.session_id.clone();
        }
        let receipt = evaluate_manifest(&rows).expect("session unit manifest");
        assert_eq!(receipt.unit_kind, "SESSION");
    }
}
