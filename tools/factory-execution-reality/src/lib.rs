use std::collections::{BTreeMap, BTreeSet};

const ROUTE: &str = include_str!("../route-contract-v0.1.tsv");
const SCENARIOS: &str = include_str!("../scenarios-v0.1.tsv");
const OBSERVATIONS: &str = include_str!("../observations-v0.1.tsv");
const COVERAGE: &str = include_str!("../coverage-v0.1.tsv");

#[derive(Clone, Debug)]
struct Scenario {
    id: String,
    observable_prefix_id: String,
    decision_time_utc: String,
    hidden_future_visible_at_decision: bool,
    paired_future_id: Option<String>,
}

#[derive(Clone, Debug)]
struct Coverage {
    id: String,
    scenario_id: String,
    sensor_id: String,
    covered_zones: BTreeSet<String>,
    start_utc: String,
    end_utc: String,
    coverage_state: String,
    calibration_ids: BTreeSet<String>,
    map_generation: String,
    position_uncertainty_m: Option<f64>,
}

#[derive(Clone, Debug)]
struct Observation {
    id: String,
    scenario_id: String,
    product_id: String,
    stream_kind: String,
    source_id: String,
    source_origin_group: String,
    event_type: String,
    event_effective_utc: String,
    observed_or_ingested_utc: String,
    resource_id: String,
    location_zone: String,
    map_generation: String,
    calibration_id: String,
    coverage_id: String,
    measurement_state: String,
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub observations_checked: usize,
    pub coverage_records_used: usize,
    pub paired_future_checked: bool,
    pub contract_clear: bool,
    pub blocking_reasons: Vec<String>,
}

fn parse_rows(input: &str, columns: usize, source: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    for (index, line) in input.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if fields.len() != columns {
            return Err(format!(
                "{source}: line {} has {} columns; expected {columns}",
                index + 1,
                fields.len()
            ));
        }
        if fields.iter().any(|field| field.trim().is_empty()) {
            return Err(format!("{source}: line {} contains a blank field", index + 1));
        }
        rows.push(fields);
    }
    Ok(rows)
}

fn parse_bool(value: &str, source: &str, line: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{source}: line {line}: expected true/false")),
    }
}

fn parse_timestamp(value: &str, source: &str, line: usize) -> Result<(), String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return Err(format!("{source}: line {line}: timestamp must be UTC RFC3339 seconds"));
    }
    Ok(())
}

fn parse_scenarios(input: &str) -> Result<BTreeMap<String, Scenario>, String> {
    let rows = parse_rows(input, 10, "scenarios-v0.1.tsv")?;
    let mut scenarios = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        parse_timestamp(&row[3], "scenarios-v0.1.tsv", line)?;
        let hidden = parse_bool(&row[4], "scenarios-v0.1.tsv", line)?;
        let pair = if row[8] == "-" { None } else { Some(row[8].clone()) };
        let scenario = Scenario {
            id: row[0].clone(),
            observable_prefix_id: row[2].clone(),
            decision_time_utc: row[3].clone(),
            hidden_future_visible_at_decision: hidden,
            paired_future_id: pair,
        };
        if scenarios.insert(row[0].clone(), scenario).is_some() {
            return Err(format!("scenarios-v0.1.tsv: duplicate scenario id {}", row[0]));
        }
    }
    if scenarios.is_empty() {
        return Err("scenarios-v0.1.tsv: empty scenario manifest".to_owned());
    }
    Ok(scenarios)
}

fn parse_coverage(input: &str, scenarios: &BTreeMap<String, Scenario>) -> Result<BTreeMap<String, Coverage>, String> {
    let rows = parse_rows(input, 13, "coverage-v0.1.tsv")?;
    let mut coverages = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        if !scenarios.contains_key(&row[1]) {
            return Err(format!("coverage-v0.1.tsv: line {line}: unknown scenario {}", row[1]));
        }
        parse_timestamp(&row[5], "coverage-v0.1.tsv", line)?;
        parse_timestamp(&row[6], "coverage-v0.1.tsv", line)?;
        if row[5] >= row[6] {
            return Err(format!("coverage-v0.1.tsv: line {line}: coverage interval is empty or inverted"));
        }
        if !matches!(row[7].as_str(), "ADEQUATE" | "GAP" | "AMBIGUOUS") {
            return Err(format!("coverage-v0.1.tsv: line {line}: invalid coverage state"));
        }
        let uncertainty = if row[10] == "-" {
            None
        } else {
            let value = row[10].parse::<f64>().map_err(|_| {
                format!("coverage-v0.1.tsv: line {line}: invalid position uncertainty")
            })?;
            if !value.is_finite() || value < 0.0 {
                return Err(format!("coverage-v0.1.tsv: line {line}: invalid position uncertainty"));
            }
            Some(value)
        };
        let coverage = Coverage {
            id: row[0].clone(),
            scenario_id: row[1].clone(),
            sensor_id: row[2].clone(),
            covered_zones: row[4].split(';').map(str::to_owned).collect(),
            start_utc: row[5].clone(),
            end_utc: row[6].clone(),
            coverage_state: row[7].clone(),
            calibration_ids: row[8].split(';').map(str::to_owned).collect(),
            map_generation: row[9].clone(),
            position_uncertainty_m: uncertainty,
        };
        if coverages.insert(row[0].clone(), coverage).is_some() {
            return Err(format!("coverage-v0.1.tsv: duplicate coverage id {}", row[0]));
        }
    }
    if coverages.is_empty() {
        return Err("coverage-v0.1.tsv: empty coverage manifest".to_owned());
    }
    Ok(coverages)
}

fn parse_observations(input: &str, scenarios: &BTreeMap<String, Scenario>) -> Result<BTreeMap<String, Observation>, String> {
    let rows = parse_rows(input, 16, "observations-v0.1.tsv")?;
    let mut observations = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        if !scenarios.contains_key(&row[1]) {
            return Err(format!("observations-v0.1.tsv: line {line}: unknown scenario {}", row[1]));
        }
        for column in [7, 8] {
            parse_timestamp(&row[column], "observations-v0.1.tsv", line)?;
        }
        if row[7] > row[8] {
            return Err(format!("observations-v0.1.tsv: line {line}: ingest time precedes event time"));
        }
        if row[2] == "-" || row[4] == "-" || row[5] == "-" || row[6] == "-" {
            return Err(format!("observations-v0.1.tsv: line {line}: identity/source/event fields may not be '-'"));
        }
        let observation = Observation {
            id: row[0].clone(),
            scenario_id: row[1].clone(),
            product_id: row[2].clone(),
            stream_kind: row[3].clone(),
            source_id: row[4].clone(),
            source_origin_group: row[5].clone(),
            event_type: row[6].clone(),
            event_effective_utc: row[7].clone(),
            observed_or_ingested_utc: row[8].clone(),
            resource_id: row[9].clone(),
            location_zone: row[10].clone(),
            map_generation: row[11].clone(),
            calibration_id: row[12].clone(),
            coverage_id: row[13].clone(),
            measurement_state: row[14].clone(),
            value: row[15].clone(),
        };
        if observations.insert(row[0].clone(), observation).is_some() {
            return Err(format!("observations-v0.1.tsv: duplicate observation id {}", row[0]));
        }
    }
    Ok(observations)
}

fn sensor_matches(sensor_id: &str, source_id: &str) -> bool {
    sensor_id.split([';', '/']).any(|candidate| candidate == source_id)
}

fn calibration_matches(coverage: &Coverage, calibration_id: &str) -> bool {
    if calibration_id == "-" {
        coverage.calibration_ids.contains("-")
    } else {
        coverage.calibration_ids.contains(calibration_id)
    }
}

fn validate_observations(
    observations: &BTreeMap<String, Observation>,
    coverages: &BTreeMap<String, Coverage>,
    scenarios: &BTreeMap<String, Scenario>,
) -> Result<BTreeMap<String, Vec<Observation>>, String> {
    let mut by_scenario: BTreeMap<String, Vec<Observation>> = BTreeMap::new();
    for observation in observations.values() {
        let scenario = scenarios.get(&observation.scenario_id).expect("parser validated scenario");
        if observation.observed_or_ingested_utc > scenario.decision_time_utc {
            return Err(format!(
                "observation {} was not observable by scenario {} decision time",
                observation.id, scenario.id
            ));
        }
        let coverage = coverages.get(&observation.coverage_id).ok_or_else(|| {
            format!("observation {} references missing coverage {}", observation.id, observation.coverage_id)
        })?;
        if coverage.scenario_id != observation.scenario_id {
            return Err(format!("observation {} references coverage from another scenario", observation.id));
        }
        if !sensor_matches(&coverage.sensor_id, &observation.source_id) {
            return Err(format!("observation {} source does not match declared coverage sensor", observation.id));
        }
        if observation.event_effective_utc < coverage.start_utc
            || observation.event_effective_utc > coverage.end_utc
        {
            return Err(format!("observation {} event time lies outside declared coverage window", observation.id));
        }
        if !coverage.covered_zones.contains("ALL")
            && !coverage.covered_zones.contains(&observation.location_zone)
        {
            return Err(format!("observation {} zone lies outside declared coverage", observation.id));
        }
        if coverage.map_generation != observation.map_generation {
            return Err(format!("observation {} map generation differs from coverage profile", observation.id));
        }
        if !calibration_matches(coverage, &observation.calibration_id) {
            return Err(format!("observation {} calibration differs from coverage profile", observation.id));
        }
        if observation.stream_kind == "RTLS" {
            if coverage.position_uncertainty_m.is_none() || observation.location_zone == "-" {
                return Err(format!("RTLS observation {} lacks declared zone uncertainty/scope", observation.id));
            }
        }
        by_scenario.entry(observation.scenario_id.clone()).or_default().push(observation.clone());
    }
    Ok(by_scenario)
}

fn canonical_observation_prefix(observations: &[Observation]) -> BTreeSet<String> {
    observations
        .iter()
        .map(|observation| {
            [
                observation.product_id.as_str(),
                observation.stream_kind.as_str(),
                observation.source_id.as_str(),
                observation.source_origin_group.as_str(),
                observation.event_type.as_str(),
                observation.event_effective_utc.as_str(),
                observation.observed_or_ingested_utc.as_str(),
                observation.resource_id.as_str(),
                observation.location_zone.as_str(),
                observation.map_generation.as_str(),
                observation.calibration_id.as_str(),
                observation.measurement_state.as_str(),
                observation.value.as_str(),
            ]
            .join("|")
        })
        .collect()
}

fn canonical_coverage_prefix(coverages: &[&Coverage]) -> BTreeSet<String> {
    coverages
        .iter()
        .map(|coverage| {
            [
                coverage.sensor_id.as_str(),
                coverage.covered_zones.iter().cloned().collect::<Vec<_>>().join(";").as_str(),
                coverage.start_utc.as_str(),
                coverage.end_utc.as_str(),
                coverage.coverage_state.as_str(),
                coverage.calibration_ids.iter().cloned().collect::<Vec<_>>().join(";").as_str(),
                coverage.map_generation.as_str(),
                coverage.position_uncertainty_m.map(|v| v.to_string()).unwrap_or_else(|| "-".to_owned()).as_str(),
            ]
            .join("|")
        })
        .collect()
}

fn verify_paired_future(
    scenarios: &BTreeMap<String, Scenario>,
    observations: &BTreeMap<String, Vec<Observation>>,
    coverages: &BTreeMap<String, Coverage>,
) -> Result<BTreeSet<String>, String> {
    let mut pairs: BTreeMap<String, Vec<&Scenario>> = BTreeMap::new();
    for scenario in scenarios.values() {
        if let Some(pair_id) = scenario.paired_future_id.as_deref() {
            pairs.entry(pair_id.to_owned()).or_default().push(scenario);
        }
    }
    let mut checked = BTreeSet::new();
    for (pair_id, members) in pairs {
        if members.len() != 2 {
            return Err(format!("paired future {pair_id} must contain exactly two scenarios"));
        }
        let left = members[0];
        let right = members[1];
        if left.hidden_future_visible_at_decision || right.hidden_future_visible_at_decision {
            return Err(format!("paired future {pair_id} leaks future event visibility"));
        }
        if left.observable_prefix_id != right.observable_prefix_id
            || left.decision_time_utc != right.decision_time_utc
        {
            return Err(format!("paired future {pair_id} does not share prefix/decision time"));
        }
        let left_obs = canonical_observation_prefix(observations.get(&left.id).map(Vec::as_slice).unwrap_or(&[]));
        let right_obs = canonical_observation_prefix(observations.get(&right.id).map(Vec::as_slice).unwrap_or(&[]));
        if left_obs != right_obs {
            return Err(format!("paired future {pair_id} observed prefixes differ"));
        }
        let left_cov = coverages.values().filter(|coverage| coverage.scenario_id == left.id).collect::<Vec<_>>();
        let right_cov = coverages.values().filter(|coverage| coverage.scenario_id == right.id).collect::<Vec<_>>();
        if canonical_coverage_prefix(&left_cov) != canonical_coverage_prefix(&right_cov) {
            return Err(format!("paired future {pair_id} coverage prefixes differ"));
        }
        checked.insert(left.id.clone());
        checked.insert(right.id.clone());
    }
    Ok(checked)
}

pub fn validate_reference_corpus() -> Result<Vec<ScenarioResult>, String> {
    let _route_rows = parse_rows(ROUTE, 10, "route-contract-v0.1.tsv")?;
    let scenarios = parse_scenarios(SCENARIOS)?;
    let coverages = parse_coverage(COVERAGE, &scenarios)?;
    let observations = parse_observations(OBSERVATIONS, &scenarios)?;
    let by_scenario = validate_observations(&observations, &coverages, &scenarios)?;
    let paired = verify_paired_future(&scenarios, &by_scenario, &coverages)?;

    let mut results = Vec::new();
    for scenario in scenarios.values() {
        let checked_obs = by_scenario.get(&scenario.id).map(Vec::len).unwrap_or(0);
        let used_coverage = coverages.values().filter(|coverage| coverage.scenario_id == scenario.id).count();
        results.push(ScenarioResult {
            scenario_id: scenario.id.clone(),
            observations_checked: checked_obs,
            coverage_records_used: used_coverage,
            paired_future_checked: paired.contains(&scenario.id),
            contract_clear: true,
            blocking_reasons: Vec::new(),
        });
    }
    Ok(results)
}

pub fn render_tsv(results: &[ScenarioResult]) -> String {
    let mut output = String::from(
        "scenario_id\tobservations_checked\tcoverage_records\tpaired_future_checked\tcontract_clear\tblocking_reasons\n",
    );
    for result in results {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            result.scenario_id,
            result.observations_checked,
            result.coverage_records_used,
            result.paired_future_checked,
            result.contract_clear,
            if result.blocking_reasons.is_empty() { "-".to_owned() } else { result.blocking_reasons.join(";") }
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_input_corpus_passes_integrity_contract() {
        let results = validate_reference_corpus().expect("reference execution corpus should validate");
        assert_eq!(results.len(), 12);
        assert!(results.iter().all(|result| result.contract_clear));
    }

    #[test]
    fn paired_future_scenarios_are_verified() {
        let results = validate_reference_corpus().expect("reference execution corpus should validate");
        let left = results.iter().find(|result| result.scenario_id == "S10").unwrap();
        let right = results.iter().find(|result| result.scenario_id == "S11").unwrap();
        assert!(left.paired_future_checked);
        assert!(right.paired_future_checked);
    }

    #[test]
    fn source_coverage_mismatch_is_rejected() {
        let mut coverages = parse_coverage(COVERAGE, &parse_scenarios(SCENARIOS).unwrap()).unwrap();
        let observations = parse_observations(OBSERVATIONS, &parse_scenarios(SCENARIOS).unwrap()).unwrap();
        let first = observations.values().next().unwrap();
        coverages.get_mut(&first.coverage_id).unwrap().sensor_id = "WRONG-SENSOR".to_owned();
        assert!(validate_observations(&observations, &coverages, &parse_scenarios(SCENARIOS).unwrap()).is_err());
    }

    #[test]
    fn late_ingest_after_decision_is_rejected() {
        let scenarios = parse_scenarios(SCENARIOS).unwrap();
        let coverages = parse_coverage(COVERAGE, &scenarios).unwrap();
        let mut observations = parse_observations(OBSERVATIONS, &scenarios).unwrap();
        let late = observations.values_mut().find(|observation| observation.scenario_id == "S05").unwrap();
        late.observed_or_ingested_utc = "2026-11-01T09:06:00Z".to_owned();
        assert!(validate_observations(&observations, &coverages, &scenarios).is_err());
    }

    #[test]
    fn wrong_map_generation_is_rejected() {
        let scenarios = parse_scenarios(SCENARIOS).unwrap();
        let mut coverages = parse_coverage(COVERAGE, &scenarios).unwrap();
        let observations = parse_observations(OBSERVATIONS, &scenarios).unwrap();
        let first = observations.values().next().unwrap();
        coverages.get_mut(&first.coverage_id).unwrap().map_generation = "MAP-WRONG".to_owned();
        assert!(validate_observations(&observations, &coverages, &scenarios).is_err());
    }
}
