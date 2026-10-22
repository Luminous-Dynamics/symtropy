use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const SEARCH_NODE_LIMIT: u64 = 200_000;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    horizon_hours: u32,
    shared_power_limit_kw: u32,
    kilns: Vec<Kiln>,
    batches: Vec<Batch>,
    expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Kiln {
    kiln_id: String,
    capacity_volume_units: u64,
    supported_profiles: Vec<String>,
    max_power_kw: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    batch_id: String,
    ready_hour: u32,
    duration_hours: u32,
    profile_id: String,
    dry_volume_units: u64,
    heat_demand_kw: u32,
    target_moisture_bp: u16,
    achieved_moisture_bp: u16,
    requires_grade_evidence: bool,
    grade_evidence_status: String,
    requires_inspection: bool,
    inspection_pass: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    baseline_scheduled_batch_ids: Vec<String>,
    baseline_conforming_volume_units: u64,
    oracle_scheduled_batch_ids: Vec<String>,
    oracle_conforming_volume_units: u64,
    oracle_makespan_hour: u32,
    gate_blocked_reasons: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Placement {
    batch_id: String,
    kiln_id: String,
    start_hour: u32,
    finish_hour: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Metrics {
    conforming_dry_volume_units: u64,
    makespan_hour: u32,
    total_wait_hours: u64,
    energy_unit_hours: u64,
    busy_kiln_hours: u64,
    available_kiln_hours: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioResult {
    scenario_id: String,
    gate_blocked_reasons: BTreeMap<String, Vec<String>>,
    baseline_scheduled_batch_ids: Vec<String>,
    baseline_metrics: Metrics,
    baseline_unscheduled_eligible_ids: Vec<String>,
    oracle_disposition: String,
    oracle_scheduled_batch_ids: Vec<String>,
    oracle_metrics: Metrics,
    oracle_unscheduled_eligible_ids: Vec<String>,
    baseline_volume_gap_units: u64,
    explored_nodes: u64,
    schedule_independently_validated: bool,
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

fn unique_nonempty(values: &[String], at: &str) -> Check<()> {
    if values.iter().any(|value| !nonempty(value)) {
        return Err(format!("{at}: identifiers must be non-empty"));
    }
    let unique: BTreeSet<_> = values.iter().collect();
    if unique.len() != values.len() {
        return Err(format!("{at}: duplicate identifiers are not allowed"));
    }
    Ok(())
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.suite_id != "WOOD_FLOW_KILN_SCHEDULE_BENCHMARK"
        || !nonempty(&corpus.suite_version)
        || corpus.evidence_class != "synthetic"
        || !nonempty(&corpus.claim_ceiling)
        || corpus.scenarios.is_empty()
    {
        return Err("invalid corpus metadata".to_owned());
    }

    let scenario_ids: Vec<_> = corpus
        .scenarios
        .iter()
        .map(|s| s.scenario_id.clone())
        .collect();
    unique_nonempty(&scenario_ids, "corpus.scenario_id")?;

    for scenario in &corpus.scenarios {
        if !nonempty(&scenario.scenario_id)
            || scenario.horizon_hours == 0
            || scenario.shared_power_limit_kw == 0
            || scenario.kilns.is_empty()
            || scenario.batches.is_empty()
        {
            return Err(format!(
                "{}: empty or invalid scenario envelope",
                scenario.scenario_id
            ));
        }
        let kiln_ids: Vec<_> = scenario
            .kilns
            .iter()
            .map(|kiln| kiln.kiln_id.clone())
            .collect();
        unique_nonempty(&kiln_ids, &format!("{}.kiln_id", scenario.scenario_id))?;
        let batch_ids: Vec<_> = scenario
            .batches
            .iter()
            .map(|batch| batch.batch_id.clone())
            .collect();
        unique_nonempty(&batch_ids, &format!("{}.batch_id", scenario.scenario_id))?;

        let mut possible_total_volume = 0_u64;
        let mut possible_total_energy = 0_u64;
        for kiln in &scenario.kilns {
            if !nonempty(&kiln.kiln_id)
                || kiln.capacity_volume_units == 0
                || kiln.max_power_kw == 0
                || kiln.supported_profiles.is_empty()
            {
                return Err(format!(
                    "{}: invalid kiln declaration",
                    scenario.scenario_id
                ));
            }
            unique_nonempty(
                &kiln.supported_profiles,
                &format!(
                    "{}.{}.supported_profiles",
                    scenario.scenario_id, kiln.kiln_id
                ),
            )?;
        }

        for batch in &scenario.batches {
            if !nonempty(&batch.batch_id)
                || !nonempty(&batch.profile_id)
                || batch.duration_hours == 0
                || batch.dry_volume_units == 0
                || batch.heat_demand_kw == 0
                || batch.target_moisture_bp > 10_000
                || batch.achieved_moisture_bp > 10_000
                || !matches!(
                    batch.grade_evidence_status.as_str(),
                    "qualified" | "missing" | "not_required" | "unresolved"
                )
            {
                return Err(format!(
                    "{}.{}: invalid batch declaration",
                    scenario.scenario_id, batch.batch_id
                ));
            }
            possible_total_volume = possible_total_volume
                .checked_add(batch.dry_volume_units)
                .ok_or_else(|| format!("{}: total volume overflow", scenario.scenario_id))?;
            possible_total_energy = possible_total_energy
                .checked_add(
                    u64::from(batch.heat_demand_kw)
                        .checked_mul(u64::from(batch.duration_hours))
                        .ok_or_else(|| {
                            format!("{}: batch energy overflow", scenario.scenario_id)
                        })?,
                )
                .ok_or_else(|| format!("{}: total energy overflow", scenario.scenario_id))?;
        }
        let _ = (possible_total_volume, possible_total_energy);
        if scenario
            .expected
            .baseline_scheduled_batch_ids
            .iter()
            .any(|id| !batch_ids.contains(id))
            || scenario
                .expected
                .oracle_scheduled_batch_ids
                .iter()
                .any(|id| !batch_ids.contains(id))
        {
            return Err(format!(
                "{}: expected schedule references unknown batch",
                scenario.scenario_id
            ));
        }
        unique_nonempty(
            &scenario.expected.baseline_scheduled_batch_ids,
            &format!(
                "{}.expected.baseline_scheduled_batch_ids",
                scenario.scenario_id
            ),
        )?;
        unique_nonempty(
            &scenario.expected.oracle_scheduled_batch_ids,
            &format!(
                "{}.expected.oracle_scheduled_batch_ids",
                scenario.scenario_id
            ),
        )?;
        for (batch_id, reasons) in &scenario.expected.gate_blocked_reasons {
            if !batch_ids.contains(batch_id) || reasons.is_empty() {
                return Err(format!(
                    "{}: invalid gate-blocked expected record",
                    scenario.scenario_id
                ));
            }
            unique_nonempty(
                reasons,
                &format!(
                    "{}.expected.gate_blocked_reasons.{batch_id}",
                    scenario.scenario_id
                ),
            )?;
        }
    }
    Ok(())
}

fn gate_reasons(scenario: &Scenario, batch: &Batch) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    let profile_kilns: Vec<_> = scenario
        .kilns
        .iter()
        .filter(|kiln| kiln.supported_profiles.contains(&batch.profile_id))
        .collect();
    if profile_kilns.is_empty() {
        reasons.insert("PROFILE_UNSUPPORTED".to_owned());
    } else {
        let capacity_kilns: Vec<_> = profile_kilns
            .iter()
            .filter(|kiln| kiln.capacity_volume_units >= batch.dry_volume_units)
            .collect();
        if capacity_kilns.is_empty() {
            reasons.insert("BATCH_EXCEEDS_KILN_CAPACITY".to_owned());
        } else if capacity_kilns
            .iter()
            .all(|kiln| kiln.max_power_kw < batch.heat_demand_kw)
        {
            reasons.insert("KILN_POWER_INSUFFICIENT".to_owned());
        }
    }
    if batch.duration_hours > scenario.horizon_hours {
        reasons.insert("DURATION_EXCEEDS_HORIZON".to_owned());
    }
    if batch.achieved_moisture_bp > batch.target_moisture_bp {
        reasons.insert("MOISTURE_TARGET_MISSED".to_owned());
    }
    if batch.requires_grade_evidence && batch.grade_evidence_status != "qualified" {
        reasons.insert("GRADE_EVIDENCE_UNQUALIFIED".to_owned());
    }
    if batch.requires_inspection && !batch.inspection_pass {
        reasons.insert("DRYING_QUALITY_GATE_FAILED".to_owned());
    }
    reasons.into_iter().collect()
}

fn gate_blocked(scenario: &Scenario) -> BTreeMap<String, Vec<String>> {
    scenario
        .batches
        .iter()
        .filter_map(|batch| {
            let reasons = gate_reasons(scenario, batch);
            (!reasons.is_empty()).then(|| (batch.batch_id.clone(), reasons))
        })
        .collect()
}

fn batch_by_id<'a>(scenario: &'a Scenario, batch_id: &str) -> Option<&'a Batch> {
    scenario
        .batches
        .iter()
        .find(|batch| batch.batch_id == batch_id)
}

fn kiln_by_id<'a>(scenario: &'a Scenario, kiln_id: &str) -> Option<&'a Kiln> {
    scenario.kilns.iter().find(|kiln| kiln.kiln_id == kiln_id)
}

fn can_place(
    scenario: &Scenario,
    batch: &Batch,
    kiln: &Kiln,
    start_hour: u32,
    current: &[Placement],
) -> bool {
    if !gate_reasons(scenario, batch).is_empty()
        || !kiln.supported_profiles.contains(&batch.profile_id)
        || kiln.capacity_volume_units < batch.dry_volume_units
        || kiln.max_power_kw < batch.heat_demand_kw
        || start_hour < batch.ready_hour
    {
        return false;
    }
    let Some(finish_hour) = start_hour.checked_add(batch.duration_hours) else {
        return false;
    };
    if finish_hour > scenario.horizon_hours {
        return false;
    }
    for existing in current {
        if existing.kiln_id == kiln.kiln_id
            && start_hour < existing.finish_hour
            && existing.start_hour < finish_hour
        {
            return false;
        }
    }
    for hour in start_hour..finish_hour {
        let mut used_kw = u64::from(batch.heat_demand_kw);
        for existing in current {
            if existing.start_hour <= hour && hour < existing.finish_hour {
                let Some(existing_batch) = batch_by_id(scenario, &existing.batch_id) else {
                    return false;
                };
                used_kw += u64::from(existing_batch.heat_demand_kw);
            }
        }
        if used_kw > u64::from(scenario.shared_power_limit_kw) {
            return false;
        }
    }
    true
}

fn scheduled_ids(placements: &[Placement]) -> Vec<String> {
    let mut values = placements.to_vec();
    values.sort_by(|left, right| {
        (left.start_hour, &left.batch_id, &left.kiln_id).cmp(&(
            right.start_hour,
            &right.batch_id,
            &right.kiln_id,
        ))
    });
    values
        .into_iter()
        .map(|placement| placement.batch_id)
        .collect()
}

fn metrics(scenario: &Scenario, placements: &[Placement]) -> Check<Metrics> {
    let mut volume = 0_u64;
    let mut makespan = 0_u32;
    let mut wait = 0_u64;
    let mut energy = 0_u64;
    let mut busy_kiln_hours = 0_u64;
    for placement in placements {
        let batch = batch_by_id(scenario, &placement.batch_id)
            .ok_or_else(|| format!("unknown batch in placement {}", placement.batch_id))?;
        volume = volume
            .checked_add(batch.dry_volume_units)
            .ok_or_else(|| "scheduled volume overflow".to_owned())?;
        makespan = makespan.max(placement.finish_hour);
        wait = wait
            .checked_add(u64::from(placement.start_hour - batch.ready_hour))
            .ok_or_else(|| "wait-hours overflow".to_owned())?;
        energy = energy
            .checked_add(u64::from(batch.heat_demand_kw) * u64::from(batch.duration_hours))
            .ok_or_else(|| "energy-unit-hours overflow".to_owned())?;
        busy_kiln_hours = busy_kiln_hours
            .checked_add(u64::from(batch.duration_hours))
            .ok_or_else(|| "kiln-hours overflow".to_owned())?;
    }
    let available_kiln_hours = u64::from(scenario.horizon_hours)
        .checked_mul(scenario.kilns.len() as u64)
        .ok_or_else(|| "available kiln-hours overflow".to_owned())?;
    Ok(Metrics {
        conforming_dry_volume_units: volume,
        makespan_hour: makespan,
        total_wait_hours: wait,
        energy_unit_hours: energy,
        busy_kiln_hours,
        available_kiln_hours,
    })
}

fn independently_validate_schedule(scenario: &Scenario, placements: &[Placement]) -> Check<()> {
    let mut seen = BTreeSet::new();
    let mut checked: Vec<Placement> = Vec::new();
    let mut power_events: BTreeMap<u32, i128> = BTreeMap::new();

    for placement in placements {
        if !seen.insert(placement.batch_id.clone()) {
            return Err(format!("duplicate batch scheduled: {}", placement.batch_id));
        }
        let batch = batch_by_id(scenario, &placement.batch_id)
            .ok_or_else(|| format!("unknown batch scheduled: {}", placement.batch_id))?;
        let kiln = kiln_by_id(scenario, &placement.kiln_id)
            .ok_or_else(|| format!("unknown kiln scheduled: {}", placement.kiln_id))?;

        if !gate_reasons(scenario, batch).is_empty() {
            return Err(format!(
                "quality-gated batch scheduled: {}",
                placement.batch_id
            ));
        }
        if !kiln.supported_profiles.contains(&batch.profile_id)
            || kiln.capacity_volume_units < batch.dry_volume_units
            || kiln.max_power_kw < batch.heat_demand_kw
        {
            return Err(format!(
                "kiln is incompatible with batch {}",
                placement.batch_id
            ));
        }
        if placement.start_hour < batch.ready_hour
            || placement.finish_hour > scenario.horizon_hours
            || placement.start_hour >= placement.finish_hour
        {
            return Err(format!(
                "batch outside release/horizon window: {}",
                placement.batch_id
            ));
        }
        let expected_finish = placement
            .start_hour
            .checked_add(batch.duration_hours)
            .ok_or_else(|| "schedule time overflow".to_owned())?;
        if expected_finish != placement.finish_hour {
            return Err(format!(
                "finish time mismatches duration: {}",
                placement.batch_id
            ));
        }

        for previous in &checked {
            if previous.kiln_id == placement.kiln_id
                && placement.start_hour < previous.finish_hour
                && previous.start_hour < placement.finish_hour
            {
                return Err(format!("kiln reservation overlap: {}", placement.batch_id));
            }
        }

        let power = i128::from(batch.heat_demand_kw);
        *power_events.entry(placement.start_hour).or_default() += power;
        *power_events.entry(placement.finish_hour).or_default() -= power;
        checked.push(placement.clone());
    }

    let mut active_power_kw = 0_i128;
    for (hour, delta_kw) in power_events {
        active_power_kw += delta_kw;
        if active_power_kw < 0 || active_power_kw > i128::from(scenario.shared_power_limit_kw) {
            return Err(format!("shared power envelope violated at hour {hour}"));
        }
    }
    Ok(())
}

fn baseline_schedule(scenario: &Scenario) -> Vec<Placement> {
    let blocked = gate_blocked(scenario);
    let mut jobs: Vec<_> = scenario
        .batches
        .iter()
        .filter(|batch| !blocked.contains_key(&batch.batch_id))
        .cloned()
        .collect();
    jobs.sort_by(|left, right| {
        (left.ready_hour, &left.batch_id).cmp(&(right.ready_hour, &right.batch_id))
    });
    let mut kilns = scenario.kilns.clone();
    kilns.sort_by(|left, right| left.kiln_id.cmp(&right.kiln_id));
    let mut placements = Vec::new();
    for batch in jobs {
        if batch.duration_hours > scenario.horizon_hours {
            continue;
        }
        let latest_start = scenario.horizon_hours - batch.duration_hours;
        if batch.ready_hour > latest_start {
            continue;
        }
        let mut found = None;
        'starts: for start in batch.ready_hour..=latest_start {
            for kiln in &kilns {
                if can_place(scenario, &batch, kiln, start, &placements) {
                    found = Some(Placement {
                        batch_id: batch.batch_id.clone(),
                        kiln_id: kiln.kiln_id.clone(),
                        start_hour: start,
                        finish_hour: start + batch.duration_hours,
                    });
                    break 'starts;
                }
            }
        }
        if let Some(placement) = found {
            placements.push(placement);
        }
    }
    placements.sort_by(|left, right| {
        (left.start_hour, &left.batch_id, &left.kiln_id).cmp(&(
            right.start_hour,
            &right.batch_id,
            &right.kiln_id,
        ))
    });
    placements
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Score {
    volume: u64,
    makespan: u32,
    energy: u64,
    wait: u64,
    signature: String,
}

fn schedule_signature(placements: &[Placement]) -> String {
    let mut values: Vec<_> = placements
        .iter()
        .map(|p| format!("{}@{}@{}", p.batch_id, p.start_hour, p.kiln_id))
        .collect();
    values.sort();
    values.join("|")
}

fn score_is_better(candidate: &Score, current: &Score) -> bool {
    if candidate.volume != current.volume {
        return candidate.volume > current.volume;
    }
    if candidate.makespan != current.makespan {
        return candidate.makespan < current.makespan;
    }
    if candidate.energy != current.energy {
        return candidate.energy < current.energy;
    }
    if candidate.wait != current.wait {
        return candidate.wait < current.wait;
    }
    candidate.signature < current.signature
}

fn score(scenario: &Scenario, placements: &[Placement]) -> Check<Score> {
    let metric = metrics(scenario, placements)?;
    Ok(Score {
        volume: metric.conforming_dry_volume_units,
        makespan: metric.makespan_hour,
        energy: metric.energy_unit_hours,
        wait: metric.total_wait_hours,
        signature: schedule_signature(placements),
    })
}

struct Search<'a> {
    scenario: &'a Scenario,
    jobs: Vec<Batch>,
    nodes: u64,
    exhausted: bool,
    best_score: Option<Score>,
    best_placements: Vec<Placement>,
}

impl Search<'_> {
    fn visit(&mut self, index: usize, current: &mut Vec<Placement>) {
        if self.nodes >= SEARCH_NODE_LIMIT {
            self.exhausted = true;
            return;
        }
        self.nodes += 1;
        if index == self.jobs.len() {
            if let Ok(candidate) = score(self.scenario, current) {
                if self
                    .best_score
                    .as_ref()
                    .is_none_or(|best| score_is_better(&candidate, best))
                {
                    self.best_score = Some(candidate);
                    self.best_placements = current.clone();
                }
            }
            return;
        }
        let batch = self.jobs[index].clone();
        let mut kilns = self.scenario.kilns.clone();
        kilns.sort_by(|left, right| left.kiln_id.cmp(&right.kiln_id));
        if batch.duration_hours <= self.scenario.horizon_hours {
            let latest_start = self.scenario.horizon_hours - batch.duration_hours;
            if batch.ready_hour <= latest_start {
                for start in batch.ready_hour..=latest_start {
                    for kiln in &kilns {
                        if can_place(self.scenario, &batch, kiln, start, current) {
                            current.push(Placement {
                                batch_id: batch.batch_id.clone(),
                                kiln_id: kiln.kiln_id.clone(),
                                start_hour: start,
                                finish_hour: start + batch.duration_hours,
                            });
                            self.visit(index + 1, current);
                            current.pop();
                            if self.exhausted {
                                return;
                            }
                        }
                    }
                }
            }
        }
        self.visit(index + 1, current);
    }
}

fn exhaustive_schedule(scenario: &Scenario) -> (Vec<Placement>, u64, bool) {
    let blocked = gate_blocked(scenario);
    let mut jobs: Vec<_> = scenario
        .batches
        .iter()
        .filter(|batch| !blocked.contains_key(&batch.batch_id))
        .cloned()
        .collect();
    jobs.sort_by(|left, right| {
        (left.ready_hour, &left.batch_id).cmp(&(right.ready_hour, &right.batch_id))
    });
    let mut search = Search {
        scenario,
        jobs,
        nodes: 0,
        exhausted: false,
        best_score: None,
        best_placements: Vec::new(),
    };
    search.visit(0, &mut Vec::new());
    (search.best_placements, search.nodes, search.exhausted)
}

fn analyze_scenario(scenario: &Scenario) -> Check<ScenarioResult> {
    let gate_blocked_reasons = gate_blocked(scenario);
    let baseline = baseline_schedule(scenario);
    independently_validate_schedule(scenario, &baseline)?;
    let baseline_metrics = metrics(scenario, &baseline)?;
    let (oracle, explored_nodes, exhausted) = exhaustive_schedule(scenario);
    independently_validate_schedule(scenario, &oracle)?;
    let oracle_metrics = metrics(scenario, &oracle)?;
    let baseline_ids = scheduled_ids(&baseline);
    let oracle_ids = scheduled_ids(&oracle);
    let eligible_ids: BTreeSet<_> = scenario
        .batches
        .iter()
        .filter(|batch| !gate_blocked_reasons.contains_key(&batch.batch_id))
        .map(|batch| batch.batch_id.clone())
        .collect();
    let baseline_ids_set: BTreeSet<_> = baseline_ids.iter().cloned().collect();
    let oracle_ids_set: BTreeSet<_> = oracle_ids.iter().cloned().collect();
    let baseline_unscheduled_eligible_ids = eligible_ids
        .difference(&baseline_ids_set)
        .cloned()
        .collect();
    let oracle_unscheduled_eligible_ids =
        eligible_ids.difference(&oracle_ids_set).cloned().collect();
    let oracle_disposition = if exhausted {
        "indeterminate_search_limit"
    } else {
        "optimal"
    }
    .to_owned();
    let baseline_volume_gap_units = oracle_metrics
        .conforming_dry_volume_units
        .saturating_sub(baseline_metrics.conforming_dry_volume_units);
    let expected = &scenario.expected;
    let expected_control_matched = !exhausted
        && gate_blocked_reasons == expected.gate_blocked_reasons
        && baseline_ids == expected.baseline_scheduled_batch_ids
        && baseline_metrics.conforming_dry_volume_units
            == expected.baseline_conforming_volume_units
        && oracle_ids == expected.oracle_scheduled_batch_ids
        && oracle_metrics.conforming_dry_volume_units == expected.oracle_conforming_volume_units
        && oracle_metrics.makespan_hour == expected.oracle_makespan_hour;
    Ok(ScenarioResult {
        scenario_id: scenario.scenario_id.clone(),
        gate_blocked_reasons,
        baseline_scheduled_batch_ids: baseline_ids,
        baseline_metrics,
        baseline_unscheduled_eligible_ids,
        oracle_disposition,
        oracle_scheduled_batch_ids: oracle_ids,
        oracle_metrics,
        oracle_unscheduled_eligible_ids,
        baseline_volume_gap_units,
        explored_nodes,
        schedule_independently_validated: true,
        expected_control_matched,
    })
}

fn source_commit_from_args(args: &[String]) -> Check<String> {
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: wood-flow-kiln-bench <corpus.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = args[3].to_ascii_lowercase();
    if source_commit.len() != 40
        || !source_commit
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err("--source-commit must be exactly 40 hexadecimal characters".to_owned());
    }
    Ok(source_commit)
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: wood-flow-kiln-bench <corpus.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = source_commit_from_args(&args)?;
    let bytes = fs::read(&args[1]).map_err(|error| format!("cannot read corpus: {error}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    validate_corpus(&corpus)?;
    let corpus_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let scenarios = corpus
        .scenarios
        .iter()
        .map(analyze_scenario)
        .collect::<Check<Vec<_>>>()?;
    let expected_controls_matched = scenarios
        .iter()
        .filter(|scenario| scenario.expected_control_matched)
        .count();
    let all_controls_matched = expected_controls_matched == scenarios.len();
    let receipt = Receipt {
        result: if all_controls_matched {
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
        expected_controls_matched,
        search_node_limit: SEARCH_NODE_LIMIT,
        scenarios,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize receipt: {error}"))?
    );
    Ok(if all_controls_matched {
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
            "../../../docs/tech/wood-flow-kiln-corpus-v0.1.json"
        ))
        .expect("frozen synthetic corpus must deserialize")
    }

    #[test]
    fn frozen_corpus_matches_known_answers() {
        let fixture = corpus();
        validate_corpus(&fixture).expect("corpus metadata must be valid");
        let results = fixture
            .scenarios
            .iter()
            .map(analyze_scenario)
            .collect::<Check<Vec<_>>>()
            .expect("all schedules should validate");
        assert_eq!(5, results.len());
        assert!(results.iter().all(|result| result.expected_control_matched));
    }

    #[test]
    fn exhaustive_oracle_beats_greedy_on_conforming_volume() {
        let fixture = corpus();
        let scenario = fixture
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "GREEDY_BASELINE_REGRESSION")
            .expect("greedy regression fixture exists");
        let result = analyze_scenario(scenario).expect("schedule should be analyzable");
        assert_eq!(10, result.baseline_metrics.conforming_dry_volume_units);
        assert_eq!(100, result.oracle_metrics.conforming_dry_volume_units);
        assert_eq!(90, result.baseline_volume_gap_units);
    }

    #[test]
    fn shared_power_envelope_breach_is_rejected() {
        let fixture = corpus();
        let scenario = fixture
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "SHARED_POWER_FORCES_SERIAL")
            .expect("shared-power fixture exists");
        let bad = vec![
            Placement {
                batch_id: "BATCH_A".to_owned(),
                kiln_id: "KILN_A".to_owned(),
                start_hour: 0,
                finish_hour: 2,
            },
            Placement {
                batch_id: "BATCH_B".to_owned(),
                kiln_id: "KILN_B".to_owned(),
                start_hour: 0,
                finish_hour: 2,
            },
        ];
        assert!(independently_validate_schedule(scenario, &bad).is_err());
    }

    #[test]
    fn same_kiln_overlap_is_rejected() {
        let fixture = corpus();
        let scenario = fixture
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "SHARED_POWER_FORCES_SERIAL")
            .expect("shared-power fixture exists");
        let bad = vec![
            Placement {
                batch_id: "BATCH_A".to_owned(),
                kiln_id: "KILN_A".to_owned(),
                start_hour: 0,
                finish_hour: 2,
            },
            Placement {
                batch_id: "BATCH_B".to_owned(),
                kiln_id: "KILN_A".to_owned(),
                start_hour: 1,
                finish_hour: 3,
            },
        ];
        assert!(independently_validate_schedule(scenario, &bad).is_err());
    }

    #[test]
    fn unknown_json_fields_fail_closed() {
        let raw = include_str!("../../../docs/tech/wood-flow-kiln-corpus-v0.1.json").replace(
            "\"suite_id\":",
            "\"unexpected_authority\": true,\n  \"suite_id\":",
        );
        assert!(serde_json::from_str::<Corpus>(&raw).is_err());
    }
}
