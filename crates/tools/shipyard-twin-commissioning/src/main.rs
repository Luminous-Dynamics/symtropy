use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const DEFAULT_CORPUS: &str = "benchmarks/shipyard/twin-commissioning/corpus-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/tech/SHIPYARD_TWIN_COMMISSIONING_V0_1.schema.json";
const BENCHMARK_ID: &str = "SHIPYARD-TWIN-COMMISSIONING-001";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readiness {
    Ready,
    ReadyLimited,
    NotReady,
}

impl Readiness {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::ReadyLimited => "READY_LIMITED",
            Self::NotReady => "NOT_READY",
        }
    }
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn u64_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn digest_valid(value: Option<&Value>) -> bool {
    value.and_then(Value::as_str).is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TwinLifecycleState {
    Unconfigured,
    Candidate,
    Commissioned,
    Degraded,
    Suspended,
    RequalificationRequired,
    Decommissioned,
}

impl TwinLifecycleState {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "UNCONFIGURED" => Some(Self::Unconfigured),
            "CANDIDATE" => Some(Self::Candidate),
            "COMMISSIONED" => Some(Self::Commissioned),
            "DEGRADED" => Some(Self::Degraded),
            "SUSPENDED" => Some(Self::Suspended),
            "REQUALIFICATION_REQUIRED" => Some(Self::RequalificationRequired),
            "DECOMMISSIONED" => Some(Self::Decommissioned),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Unconfigured => "UNCONFIGURED",
            Self::Candidate => "CANDIDATE",
            Self::Commissioned => "COMMISSIONED",
            Self::Degraded => "DEGRADED",
            Self::Suspended => "SUSPENDED",
            Self::RequalificationRequired => "REQUALIFICATION_REQUIRED",
            Self::Decommissioned => "DECOMMISSIONED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleEvent {
    ConfigurationComplete,
    CommissioningPassed,
    CommissioningFailed,
    OptionalBudgetExceeded,
    CriticalBudgetExceeded,
    DependencyRevisionChanged,
    RecommissioningStarted,
    SuspensionCleared,
    RequalificationStarted,
    Decommission,
}

impl LifecycleEvent {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "CONFIGURATION_COMPLETE" => Some(Self::ConfigurationComplete),
            "COMMISSIONING_PASSED" => Some(Self::CommissioningPassed),
            "COMMISSIONING_FAILED" => Some(Self::CommissioningFailed),
            "OPTIONAL_BUDGET_EXCEEDED" => Some(Self::OptionalBudgetExceeded),
            "CRITICAL_BUDGET_EXCEEDED" => Some(Self::CriticalBudgetExceeded),
            "DEPENDENCY_REVISION_CHANGED" => Some(Self::DependencyRevisionChanged),
            "RECOMMISSIONING_STARTED" => Some(Self::RecommissioningStarted),
            "SUSPENSION_CLEARED" => Some(Self::SuspensionCleared),
            "REQUALIFICATION_STARTED" => Some(Self::RequalificationStarted),
            "DECOMMISSION" => Some(Self::Decommission),
            _ => None,
        }
    }
}

/// Apply one lifecycle event. Illegal transitions return None and cannot be
/// interpreted as readiness. Recovery always requires fresh commissioning.
fn lifecycle_transition(
    state: TwinLifecycleState,
    event: LifecycleEvent,
) -> Option<TwinLifecycleState> {
    use LifecycleEvent as E;
    use TwinLifecycleState as S;

    if event == E::Decommission && state != S::Decommissioned {
        return Some(S::Decommissioned);
    }

    match (state, event) {
        (S::Unconfigured, E::ConfigurationComplete) => Some(S::Candidate),
        (S::Candidate, E::CommissioningPassed) => Some(S::Commissioned),
        (S::Candidate, E::CommissioningFailed) => Some(S::Candidate),
        (S::Commissioned, E::OptionalBudgetExceeded) => Some(S::Degraded),
        (S::Commissioned, E::CriticalBudgetExceeded) => Some(S::Suspended),
        (S::Commissioned, E::DependencyRevisionChanged) => Some(S::RequalificationRequired),
        (S::Degraded, E::CriticalBudgetExceeded) => Some(S::Suspended),
        (S::Degraded, E::DependencyRevisionChanged) => Some(S::RequalificationRequired),
        (S::Degraded, E::RecommissioningStarted) => Some(S::Candidate),
        (S::Suspended, E::SuspensionCleared) => Some(S::RequalificationRequired),
        (S::Suspended, E::DependencyRevisionChanged) => Some(S::RequalificationRequired),
        (S::RequalificationRequired, E::RequalificationStarted) => Some(S::Candidate),
        _ => None,
    }
}

fn validate_lifecycle_scenarios(corpus: &Value) -> Result<usize, Vec<String>> {
    let Some(scenarios) = corpus.get("lifecycle_scenarios").and_then(Value::as_array) else {
        return Err(vec!["lifecycle_scenarios must be an array".to_owned()]);
    };
    let mut errors = Vec::new();
    if scenarios.len() != 8 {
        errors.push(format!(
            "expected 8 lifecycle scenarios, found {}",
            scenarios.len()
        ));
    }
    let mut scenario_ids = HashSet::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        let Some(id) = str_at(scenario, "scenario_id") else {
            errors.push(format!("lifecycle_scenarios[{index}] has no scenario_id"));
            continue;
        };
        if !scenario_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate lifecycle scenario {id}"));
        }
        let Some(mut state) = str_at(scenario, "initial_state").and_then(TwinLifecycleState::parse)
        else {
            errors.push(format!("{id}: unknown initial lifecycle state"));
            continue;
        };
        let Some(events) = scenario.get("events").and_then(Value::as_array) else {
            errors.push(format!("{id}: events must be an array"));
            continue;
        };
        let Some(expected_states) = scenario.get("expected_states").and_then(Value::as_array)
        else {
            errors.push(format!("{id}: expected_states must be an array"));
            continue;
        };
        if expected_states.len() != events.len() + 1
            || expected_states.first().and_then(Value::as_str) != Some(state.as_str())
        {
            errors.push(format!(
                "{id}: expected_states must include the initial state and every transition"
            ));
            continue;
        }
        for (step, event_value) in events.iter().enumerate() {
            let Some(event_name) = event_value.as_str() else {
                errors.push(format!("{id}: event[{step}] must be a string"));
                break;
            };
            let Some(event) = LifecycleEvent::parse(event_name) else {
                errors.push(format!("{id}: unknown lifecycle event {event_name}"));
                break;
            };
            let Some(next_state) = lifecycle_transition(state, event) else {
                errors.push(format!(
                    "{id}: illegal transition {} + {event_name}",
                    state.as_str()
                ));
                break;
            };
            let expected = expected_states
                .get(step + 1)
                .and_then(Value::as_str)
                .unwrap_or("");
            if next_state.as_str() != expected {
                errors.push(format!(
                    "{id}: after {event_name}, expected {expected}, computed {}",
                    next_state.as_str()
                ));
                break;
            }
            state = next_state;
        }
    }
    if errors.is_empty() {
        Ok(scenarios.len())
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaseResult {
    readiness: Readiness,
    failures: Vec<String>,
    required_failures: usize,
    optional_failures: usize,
}

fn evaluate_case(case: &Value) -> Result<CaseResult, String> {
    let case_id = str_at(case, "case_id").ok_or("case_id missing")?;
    let composition = str_at(case, "composition_mode").ok_or("composition_mode missing")?;
    let components = case
        .get("components")
        .and_then(Value::as_array)
        .ok_or("components must be an array")?;
    let interfaces = case
        .get("interfaces")
        .and_then(Value::as_array)
        .ok_or("interfaces must be an array")?;

    if components.len() < 2 || interfaces.is_empty() {
        return Err(format!(
            "{case_id}: at least two components and one interface are required"
        ));
    }

    let mut component_ids = HashSet::new();
    let mut schemas = HashSet::new();
    for (index, component) in components.iter().enumerate() {
        let id = str_at(component, "component_id")
            .ok_or_else(|| format!("{case_id}: component[{index}] has no component_id"))?;
        if !component_ids.insert(id.to_owned()) {
            return Err(format!("{case_id}: duplicate component_id {id}"));
        }
        if !nonempty(str_at(component, "owner_system"))
            || !nonempty(str_at(component, "source_system"))
            || !nonempty(str_at(component, "source_revision"))
            || !nonempty(str_at(component, "schema_id"))
            || !nonempty(str_at(component, "schema_revision"))
            || !nonempty(str_at(component, "digital_thread_uri"))
            || !nonempty(str_at(component, "digital_thread_revision"))
            || !digest_valid(component.get("source_identity_sha256"))
        {
            return Err(format!(
                "{case_id}: component {id} has incomplete source/thread identity"
            ));
        }
        if let Some(schema) = str_at(component, "schema_id") {
            schemas.insert(schema.to_owned());
        }
    }

    let mut failures = HashSet::new();
    let mut interface_ids = HashSet::new();
    let mut event_ids = HashSet::new();
    let mut mapping_modes = HashSet::new();
    let mut required_failures = 0usize;
    let mut optional_failures = 0usize;

    for (index, interface) in interfaces.iter().enumerate() {
        let id = str_at(interface, "interface_id")
            .ok_or_else(|| format!("{case_id}: interface[{index}] has no interface_id"))?;
        if !interface_ids.insert(id.to_owned()) {
            return Err(format!("{case_id}: duplicate interface_id {id}"));
        }
        let from = str_at(interface, "from_component")
            .ok_or_else(|| format!("{case_id}: {id} has no from_component"))?;
        let to = str_at(interface, "to_component")
            .ok_or_else(|| format!("{case_id}: {id} has no to_component"))?;
        if !component_ids.contains(from) || !component_ids.contains(to) || from == to {
            return Err(format!(
                "{case_id}: {id} references missing or identical components"
            ));
        }
        let required = bool_at(interface, "required")
            .ok_or_else(|| format!("{case_id}: {id}.required must be boolean"))?;
        let mapping_mode = str_at(interface, "mapping_mode")
            .ok_or_else(|| format!("{case_id}: {id}.mapping_mode missing"))?;
        if !["integrated", "unified", "federated"].contains(&mapping_mode) {
            return Err(format!("{case_id}: {id} has unsupported mapping mode"));
        }
        mapping_modes.insert(mapping_mode.to_owned());
        match mapping_mode {
            "integrated" if !nonempty(str_at(interface, "composition_schema_id")) => {
                failures.insert("COMPOSITION_MODE".to_owned());
            }
            "unified" | "federated"
                if !interface
                    .get("composition_schema_id")
                    .is_some_and(Value::is_null) =>
            {
                failures.insert("COMPOSITION_MODE".to_owned());
            }
            _ => {}
        }

        let event_id = str_at(interface, "source_event_id")
            .ok_or_else(|| format!("{case_id}: {id}.source_event_id missing"))?;
        if !event_ids.insert(event_id.to_owned()) {
            return Err(format!("{case_id}: duplicate event identity {event_id}"));
        }

        let mut local_failures = Vec::new();
        if !digest_valid(interface.get("source_sha256"))
            || !digest_valid(interface.get("mapping_sha256"))
            || !nonempty(str_at(interface, "source_revision"))
            || !["source_verified", "peer_verified"]
                .contains(&str_at(interface, "authority_mode").unwrap_or(""))
        {
            local_failures.push("PROVENANCE");
        }

        if u64_at(interface, "update_interval_ms")
            .is_none_or(|value| value > u64_at(interface, "max_update_interval_ms").unwrap_or(0))
        {
            local_failures.push("UPDATE_FREQUENCY");
        }
        if u64_at(interface, "sample_interval_us")
            .is_none_or(|value| value > u64_at(interface, "max_sample_interval_us").unwrap_or(0))
        {
            local_failures.push("SAMPLE_FREQUENCY");
        }

        if u64_at(interface, "jitter_p95_ms")
            .is_none_or(|value| value > u64_at(interface, "max_jitter_p95_ms").unwrap_or(0))
        {
            local_failures.push("UPDATE_JITTER");
        }
        if u64_at(interface, "latency_p95_ms")
            .is_none_or(|value| value > u64_at(interface, "max_latency_p95_ms").unwrap_or(0))
        {
            local_failures.push("LATENCY_P95");
        }
        if u64_at(interface, "data_age_ms")
            .is_none_or(|value| value > u64_at(interface, "max_data_age_ms").unwrap_or(0))
        {
            local_failures.push("SOURCE_STALE");
        }
        if u64_at(interface, "semantic_fidelity_permille").is_none_or(|value| {
            value < u64_at(interface, "minimum_semantic_fidelity_permille").unwrap_or(1001)
        }) {
            local_failures.push("SEMANTIC_FIDELITY");
        }
        if !digest_valid(interface.get("clock_receipt_sha256"))
            || u64_at(interface, "clock_offset_abs_us").is_none_or(|value| {
                value > u64_at(interface, "maximum_clock_offset_us").unwrap_or(0)
            })
        {
            local_failures.push("CLOCK_SYNC");
        }
        if u64_at(interface, "uncertainty_permille").is_none_or(|value| {
            value > u64_at(interface, "maximum_uncertainty_permille").unwrap_or(0)
        }) {
            local_failures.push("UNCERTAINTY_LIMIT");
        }
        if mapping_mode == "federated"
            && (str_at(interface, "authority_mode") != Some("peer_verified")
                || !digest_valid(interface.get("peer_identity_sha256")))
        {
            local_failures.push("PROVENANCE");
        }
        if mapping_mode != "federated"
            && str_at(interface, "authority_mode") != Some("source_verified")
        {
            local_failures.push("PROVENANCE");
        }

        if !local_failures.is_empty() {
            if required {
                required_failures += local_failures.len();
            } else {
                optional_failures += local_failures.len();
            }
            for failure in local_failures {
                failures.insert(failure.to_owned());
            }
        }
    }

    match composition {
        "integrated" => {
            if schemas.len() != 1 || mapping_modes.iter().any(|mode| mode != "integrated") {
                failures.insert("COMPOSITION_MODE".to_owned());
                required_failures += 1;
            }
        }
        "unified" => {
            if schemas.len() < 2 || mapping_modes.iter().any(|mode| mode != "unified") {
                failures.insert("COMPOSITION_MODE".to_owned());
                required_failures += 1;
            }
        }
        "federated" => {
            if mapping_modes.iter().any(|mode| mode != "federated")
                || components
                    .iter()
                    .any(|component| str_at(component, "authority_mode") != Some("peer_verified"))
            {
                failures.insert("COMPOSITION_MODE".to_owned());
                required_failures += 1;
            }
        }
        "mixed" => {
            if mapping_modes.len() < 2 {
                failures.insert("COMPOSITION_MODE".to_owned());
                required_failures += 1;
            }
        }
        _ => return Err(format!("{case_id}: unknown composition mode {composition}")),
    }

    let readiness = if required_failures > 0 {
        Readiness::NotReady
    } else if optional_failures > 0 {
        Readiness::ReadyLimited
    } else {
        Readiness::Ready
    };
    let mut failures: Vec<String> = failures.into_iter().collect();
    failures.sort();

    Ok(CaseResult {
        readiness,
        failures,
        required_failures,
        optional_failures,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Summary {
    cases: usize,
    ready: usize,
    ready_limited: usize,
    not_ready: usize,
    exact_oracle_agreements: usize,
    lifecycle_scenarios: usize,
}

fn validate_corpus(schema_text: &str, corpus_text: &str) -> Result<Summary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("schema JSON parse failed: {error}")]),
    };
    let corpus: Value = match serde_json::from_str(corpus_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("corpus JSON parse failed: {error}")]),
    };
    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("schema must declare Draft 2020-12".to_owned());
    }
    if str_at(&corpus, "schema_version") != Some("0.1.0")
        || str_at(&corpus, "benchmark_id") != Some(BENCHMARK_ID)
        || str_at(&corpus, "claim_ceiling")
            != Some("SYNTHETIC_COMPOSITION_COMMISSIONING_CONTRACT_ONLY")
    {
        errors.push("corpus identity or claim ceiling mismatch".to_owned());
    }
    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["cases must be an array".to_owned()]);
    };
    if cases.len() != 13 {
        errors.push(format!(
            "expected 13 commissioning cases, found {}",
            cases.len()
        ));
    }

    let mut case_ids = HashSet::new();
    let mut ready = 0usize;
    let mut ready_limited = 0usize;
    let mut not_ready = 0usize;
    let mut agreements = 0usize;
    let allowed_failures = [
        "UPDATE_FREQUENCY",
        "SAMPLE_FREQUENCY",
        "UPDATE_JITTER",
        "LATENCY_P95",
        "SOURCE_STALE",
        "SEMANTIC_FIDELITY",
        "PROVENANCE",
        "CLOCK_SYNC",
        "UNCERTAINTY_LIMIT",
        "COMPOSITION_MODE",
    ];

    for (index, case) in cases.iter().enumerate() {
        let case_id = match str_at(case, "case_id") {
            Some(value) => value.to_owned(),
            None => {
                errors.push(format!("cases[{index}] has no case_id"));
                continue;
            }
        };
        if !case_ids.insert(case_id.clone()) {
            errors.push(format!("duplicate case_id {case_id}"));
        }
        let result = match evaluate_case(case) {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        match result.readiness {
            Readiness::Ready => ready += 1,
            Readiness::ReadyLimited => ready_limited += 1,
            Readiness::NotReady => not_ready += 1,
        }
        let expected_readiness = str_at(case, "expected_readiness").unwrap_or("");
        let mut expected_failures = case
            .get("expected_failures")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        expected_failures.sort();
        if expected_failures
            .iter()
            .any(|value| !allowed_failures.contains(&value.as_str()))
        {
            errors.push(format!("{case_id}: unknown expected failure code"));
        }
        if result.readiness.as_str() == expected_readiness && result.failures == expected_failures {
            agreements += 1;
        } else {
            errors.push(format!(
                "{case_id}: expected {} {:?}, computed {} {:?}",
                expected_readiness,
                expected_failures,
                result.readiness.as_str(),
                result.failures
            ));
        }
    }

    let lifecycle_scenarios = match validate_lifecycle_scenarios(&corpus) {
        Ok(count) => count,
        Err(lifecycle_errors) => {
            errors.extend(lifecycle_errors);
            0
        }
    };

    if errors.is_empty() {
        Ok(Summary {
            cases: cases.len(),
            ready,
            ready_limited,
            not_ready,
            exact_oracle_agreements: agreements,
            lifecycle_scenarios,
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    let schema_path = args.next().unwrap_or_else(|| DEFAULT_SCHEMA.to_owned());
    if args.next().is_some() {
        eprintln!("usage: shipyard-twin-commissioning [CORPUS_PATH] [SCHEMA_PATH]");
        return ExitCode::from(2);
    }
    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read schema {schema_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus_text = match fs::read_to_string(&corpus_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read corpus {corpus_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match validate_corpus(&schema_text, &corpus_text) {
        Ok(summary) => {
            println!(
                concat!(
                    "PASS: {cases} synthetic cases; {lifecycle} lifecycle scenarios; ",
                    "{ready} READY / {limited} READY_LIMITED / ",
                    "{blocked} NOT_READY; {agreements} exact oracle agreements; ",
                    "no ISO conformance claimed"
                ),
                cases = summary.cases,
                lifecycle = summary.lifecycle_scenarios,
                ready = summary.ready,
                limited = summary.ready_limited,
                blocked = summary.not_ready,
                agreements = summary.exact_oracle_agreements
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!(
                "FAIL: {} commissioning contract discrepancy/ies",
                errors.len()
            );
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_TEXT: &str =
        include_str!("../../../../docs/tech/SHIPYARD_TWIN_COMMISSIONING_V0_1.schema.json");
    const CORPUS_TEXT: &str =
        include_str!("../../../../benchmarks/shipyard/twin-commissioning/corpus-v0.1.json");

    fn corpus_value() -> Value {
        serde_json::from_str(CORPUS_TEXT).expect("corpus parses")
    }

    fn case_named<'a>(corpus: &'a mut Value, id: &str) -> &'a mut Value {
        corpus["cases"]
            .as_array_mut()
            .expect("case array")
            .iter_mut()
            .find(|case| case["case_id"] == id)
            .expect("case exists")
    }

    #[test]
    fn frozen_lifecycle_corpus_matches_all_expected_transitions() {
        let corpus = corpus_value();
        assert_eq!(
            validate_lifecycle_scenarios(&corpus).expect("lifecycle corpus should be legal"),
            8
        );
    }

    #[test]
    fn candidate_cannot_be_commissioned_without_a_passed_commissioning_event() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Unconfigured,
                LifecycleEvent::CommissioningPassed
            ),
            None
        );
    }

    #[test]
    fn optional_budget_breach_degrades_but_does_not_suspend() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Commissioned,
                LifecycleEvent::OptionalBudgetExceeded
            ),
            Some(TwinLifecycleState::Degraded)
        );
    }

    #[test]
    fn critical_budget_breach_suspends_a_commissioned_twin() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Commissioned,
                LifecycleEvent::CriticalBudgetExceeded
            ),
            Some(TwinLifecycleState::Suspended)
        );
    }

    #[test]
    fn degraded_twin_requires_a_fresh_commissioning_cycle() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Degraded,
                LifecycleEvent::RecommissioningStarted
            ),
            Some(TwinLifecycleState::Candidate)
        );
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Degraded,
                LifecycleEvent::CommissioningPassed
            ),
            None
        );
    }

    #[test]
    fn suspended_twin_cannot_recommission_directly() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Suspended,
                LifecycleEvent::RequalificationStarted
            ),
            None
        );
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Suspended,
                LifecycleEvent::SuspensionCleared
            ),
            Some(TwinLifecycleState::RequalificationRequired)
        );
    }

    #[test]
    fn dependency_revision_change_invalidates_commissioning() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Commissioned,
                LifecycleEvent::DependencyRevisionChanged
            ),
            Some(TwinLifecycleState::RequalificationRequired)
        );
    }

    #[test]
    fn requalification_must_return_through_candidate_state() {
        let required = lifecycle_transition(
            TwinLifecycleState::Suspended,
            LifecycleEvent::SuspensionCleared,
        )
        .expect("suspension can be cleared into requalification");
        let candidate = lifecycle_transition(required, LifecycleEvent::RequalificationStarted)
            .expect("requalification can begin");
        let commissioned = lifecycle_transition(candidate, LifecycleEvent::CommissioningPassed)
            .expect("fresh commissioning can pass");
        assert_eq!(commissioned, TwinLifecycleState::Commissioned);
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::RequalificationRequired,
                LifecycleEvent::CommissioningPassed
            ),
            None
        );
    }

    #[test]
    fn decommissioned_twin_is_terminal() {
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Commissioned,
                LifecycleEvent::Decommission
            ),
            Some(TwinLifecycleState::Decommissioned)
        );
        assert_eq!(
            lifecycle_transition(
                TwinLifecycleState::Decommissioned,
                LifecycleEvent::ConfigurationComplete
            ),
            None
        );
    }

    #[test]
    fn frozen_thirteen_case_corpus_matches_all_expected_dispositions() {
        let summary = validate_corpus(SCHEMA_TEXT, CORPUS_TEXT)
            .expect("frozen commissioning corpus should match its oracle");
        assert_eq!(
            summary,
            Summary {
                cases: 13,
                ready: 4,
                ready_limited: 0,
                not_ready: 9,
                exact_oracle_agreements: 13,
            }
        );
    }

    #[test]
    fn all_three_iso_composition_modes_are_exercised() {
        let corpus = corpus_value();
        let cases = corpus["cases"].as_array().expect("cases");
        for mode in ["integrated", "unified", "federated", "mixed"] {
            assert!(cases.iter().any(|case| case["composition_mode"] == mode));
        }
    }

    #[test]
    fn low_update_frequency_blocks_readiness() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["update_interval_ms"] = Value::from(1_000);
        assert_eq!(
            evaluate_case(case).expect("case evaluates").readiness,
            Readiness::NotReady
        );
    }

    #[test]
    fn high_sample_interval_is_not_hidden_by_good_update_frequency() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["sample_interval_us"] = Value::from(50_000);
        case["interfaces"][0]["max_sample_interval_us"] = Value::from(10_000);
        case["interfaces"][0]["update_interval_ms"] = Value::from(10);
        case["interfaces"][0]["max_update_interval_ms"] = Value::from(250);
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"SAMPLE_FREQUENCY".to_owned()));
        assert!(!result.failures.contains(&"UPDATE_FREQUENCY".to_owned()));
    }

    #[test]
    fn bam_like_interpass_streams_have_distinct_sampling_and_update_contracts() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        let interface = &mut case["interfaces"][0];
        interface["sample_interval_us"] = Value::from(100); // 0.1 ms current/voltage
        interface["max_sample_interval_us"] = Value::from(200);
        interface["update_interval_ms"] = Value::from(100);
        interface["max_update_interval_ms"] = Value::from(250);
        assert_eq!(
            evaluate_case(case).expect("case evaluates").readiness,
            Readiness::Ready
        );
    }

    #[test]
    fn high_jitter_is_not_hidden_by_good_mean_update_frequency() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["jitter_p95_ms"] = Value::from(80);
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"UPDATE_JITTER".to_owned()));
    }

    #[test]
    fn delivery_latency_does_not_substitute_for_source_freshness() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["latency_p95_ms"] = Value::from(10);
        case["interfaces"][0]["data_age_ms"] = Value::from(10_000);
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"SOURCE_STALE".to_owned()));
        assert!(!result.failures.contains(&"LATENCY_P95".to_owned()));
    }

    #[test]
    fn good_schema_names_do_not_hide_bad_semantic_fidelity() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-002");
        case["interfaces"][0]["semantic_fidelity_permille"] = Value::from(400);
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"SEMANTIC_FIDELITY".to_owned()));
    }

    #[test]
    fn missing_source_digest_blocks_provenance() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["source_sha256"] = Value::Null;
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"PROVENANCE".to_owned()));
    }

    #[test]
    fn clock_receipt_and_offset_are_independent_commissioning_gates() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["clock_offset_abs_us"] = Value::from(500);
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"CLOCK_SYNC".to_owned()));
    }

    #[test]
    fn model_uncertainty_exceeding_purpose_limit_blocks_readiness() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["uncertainty_permille"] = Value::from(500);
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"UNCERTAINTY_LIMIT".to_owned()));
    }

    #[test]
    fn optional_component_failure_limits_only_this_purpose() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][0]["required"] = Value::Bool(false);
        case["interfaces"][0]["data_age_ms"] = Value::from(10_000);
        let result = evaluate_case(case).expect("case evaluates");
        assert_eq!(result.readiness, Readiness::ReadyLimited);
        assert_eq!(result.required_failures, 0);
        assert_eq!(result.optional_failures, 1);
    }

    #[test]
    fn integrated_mode_requires_a_common_schema() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["components"][1]["schema_id"] = Value::String("foreign.schema.v1".to_owned());
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"COMPOSITION_MODE".to_owned()));
    }

    #[test]
    fn unified_mode_retains_distinct_component_schemas() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-002");
        for component in case["components"].as_array_mut().expect("components") {
            component["schema_id"] = Value::String("one.schema.v1".to_owned());
        }
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"COMPOSITION_MODE".to_owned()));
    }

    #[test]
    fn federated_mode_requires_peer_identity_proofs() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-003");
        case["interfaces"][0]["peer_identity_sha256"] = Value::Null;
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"PROVENANCE".to_owned()));
    }

    #[test]
    fn mixed_composition_requires_explicit_canonical_schema_per_integrated_edge() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-004");
        case["interfaces"][0]["composition_schema_id"] = Value::Null;
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"COMPOSITION_MODE".to_owned()));
    }

    #[test]
    fn unified_composition_does_not_silently_add_a_global_schema() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-002");
        case["interfaces"][0]["composition_schema_id"] =
            Value::String("shipyard.manufacturing.common.v1".to_owned());
        let result = evaluate_case(case).expect("case evaluates");
        assert!(result.failures.contains(&"COMPOSITION_MODE".to_owned()));
    }

    #[test]
    fn duplicate_event_identity_is_rejected() {
        let mut corpus = corpus_value();
        let case = case_named(&mut corpus, "DTC-001");
        case["interfaces"][1]["source_event_id"] = case["interfaces"][0]["source_event_id"].clone();
        assert!(evaluate_case(case).is_err());
    }

    #[test]
    fn composition_failure_blocks_required_purpose_not_global_platform() {
        let mut corpus = corpus_value();
        let first = case_named(&mut corpus, "DTC-001");
        first["interfaces"][0]["required"] = Value::Bool(false);
        first["interfaces"][0]["source_sha256"] = Value::Null;
        let result = evaluate_case(first).expect("case evaluates");
        assert_eq!(result.readiness, Readiness::ReadyLimited);
    }
}
