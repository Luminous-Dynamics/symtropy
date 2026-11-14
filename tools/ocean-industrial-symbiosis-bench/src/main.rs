// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");
const SCHEMA_ID: &str = "luminous.symtropy.ocean-industrial-symbiosis";
const SCHEMA_VERSION: &str = "0.1.0";
const SUITE_ID: &str = "OCEAN-INDUSTRIAL-SYMBIOSIS-SYNTH-001";
const DEFAULT_CORPUS: &str = "docs/tech/OCEAN_INDUSTRIAL_SYMBIOSIS_CORPUS_V0_1.json";
const MAX_CORPUS_BYTES: usize = 250_000;
const MAX_LINKS_PER_SCENARIO: usize = 12;
const SCALE_PPM: u64 = 1_000_000;
const SCENARIO_IDS: &[&str] = &[
    "heat-water-circularity",
    "quality-and-timing-false-synergies",
    "brine-residual-hazard",
    "duplicate-benefit-claims",
    "storm-resilience-and-no-build",
];

type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StreamKind {
    Electricity,
    LowGradeHeat,
    FreshWater,
    DesalinationBrine,
    BiomassResidue,
    RecycledMarinePlastic,
    EnvironmentalData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Unit {
    KilowattHourMilli,
    CubicMetreMilli,
    KilogramMilli,
    DataBundle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PortRole {
    Producer,
    Consumer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum QualityMetric {
    None,
    TemperatureMilliC,
    PurityPpm,
    SalinityPpm,
    ContaminantPpm,
    NutrientPpm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReviewStatus {
    Reviewed,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum UsePermission {
    Permitted,
    Restricted,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EnvironmentalReview {
    Reviewed,
    ReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Port {
    port_id: String,
    node_id: String,
    role: PortRole,
    stream_kind: StreamKind,
    unit: Unit,
    capacity_units: u64,
    time_window_id: String,
    quality_metric: QualityMetric,
    quality_value: Option<i64>,
    min_quality: Option<i64>,
    max_quality: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Impacts {
    sound_units: u32,
    light_units: u32,
    sediment_units: u32,
    thermal_units: u32,
    nutrient_units: u32,
    chemical_units: u32,
}

impl Impacts {
    fn add(self, other: Self) -> Self {
        Self {
            sound_units: self.sound_units.saturating_add(other.sound_units),
            light_units: self.light_units.saturating_add(other.light_units),
            sediment_units: self.sediment_units.saturating_add(other.sediment_units),
            thermal_units: self.thermal_units.saturating_add(other.thermal_units),
            nutrient_units: self.nutrient_units.saturating_add(other.nutrient_units),
            chemical_units: self.chemical_units.saturating_add(other.chemical_units),
        }
    }

    fn within(self, budget: Self) -> bool {
        self.sound_units <= budget.sound_units
            && self.light_units <= budget.light_units
            && self.sediment_units <= budget.sediment_units
            && self.thermal_units <= budget.thermal_units
            && self.nutrient_units <= budget.nutrient_units
            && self.chemical_units <= budget.chemical_units
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateLink {
    link_id: String,
    source_port_id: String,
    consumer_port_id: String,
    quantity_units: u64,
    route_distance_km_milli: u64,
    max_distance_km_milli: u64,
    transfer_energy_kwh_milli: u64,
    net_value_milli: i64,
    /// Lifecycle change including transport and processing; negative means lower emissions.
    net_ghg_change_g_co2e: i64,
    water_saved_litres: u64,
    local_value_milli: i64,
    /// Same claim ID means the same counterfactual benefit: never credit twice in one portfolio.
    benefit_claim_id: String,
    counterfactual_id: String,
    rights_review: ReviewStatus,
    use_permission: UsePermission,
    environmental_review: EnvironmentalReview,
    chemical_fate_known: bool,
    monitoring_coverage_ppm: u64,
    impacts: Impacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    description: String,
    min_monitoring_coverage_ppm: u64,
    impact_budget: Impacts,
    ports: Vec<Port>,
    candidate_links: Vec<CandidateLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    schema_version: String,
    suite_id: String,
    evidence_class: String,
    claim_ceiling: String,
    seed: u64,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LinkIssueCode {
    MissingPort,
    WrongPortRole,
    StreamMismatch,
    UnitMismatch,
    WrongCanonicalUnit,
    ZeroQuantity,
    TimeWindowMismatch,
    QualityMetricMismatch,
    MissingQualityEvidence,
    QualityOutsideSinkRange,
    InvalidSinkQualityRange,
    DistanceExceeded,
    RightsReviewIncomplete,
    UseNotPermitted,
    EnvironmentalReviewIncomplete,
    ChemicalFateUnknown,
    MonitoringCoverageInsufficient,
    MissingBenefitClaimId,
    MissingCounterfactualId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RejectedLink {
    link_id: String,
    issues: Vec<LinkIssueCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortfolioOutcome {
    candidate_link_ids: Vec<String>,
    net_value_milli: i64,
    net_ghg_change_g_co2e: i64,
    water_saved_litres: u64,
    local_value_milli: i64,
    transfer_energy_kwh_milli: u64,
    impacts: Impacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioReport {
    scenario_id: String,
    individually_eligible_link_count: usize,
    rejected_links: Vec<RejectedLink>,
    feasible_portfolio_count: usize,
    pareto_frontier: Vec<PortfolioOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvariantResult {
    invariant_id: String,
    passed: bool,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_id: String,
    schema_version: String,
    tool_version: String,
    suite_id: String,
    corpus_sha256: String,
    source_commit: String,
    evidence_class: String,
    claim_ceiling: String,
    seed: u64,
    scenario_count: usize,
    passed_invariant_count: usize,
    failed_invariant_count: usize,
    result: String,
    invariants: Vec<InvariantResult>,
    scenarios: Vec<ScenarioReport>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn canonical_unit(kind: StreamKind) -> Unit {
    match kind {
        StreamKind::Electricity | StreamKind::LowGradeHeat => Unit::KilowattHourMilli,
        StreamKind::FreshWater | StreamKind::DesalinationBrine => Unit::CubicMetreMilli,
        StreamKind::BiomassResidue | StreamKind::RecycledMarinePlastic => Unit::KilogramMilli,
        StreamKind::EnvironmentalData => Unit::DataBundle,
    }
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.schema_id != SCHEMA_ID || corpus.schema_version != SCHEMA_VERSION {
        return Err("unsupported schema ID/version".into());
    }
    if corpus.suite_id != SUITE_ID || corpus.evidence_class != "SyntheticAdversarial" {
        return Err("unexpected suite ID or non-synthetic evidence class".into());
    }
    if corpus.claim_ceiling.trim().is_empty() {
        return Err("claim ceiling must be non-empty".into());
    }
    if corpus.scenarios.len() != SCENARIO_IDS.len() {
        return Err(format!("expected {} frozen scenarios", SCENARIO_IDS.len()));
    }
    for (index, scenario) in corpus.scenarios.iter().enumerate() {
        if scenario.scenario_id != SCENARIO_IDS[index] {
            return Err(format!("scenario {index} must be {}", SCENARIO_IDS[index]));
        }
        if scenario.description.trim().is_empty()
            || scenario.ports.is_empty()
            || scenario.candidate_links.is_empty()
        {
            return Err(format!(
                "scenario {} missing description/ports/links",
                scenario.scenario_id
            ));
        }
        if scenario.min_monitoring_coverage_ppm > SCALE_PPM {
            return Err("minimum monitoring coverage exceeds 1,000,000 ppm".into());
        }
        if scenario.candidate_links.len() > MAX_LINKS_PER_SCENARIO {
            return Err(format!(
                "scenario {} exceeds exact portfolio size limit",
                scenario.scenario_id
            ));
        }
        let mut port_ids = HashSet::new();
        for port in &scenario.ports {
            if port.port_id.trim().is_empty()
                || port.node_id.trim().is_empty()
                || port.time_window_id.trim().is_empty()
                || !port_ids.insert(port.port_id.as_str())
            {
                return Err(format!(
                    "scenario {} has blank/duplicate port identity",
                    scenario.scenario_id
                ));
            }
            if port.capacity_units == 0 {
                return Err(format!("port {} capacity must be positive", port.port_id));
            }
            if port.role == PortRole::Producer {
                if port.quality_metric != QualityMetric::None && port.quality_value.is_none() {
                    return Err(format!(
                        "source port {} lacks declared quality",
                        port.port_id
                    ));
                }
                if port.min_quality.is_some() || port.max_quality.is_some() {
                    return Err(format!(
                        "source port {} cannot carry consumer quality bounds",
                        port.port_id
                    ));
                }
            } else if port.quality_metric != QualityMetric::None {
                if port.min_quality.is_none() || port.max_quality.is_none() {
                    return Err(format!(
                        "consumer port {} needs both quality bounds",
                        port.port_id
                    ));
                }
                if port.min_quality > port.max_quality {
                    return Err(format!(
                        "consumer port {} has inverted quality range",
                        port.port_id
                    ));
                }
                if port.quality_value.is_some() {
                    return Err(format!(
                        "consumer port {} cannot claim source quality value",
                        port.port_id
                    ));
                }
            }
        }
        let mut link_ids = HashSet::new();
        for link in &scenario.candidate_links {
            if link.link_id.trim().is_empty() || !link_ids.insert(link.link_id.as_str()) {
                return Err(format!(
                    "scenario {} has blank/duplicate link IDs",
                    scenario.scenario_id
                ));
            }
            if link.monitoring_coverage_ppm > SCALE_PPM {
                return Err(format!(
                    "link {} monitoring coverage out of range",
                    link.link_id
                ));
            }
            if link.quantity_units == 0 {
                // Invalid links are useful as negative controls, so corpus validity keeps them representable.
            }
        }
    }
    Ok(())
}

fn corpus_from_bytes(bytes: &[u8]) -> Check<Corpus> {
    if bytes.len() > MAX_CORPUS_BYTES {
        return Err(format!("corpus exceeds byte limit {MAX_CORPUS_BYTES}"));
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    validate_corpus(&corpus)?;
    Ok(corpus)
}

fn port<'a>(scenario: &'a Scenario, port_id: &str) -> Option<&'a Port> {
    scenario.ports.iter().find(|port| port.port_id == port_id)
}

fn link_issues(scenario: &Scenario, link: &CandidateLink) -> Vec<LinkIssueCode> {
    let mut issues = Vec::new();
    let Some(source) = port(scenario, &link.source_port_id) else {
        return vec![LinkIssueCode::MissingPort];
    };
    let Some(consumer) = port(scenario, &link.consumer_port_id) else {
        return vec![LinkIssueCode::MissingPort];
    };
    if source.role != PortRole::Producer || consumer.role != PortRole::Consumer {
        issues.push(LinkIssueCode::WrongPortRole);
    }
    if source.stream_kind != consumer.stream_kind {
        issues.push(LinkIssueCode::StreamMismatch);
    }
    if source.unit != consumer.unit {
        issues.push(LinkIssueCode::UnitMismatch);
    }
    if source.unit != canonical_unit(source.stream_kind)
        || consumer.unit != canonical_unit(consumer.stream_kind)
    {
        issues.push(LinkIssueCode::WrongCanonicalUnit);
    }
    if link.quantity_units == 0 {
        issues.push(LinkIssueCode::ZeroQuantity);
    }
    if source.time_window_id != consumer.time_window_id {
        issues.push(LinkIssueCode::TimeWindowMismatch);
    }
    if source.quality_metric != consumer.quality_metric {
        issues.push(LinkIssueCode::QualityMetricMismatch);
    }
    if source.quality_metric != QualityMetric::None {
        match (
            source.quality_value,
            consumer.min_quality,
            consumer.max_quality,
        ) {
            (Some(value), Some(min), Some(max)) if min <= max => {
                if value < min || value > max {
                    issues.push(LinkIssueCode::QualityOutsideSinkRange);
                }
            }
            _ => issues.push(LinkIssueCode::MissingQualityEvidence),
        }
    }
    if consumer
        .min_quality
        .zip(consumer.max_quality)
        .is_some_and(|(min, max)| min > max)
    {
        issues.push(LinkIssueCode::InvalidSinkQualityRange);
    }
    if link.route_distance_km_milli > link.max_distance_km_milli {
        issues.push(LinkIssueCode::DistanceExceeded);
    }
    if link.rights_review != ReviewStatus::Reviewed {
        issues.push(LinkIssueCode::RightsReviewIncomplete);
    }
    if link.use_permission != UsePermission::Permitted {
        issues.push(LinkIssueCode::UseNotPermitted);
    }
    if link.environmental_review != EnvironmentalReview::Reviewed {
        issues.push(LinkIssueCode::EnvironmentalReviewIncomplete);
    }
    if !link.chemical_fate_known {
        issues.push(LinkIssueCode::ChemicalFateUnknown);
    }
    if link.monitoring_coverage_ppm < scenario.min_monitoring_coverage_ppm {
        issues.push(LinkIssueCode::MonitoringCoverageInsufficient);
    }
    if link.benefit_claim_id.trim().is_empty() {
        issues.push(LinkIssueCode::MissingBenefitClaimId);
    }
    if link.counterfactual_id.trim().is_empty() {
        issues.push(LinkIssueCode::MissingCounterfactualId);
    }
    issues.sort_by_key(|issue| format!("{issue:?}"));
    issues.dedup();
    issues
}

fn add_port_quantity(totals: &mut BTreeMap<String, u64>, port_id: &str, quantity: u64) {
    let entry = totals.entry(port_id.to_owned()).or_default();
    *entry = entry.saturating_add(quantity);
}

fn portfolio_outcome(scenario: &Scenario, selected: &[usize]) -> Option<PortfolioOutcome> {
    let mut source_used: BTreeMap<String, u64> = BTreeMap::new();
    let mut sink_used: BTreeMap<String, u64> = BTreeMap::new();
    let mut claim_ids = HashSet::new();
    let mut impact_total = Impacts {
        sound_units: 0,
        light_units: 0,
        sediment_units: 0,
        thermal_units: 0,
        nutrient_units: 0,
        chemical_units: 0,
    };
    let mut net_value = 0_i64;
    let mut ghg_change = 0_i64;
    let mut water_saved = 0_u64;
    let mut local_value = 0_i64;
    let mut transfer_energy = 0_u64;
    let mut link_ids = Vec::with_capacity(selected.len());

    for index in selected {
        let link = &scenario.candidate_links[*index];
        if !link_issues(scenario, link).is_empty() {
            return None;
        }
        if !claim_ids.insert(link.benefit_claim_id.as_str()) {
            return None;
        }
        add_port_quantity(&mut source_used, &link.source_port_id, link.quantity_units);
        add_port_quantity(&mut sink_used, &link.consumer_port_id, link.quantity_units);
        impact_total = impact_total.add(link.impacts);
        link_ids.push(link.link_id.clone());
        net_value = net_value.saturating_add(link.net_value_milli);
        ghg_change = ghg_change.saturating_add(link.net_ghg_change_g_co2e);
        water_saved = water_saved.saturating_add(link.water_saved_litres);
        local_value = local_value.saturating_add(link.local_value_milli);
        transfer_energy = transfer_energy.saturating_add(link.transfer_energy_kwh_milli);
    }

    if !impact_total.within(scenario.impact_budget) {
        return None;
    }
    for (port_id, amount) in source_used {
        if port(scenario, &port_id)
            .is_none_or(|p| p.role != PortRole::Producer || amount > p.capacity_units)
        {
            return None;
        }
    }
    for (port_id, amount) in sink_used {
        if port(scenario, &port_id)
            .is_none_or(|p| p.role != PortRole::Consumer || amount > p.capacity_units)
        {
            return None;
        }
    }
    link_ids.sort();
    Some(PortfolioOutcome {
        candidate_link_ids: link_ids,
        net_value_milli: net_value,
        net_ghg_change_g_co2e: ghg_change,
        water_saved_litres: water_saved,
        local_value_milli: local_value,
        transfer_energy_kwh_milli: transfer_energy,
        impacts: impact_total,
    })
}

fn dominates(left: &PortfolioOutcome, right: &PortfolioOutcome) -> bool {
    let no_worse = left.net_value_milli >= right.net_value_milli
        && left.net_ghg_change_g_co2e <= right.net_ghg_change_g_co2e
        && left.water_saved_litres >= right.water_saved_litres
        && left.local_value_milli >= right.local_value_milli
        && left.transfer_energy_kwh_milli <= right.transfer_energy_kwh_milli
        && left.impacts.sound_units <= right.impacts.sound_units
        && left.impacts.light_units <= right.impacts.light_units
        && left.impacts.sediment_units <= right.impacts.sediment_units
        && left.impacts.thermal_units <= right.impacts.thermal_units
        && left.impacts.nutrient_units <= right.impacts.nutrient_units
        && left.impacts.chemical_units <= right.impacts.chemical_units;
    let strictly_better = left.net_value_milli > right.net_value_milli
        || left.net_ghg_change_g_co2e < right.net_ghg_change_g_co2e
        || left.water_saved_litres > right.water_saved_litres
        || left.local_value_milli > right.local_value_milli
        || left.transfer_energy_kwh_milli < right.transfer_energy_kwh_milli
        || left.impacts.sound_units < right.impacts.sound_units
        || left.impacts.light_units < right.impacts.light_units
        || left.impacts.sediment_units < right.impacts.sediment_units
        || left.impacts.thermal_units < right.impacts.thermal_units
        || left.impacts.nutrient_units < right.impacts.nutrient_units
        || left.impacts.chemical_units < right.impacts.chemical_units;
    no_worse && strictly_better
}

fn evaluate_scenario(scenario: &Scenario) -> ScenarioReport {
    let mut rejected_links = Vec::new();
    let mut individually_eligible_link_count = 0;
    for link in &scenario.candidate_links {
        let issues = link_issues(scenario, link);
        if issues.is_empty() {
            individually_eligible_link_count += 1;
        } else {
            rejected_links.push(RejectedLink {
                link_id: link.link_id.clone(),
                issues,
            });
        }
    }
    let count = scenario.candidate_links.len();
    let limit = 1_u64 << count;
    let mut feasible = Vec::new();
    for mask in 0..limit {
        let selected: Vec<usize> = (0..count)
            .filter(|index| mask & (1_u64 << index) != 0)
            .collect();
        if let Some(outcome) = portfolio_outcome(scenario, &selected) {
            feasible.push(outcome);
        }
    }
    let feasible_portfolio_count = feasible.len();
    let mut pareto = Vec::new();
    for (index, outcome) in feasible.iter().enumerate() {
        let dominated = feasible
            .iter()
            .enumerate()
            .any(|(other_index, other)| other_index != index && dominates(other, outcome));
        if !dominated {
            pareto.push(outcome.clone());
        }
    }
    pareto.sort_by(|a, b| a.candidate_link_ids.cmp(&b.candidate_link_ids));
    rejected_links.sort_by(|a, b| a.link_id.cmp(&b.link_id));
    ScenarioReport {
        scenario_id: scenario.scenario_id.clone(),
        individually_eligible_link_count,
        rejected_links,
        feasible_portfolio_count,
        pareto_frontier: pareto,
    }
}

fn add_invariant(invariants: &mut Vec<InvariantResult>, id: &str, passed: bool, detail: &str) {
    invariants.push(InvariantResult {
        invariant_id: id.to_owned(),
        passed,
        detail: detail.to_owned(),
    });
}

fn build_receipt(corpus: &Corpus, bytes: &[u8], source_commit: String) -> Receipt {
    let reports: Vec<ScenarioReport> = corpus.scenarios.iter().map(evaluate_scenario).collect();
    let mut all_frontiers_respect_gates = true;
    let mut duplicate_benefit_never_reused = true;
    let mut all_frontiers_respect_impact_budget = true;
    let mut quality_and_timing_are_enforced = true;
    let mut negative_controls_rejected = true;

    for scenario in &corpus.scenarios {
        for left in 0..scenario.candidate_links.len() {
            for right in (left + 1)..scenario.candidate_links.len() {
                if scenario.candidate_links[left].benefit_claim_id
                    == scenario.candidate_links[right].benefit_claim_id
                    && portfolio_outcome(scenario, &[left, right]).is_some()
                {
                    duplicate_benefit_never_reused = false;
                }
            }
        }

        let report = reports
            .iter()
            .find(|report| report.scenario_id == scenario.scenario_id)
            .expect("report exists");
        for outcome in &report.pareto_frontier {
            let selected_indices = outcome
                .candidate_link_ids
                .iter()
                .filter_map(|id| {
                    scenario
                        .candidate_links
                        .iter()
                        .position(|link| &link.link_id == id)
                })
                .collect::<Vec<_>>();
            if portfolio_outcome(scenario, &selected_indices).is_none() {
                all_frontiers_respect_gates = false;
            }
            if !outcome.impacts.within(scenario.impact_budget) {
                all_frontiers_respect_impact_budget = false;
            }
        }
    }

    for (scenario_id, link_id, expected_issue) in [
        (
            "quality-and-timing-false-synergies",
            "heat-too-cool",
            LinkIssueCode::QualityOutsideSinkRange,
        ),
        (
            "quality-and-timing-false-synergies",
            "heat-too-cool",
            LinkIssueCode::TimeWindowMismatch,
        ),
        (
            "quality-and-timing-false-synergies",
            "heat-treated-as-electricity",
            LinkIssueCode::StreamMismatch,
        ),
        (
            "brine-residual-hazard",
            "brine-recovery-unreviewed-fate",
            LinkIssueCode::ChemicalFateUnknown,
        ),
        (
            "brine-residual-hazard",
            "brine-recovery-unreviewed-fate",
            LinkIssueCode::EnvironmentalReviewIncomplete,
        ),
        (
            "brine-residual-hazard",
            "brine-recovery-unreviewed-fate",
            LinkIssueCode::MonitoringCoverageInsufficient,
        ),
        (
            "storm-resilience-and-no-build",
            "monitoring-under-covered",
            LinkIssueCode::MonitoringCoverageInsufficient,
        ),
        (
            "storm-resilience-and-no-build",
            "restricted-data-service",
            LinkIssueCode::UseNotPermitted,
        ),
        (
            "storm-resilience-and-no-build",
            "rights-review-required",
            LinkIssueCode::RightsReviewIncomplete,
        ),
    ] {
        let found_issue = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .and_then(|scenario| {
                scenario
                    .candidate_links
                    .iter()
                    .find(|link| link.link_id == link_id)
                    .map(|link| link_issues(scenario, link).contains(&expected_issue))
            })
            .unwrap_or(false);
        negative_controls_rejected &= found_issue;
        if matches!(
            expected_issue,
            LinkIssueCode::TimeWindowMismatch
                | LinkIssueCode::QualityOutsideSinkRange
                | LinkIssueCode::QualityMetricMismatch
        ) {
            quality_and_timing_are_enforced &= found_issue;
        }
    }

    for (scenario_id, link_id) in [
        (
            "brine-residual-hazard",
            "brine-treatment-known-but-chemical-over-cap",
        ),
        ("storm-resilience-and-no-build", "thermal-impact-over-cap"),
    ] {
        let rejected_by_budget = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == scenario_id)
            .and_then(|scenario| {
                scenario
                    .candidate_links
                    .iter()
                    .position(|link| link.link_id == link_id)
                    .map(|index| portfolio_outcome(scenario, &[index]).is_none())
            })
            .unwrap_or(false);
        all_frontiers_respect_impact_budget &= rejected_by_budget;
    }

    let mut invariants = Vec::new();
    add_invariant(&mut invariants, "portfolio_links_pass_evidence_and_physical_gates", all_frontiers_respect_gates,
        "Every Pareto portfolio consists only of links with compatible typed streams, timing/quality, rights, permissions, monitoring and environmental review.");
    add_invariant(
        &mut invariants,
        "no_duplicate_benefit_claims",
        duplicate_benefit_never_reused,
        "A benefit/counterfactual claim ID cannot be credited twice within one portfolio.",
    );
    add_invariant(&mut invariants, "aggregate_impact_budgets_respected", all_frontiers_respect_impact_budget,
        "Every reported frontier portfolio respects independent sound, light, sediment, thermal, nutrient and chemical-pressure caps.");
    add_invariant(&mut invariants, "quality_and_timing_faults_are_enforced", quality_and_timing_are_enforced,
        "Frozen heat-grade and timing negative controls are rejected with their declared reason codes.");
    add_invariant(&mut invariants, "frozen_rights_chemical_monitoring_and_stream_negative_controls_rejected", negative_controls_rejected,
        "Restricted/unreviewed use, unknown chemical fate, inadequate monitoring and wrong stream/quality/time matches are actively checked as negative controls.");
    add_invariant(
        &mut invariants,
        "exact_corpus_and_source_are_pinned",
        !sha256_hex(bytes).is_empty() && !source_commit.is_empty(),
        "Receipt binds the exact synthetic corpus bytes and source commit.",
    );
    add_invariant(&mut invariants, "empty_or_no_build_portfolio_remains_a_valid_baseline",
        reports.iter().all(|report| report.feasible_portfolio_count > 0),
        "The empty portfolio is always enumerated as a feasible no-build baseline, even when another portfolio dominates it on declared objectives.");

    let passed_invariant_count = invariants.iter().filter(|i| i.passed).count();
    let failed_invariant_count = invariants.len() - passed_invariant_count;
    Receipt {
        schema_id: SCHEMA_ID.into(),
        schema_version: SCHEMA_VERSION.into(),
        tool_version: TOOL_VERSION.into(),
        suite_id: SUITE_ID.into(),
        corpus_sha256: sha256_hex(bytes),
        source_commit,
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        seed: corpus.seed,
        scenario_count: reports.len(),
        passed_invariant_count,
        failed_invariant_count,
        result: if failed_invariant_count == 0 {
            "PASS"
        } else {
            "FAIL"
        }
        .into(),
        invariants,
        scenarios: reports,
    }
}

fn source_commit(explicit: Option<String>) -> Check<String> {
    let value = explicit
        .or_else(|| env::var("OCEAN_SYMBIOSIS_SOURCE_COMMIT").ok())
        .or_else(|| env::var("GITHUB_SHA").ok())
        .ok_or_else(|| "source commit required via --source-commit, OCEAN_SYMBIOSIS_SOURCE_COMMIT, or GITHUB_SHA".to_owned())?;
    if !(40..=64).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be a 40-64 character hexadecimal Git object ID".into());
    }
    Ok(value.to_ascii_lowercase())
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut corpus_path = PathBuf::from(DEFAULT_CORPUS);
    let mut receipt_path: Option<PathBuf> = None;
    let mut commit = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--corpus" => {
                index += 1;
                corpus_path = PathBuf::from(args.get(index).ok_or("--corpus requires a path")?);
            }
            "--receipt" => {
                index += 1;
                receipt_path = Some(PathBuf::from(
                    args.get(index).ok_or("--receipt requires a path")?,
                ));
            }
            "--source-commit" => {
                index += 1;
                commit = Some(
                    args.get(index)
                        .ok_or("--source-commit requires a Git ID")?
                        .clone(),
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }
    let bytes = fs::read(&corpus_path)
        .map_err(|error| format!("cannot read corpus {}: {error}", corpus_path.display()))?;
    let corpus = corpus_from_bytes(&bytes)?;
    let source_commit = source_commit(commit)?;
    let receipt = build_receipt(&corpus, &bytes, source_commit);
    let encoded = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("cannot encode receipt: {error}"))?;
    if let Some(path) = receipt_path {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create receipt directory: {error}"))?;
        }
        fs::write(path, encoded).map_err(|error| format!("cannot write receipt: {error}"))?;
    }
    println!(
        "suite={} corpus_sha256={} scenarios={} invariants={}/{} result={}",
        receipt.suite_id,
        receipt.corpus_sha256,
        receipt.scenario_count,
        receipt.passed_invariant_count,
        receipt.passed_invariant_count + receipt.failed_invariant_count,
        receipt.result
    );
    for item in &receipt.invariants {
        println!(
            "invariant={} result={} detail={}",
            item.invariant_id,
            if item.passed { "PASS" } else { "FAIL" },
            item.detail
        );
    }
    for scenario in &receipt.scenarios {
        println!(
            "scenario={} eligible_links={} feasible_portfolios={} pareto_count={}",
            scenario.scenario_id,
            scenario.individually_eligible_link_count,
            scenario.feasible_portfolio_count,
            scenario.pareto_frontier.len()
        );
        for outcome in &scenario.pareto_frontier {
            println!(
                "portfolio links={} net_value_milli={} net_ghg_change_g_co2e={} water_saved_litres={} local_value_milli={} transfer_energy_kwh_milli={}",
                outcome.candidate_link_ids.join("|"), outcome.net_value_milli, outcome.net_ghg_change_g_co2e,
                outcome.water_saved_litres, outcome.local_value_milli, outcome.transfer_energy_kwh_milli
            );
        }
        for link in &scenario.rejected_links {
            println!("rejected_link={} issues={:?}", link.link_id, link.issues);
        }
    }
    Ok(if receipt.result == "PASS" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("ocean-industrial-symbiosis-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Corpus, Vec<u8>) {
        let bytes =
            include_bytes!("../../../docs/tech/OCEAN_INDUSTRIAL_SYMBIOSIS_CORPUS_V0_1.json")
                .to_vec();
        let corpus =
            corpus_from_bytes(&bytes).expect("synthetic corpus parses and passes schema gate");
        (corpus, bytes)
    }

    #[test]
    fn frozen_corpus_is_synthetic_and_strict() {
        let (corpus, _) = fixture();
        assert_eq!(corpus.evidence_class, "SyntheticAdversarial");
        assert_eq!(corpus.scenarios.len(), SCENARIO_IDS.len());
    }

    #[test]
    fn heat_water_scenario_has_a_nonempty_frontier_and_rejects_no_physics() {
        let (corpus, _) = fixture();
        let report = evaluate_scenario(&corpus.scenarios[0]);
        assert!(report.individually_eligible_link_count >= 2);
        assert!(!report.pareto_frontier.is_empty());
    }

    #[test]
    fn shared_source_capacity_prevents_double_allocation_of_waste_heat() {
        let (corpus, _) = fixture();
        let scenario = &corpus.scenarios[0];
        let thermal = scenario
            .candidate_links
            .iter()
            .position(|link| link.link_id == "waste-heat-to-thermal-desalination")
            .unwrap();
        let aquaculture = scenario
            .candidate_links
            .iter()
            .position(|link| link.link_id == "waste-heat-to-aquaculture-cold-chain")
            .unwrap();
        assert!(link_issues(scenario, &scenario.candidate_links[thermal]).is_empty());
        assert!(link_issues(scenario, &scenario.candidate_links[aquaculture]).is_empty());
        assert!(portfolio_outcome(scenario, &[thermal, aquaculture]).is_none());
    }

    #[test]
    fn unsafe_brine_scenario_keeps_no_build_as_the_only_feasible_portfolio() {
        let (corpus, _) = fixture();
        let scenario = &corpus.scenarios[2];
        let report = evaluate_scenario(scenario);
        assert_eq!(report.feasible_portfolio_count, 1);
        assert_eq!(report.pareto_frontier.len(), 1);
        assert!(report.pareto_frontier[0].candidate_link_ids.is_empty());
    }

    #[test]
    fn temperature_grade_and_time_window_faults_are_rejected() {
        let (corpus, _) = fixture();
        let report = evaluate_scenario(&corpus.scenarios[1]);
        let heat = report
            .rejected_links
            .iter()
            .find(|item| item.link_id == "heat-too-cool")
            .unwrap();
        assert!(heat
            .issues
            .contains(&LinkIssueCode::QualityOutsideSinkRange));
        assert!(heat.issues.contains(&LinkIssueCode::TimeWindowMismatch));
    }

    #[test]
    fn toxic_or_unknown_brine_fate_is_not_a_viable_coproduct() {
        let (corpus, _) = fixture();
        let report = evaluate_scenario(&corpus.scenarios[2]);
        let rejected = report
            .rejected_links
            .iter()
            .find(|item| item.link_id == "brine-recovery-unreviewed-fate")
            .unwrap();
        assert!(rejected
            .issues
            .contains(&LinkIssueCode::ChemicalFateUnknown));
        assert!(rejected
            .issues
            .contains(&LinkIssueCode::EnvironmentalReviewIncomplete));
    }

    #[test]
    fn duplicate_counterfactual_claims_cannot_share_a_portfolio() {
        let (corpus, _) = fixture();
        let scenario = &corpus.scenarios[3];
        let a = scenario
            .candidate_links
            .iter()
            .position(|link| link.link_id == "power-to-electrolyser")
            .unwrap();
        let b = scenario
            .candidate_links
            .iter()
            .position(|link| link.link_id == "power-to-cold-store")
            .unwrap();
        assert!(portfolio_outcome(scenario, &[a]).is_some());
        assert!(portfolio_outcome(scenario, &[b]).is_some());
        assert!(portfolio_outcome(scenario, &[a, b]).is_none());
    }

    #[test]
    fn cumulative_impact_caps_are_separate_not_scalarized() {
        let (corpus, _) = fixture();
        let scenario = &corpus.scenarios[4];
        let valid = scenario
            .candidate_links
            .iter()
            .position(|link| link.link_id == "storm-resilient-sensor")
            .unwrap();
        let thermal = scenario
            .candidate_links
            .iter()
            .position(|link| link.link_id == "thermal-impact-over-cap")
            .unwrap();
        assert!(portfolio_outcome(scenario, &[valid]).is_some());
        assert!(portfolio_outcome(scenario, &[thermal]).is_none());
    }

    #[test]
    fn monitoring_rights_and_permission_are_independent_hard_gates() {
        let (corpus, _) = fixture();
        let scenario = &corpus.scenarios[4];
        assert!(link_issues(
            scenario,
            scenario
                .candidate_links
                .iter()
                .find(|link| link.link_id == "monitoring-under-covered")
                .unwrap()
        )
        .contains(&LinkIssueCode::MonitoringCoverageInsufficient));
        assert!(link_issues(
            scenario,
            scenario
                .candidate_links
                .iter()
                .find(|link| link.link_id == "rights-review-required")
                .unwrap()
        )
        .contains(&LinkIssueCode::RightsReviewIncomplete));
        assert!(link_issues(
            scenario,
            scenario
                .candidate_links
                .iter()
                .find(|link| link.link_id == "restricted-data-service")
                .unwrap()
        )
        .contains(&LinkIssueCode::UseNotPermitted));
    }

    #[test]
    fn incompatible_stream_units_never_pass_matching() {
        let (corpus, _) = fixture();
        let scenario = &corpus.scenarios[1];
        let wrong = scenario
            .candidate_links
            .iter()
            .find(|link| link.link_id == "heat-treated-as-electricity")
            .unwrap();
        assert!(link_issues(scenario, wrong).contains(&LinkIssueCode::StreamMismatch));
    }

    #[test]
    fn pareto_frontier_keeps_no_build_and_distinct_tradeoffs() {
        let (corpus, _) = fixture();
        for scenario in &corpus.scenarios {
            let report = evaluate_scenario(scenario);
            assert!(report.feasible_portfolio_count > 0);
            for outcome in &report.pareto_frontier {
                assert!(portfolio_outcome(
                    scenario,
                    &outcome
                        .candidate_link_ids
                        .iter()
                        .filter_map(|id| scenario
                            .candidate_links
                            .iter()
                            .position(|link| &link.link_id == id))
                        .collect::<Vec<_>>()
                )
                .is_some());
            }
        }
    }

    #[test]
    fn same_synthetic_inputs_produce_same_receipt() {
        let (corpus, bytes) = fixture();
        let a = serde_json::to_vec(&build_receipt(&corpus, &bytes, "a".repeat(40))).unwrap();
        let b = serde_json::to_vec(&build_receipt(&corpus, &bytes, "a".repeat(40))).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn unsupported_schema_and_unknown_fields_fail_closed() {
        let (_, bytes) = fixture();
        let text = String::from_utf8(bytes).unwrap();
        assert!(corpus_from_bytes(text.replace(SCHEMA_VERSION, "99.0.0").as_bytes()).is_err());
        let unknown = text.replacen("\"seed\":", "\"owner\":\"claimed\",\n  \"seed\":", 1);
        assert!(corpus_from_bytes(unknown.as_bytes()).is_err());
        assert!(corpus_from_bytes(&vec![b'x'; MAX_CORPUS_BYTES + 1]).is_err());
    }
}
