// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Strict, dependency-free ingestion of the CivBench decision-closure fixture.
//!
//! This adapter parses the JSON fixture, converts each arm into the existing
//! DecisionClosureRecord type and invokes that type's validator. It does not
//! authenticate source signatures, calculate hydraulics or infer causal effects.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::decision_closure::{
    ActionProgress, DecisionClosureRecord, HumanDisposition, ReceiptBinding, ReceiptOwner,
    ResourceFeasibility, ResourceSettlement, ServiceEffect, ServiceEvidenceClass,
};

const MAX_FIXTURE_BYTES: usize = 2 * 1024 * 1024;
const MAX_JSON_DEPTH: usize = 96;
const EXPECTED_FIXTURE_ID: &str = "CIVBENCH-DECISION-CLOSURE-FIXTURE-001";

/// Location-aware parsing or semantic validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureError {
    /// Stable machine-readable category.
    pub code: &'static str,
    /// JSON path associated with the error.
    pub path: String,
    /// Byte offset; zero for semantic errors.
    pub offset: usize,
    /// Human-readable detail.
    pub detail: String,
}

impl FixtureError {
    fn new(code: &'static str, path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { code, path: path.into(), offset: 0, detail: detail.into() }
    }

    fn at(code: &'static str, offset: usize, detail: impl Into<String>) -> Self {
        Self { code, path: "$".into(), offset, detail: detail.into() }
    }
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {} (byte {}): {}", self.code, self.path, self.offset, self.detail)
    }
}

impl std::error::Error for FixtureError {}

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

struct JsonParser<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input: input.as_bytes(), cursor: 0 }
    }

    fn parse(mut self) -> Result<JsonValue, FixtureError> {
        self.skip_whitespace();
        let value = self.parse_value(0)?;
        self.skip_whitespace();
        if self.cursor != self.input.len() {
            return Err(FixtureError::at(
                "trailing_json_data",
                self.cursor,
                "non-whitespace data follows the JSON value",
            ));
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, FixtureError> {
        if depth > MAX_JSON_DEPTH {
            return Err(FixtureError::at("json_nesting_limit", self.cursor, "JSON nesting limit exceeded"));
        }
        self.skip_whitespace();
        match self.peek() {
            Some(b'n') => {
                self.expect_literal(b"null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.expect_literal(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(_) => Err(FixtureError::at("invalid_json_value", self.cursor, "unexpected JSON token")),
            None => Err(FixtureError::at("unexpected_end_of_json", self.cursor, "expected a JSON value")),
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, FixtureError> {
        self.consume(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.cursor += 1;
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.cursor += 1;
                    self.skip_whitespace();
                }
                Some(b']') => {
                    self.cursor += 1;
                    break;
                }
                _ => return Err(FixtureError::at("invalid_json_array", self.cursor, "expected ',' or ']'")),
            }
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, FixtureError> {
        self.consume(b'{')?;
        self.skip_whitespace();
        let mut values = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.cursor += 1;
            return Ok(JsonValue::Object(values));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(FixtureError::at("invalid_json_object", self.cursor, "object keys must be strings"));
            }
            let key_offset = self.cursor;
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.consume(b':')?;
            let value = self.parse_value(depth)?;
            if values.insert(key.clone(), value).is_some() {
                return Err(FixtureError::at(
                    "duplicate_json_key",
                    key_offset,
                    format!("duplicate object key '{key}'"),
                ));
            }
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.cursor += 1;
                    self.skip_whitespace();
                }
                Some(b'}') => {
                    self.cursor += 1;
                    break;
                }
                _ => return Err(FixtureError::at("invalid_json_object", self.cursor, "expected ',' or '}'")),
            }
        }
        Ok(JsonValue::Object(values))
    }

    fn parse_string(&mut self) -> Result<String, FixtureError> {
        self.consume(b'"')?;
        let mut output = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(FixtureError::at("unterminated_json_string", self.cursor, "missing closing quote"));
            };
            match byte {
                b'"' => {
                    self.cursor += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.cursor += 1;
                    let escaped = self.peek().ok_or_else(|| {
                        FixtureError::at("unterminated_json_escape", self.cursor, "incomplete escape")
                    })?;
                    self.cursor += 1;
                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => output.push(self.parse_unicode_escape()?),
                        _ => return Err(FixtureError::at("invalid_json_escape", self.cursor - 1, "unsupported escape")),
                    }
                }
                0x00..=0x1f => {
                    return Err(FixtureError::at("control_in_json_string", self.cursor, "unescaped control byte"));
                }
                0x20..=0x7f => {
                    output.push(char::from(byte));
                    self.cursor += 1;
                }
                _ => {
                    let remaining = std::str::from_utf8(&self.input[self.cursor..]).map_err(|_| {
                        FixtureError::at("invalid_utf8", self.cursor, "invalid UTF-8 in string")
                    })?;
                    let character = remaining.chars().next().ok_or_else(|| {
                        FixtureError::at("invalid_utf8", self.cursor, "missing UTF-8 character")
                    })?;
                    self.cursor += character.len_utf8();
                    output.push(character);
                }
            }
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, FixtureError> {
        let first = self.parse_hex_quad()?;
        let scalar = if (0xd800..=0xdbff).contains(&first) {
            if self.peek() != Some(b'\\') || self.input.get(self.cursor + 1) != Some(&b'u') {
                return Err(FixtureError::at("invalid_unicode_surrogate", self.cursor, "high surrogate requires low surrogate"));
            }
            self.cursor += 2;
            let second = self.parse_hex_quad()?;
            if !(0xdc00..=0xdfff).contains(&second) {
                return Err(FixtureError::at("invalid_unicode_surrogate", self.cursor - 4, "expected low surrogate"));
            }
            0x10000 + (((first - 0xd800) as u32) << 10) + (second - 0xdc00) as u32
        } else if (0xdc00..=0xdfff).contains(&first) {
            return Err(FixtureError::at("invalid_unicode_surrogate", self.cursor - 4, "unpaired low surrogate"));
        } else {
            first as u32
        };
        char::from_u32(scalar).ok_or_else(|| {
            FixtureError::at("invalid_unicode_scalar", self.cursor - 4, "invalid Unicode scalar")
        })
    }

    fn parse_hex_quad(&mut self) -> Result<u16, FixtureError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let byte = self.peek().ok_or_else(|| {
                FixtureError::at("incomplete_unicode_escape", self.cursor, "expected four hexadecimal digits")
            })?;
            let digit = (byte as char).to_digit(16).ok_or_else(|| {
                FixtureError::at("invalid_unicode_escape", self.cursor, "non-hexadecimal Unicode digit")
            })?;
            value = (value << 4) | digit as u16;
            self.cursor += 1;
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, FixtureError> {
        let start = self.cursor;
        if self.peek() == Some(b'-') {
            self.cursor += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.cursor += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(FixtureError::at("invalid_json_number", self.cursor, "leading zero"));
                }
            }
            Some(b'1'..=b'9') => {
                self.cursor += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.cursor += 1;
                }
            }
            _ => return Err(FixtureError::at("invalid_json_number", self.cursor, "expected digits")),
        }
        if self.peek() == Some(b'.') {
            self.cursor += 1;
            let fraction_start = self.cursor;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.cursor += 1;
            }
            if self.cursor == fraction_start {
                return Err(FixtureError::at("invalid_json_number", self.cursor, "fraction requires digits"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.cursor += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.cursor += 1;
            }
            let exponent_start = self.cursor;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.cursor += 1;
            }
            if self.cursor == exponent_start {
                return Err(FixtureError::at("invalid_json_number", self.cursor, "exponent requires digits"));
            }
        }
        let number = std::str::from_utf8(&self.input[start..self.cursor]).map_err(|_| {
            FixtureError::at("invalid_json_number", start, "number bytes are not UTF-8")
        })?;
        Ok(number.to_owned())
    }

    fn expect_literal(&mut self, literal: &[u8]) -> Result<(), FixtureError> {
        if self.input.get(self.cursor..self.cursor + literal.len()) == Some(literal) {
            self.cursor += literal.len();
            Ok(())
        } else {
            Err(FixtureError::at("invalid_json_literal", self.cursor, "unexpected literal"))
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), FixtureError> {
        if self.peek() == Some(expected) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(FixtureError::at(
                "unexpected_json_token",
                self.cursor,
                format!("expected '{}'", char::from(expected)),
            ))
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.cursor += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.cursor).copied()
    }
}

/// Evidence status carried by an outcome metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricEvidenceStatus {
    /// Direct qualified observation.
    Observed,
    /// Reproducible calculation from qualified inputs.
    Derived,
    /// Simulation or model output.
    Modelled,
    /// No measurement exists.
    NotMeasured,
    /// Evaluator cannot qualify the metric.
    Unsupported,
    /// Data exists but is governed or restricted.
    Withheld,
    /// Metric does not apply.
    NotApplicable,
}

/// One unit-explicit metric from an outcome arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeMetric {
    /// Stable metric key.
    pub name: String,
    /// Numeric value, absent when not qualified.
    pub value: Option<i64>,
    /// Evidence status.
    pub status: MetricEvidenceStatus,
}

/// Acquisition result retained alongside the owner receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquisitionResult {
    /// Whether a valid observation was returned.
    pub valid_observation_returned: bool,
    /// Whether the observation changed the decision.
    pub decision_changed: bool,
    /// Status of the decision-value estimate.
    pub decision_value_status: AcquisitionValueStatus,
    /// Decision value in minor currency units.
    pub decision_value_minor: Option<i64>,
    /// Acquisition cost in minor currency units.
    pub acquisition_cost_minor: u64,
}

/// Evidence status for an acquisition decision-value estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionValueStatus {
    /// Modelled estimate.
    Modelled,
    /// Observed estimate.
    Observed,
    /// Not measured.
    NotMeasured,
    /// Unsupported.
    Unsupported,
}

/// One control/candidate arm after typed conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureArm {
    /// Owner policy reference.
    pub policy_ref: String,
    /// Validated cross-owner closure record.
    pub record: DecisionClosureRecord,
    /// Source outcome vector; no cross-status arithmetic is performed.
    pub outcomes: Vec<OutcomeMetric>,
    /// Acquisition summary, when an acquisition receipt exists.
    pub acquisition: Option<AcquisitionResult>,
}

/// Expected semantics declared by a fixture case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedClosureSemantics {
    /// Expected closure status.
    pub expected_status: String,
    /// Expected evidence-qualified service-effect claim.
    pub service_effect_claim: String,
    /// Event identifiers declared as detected by the fixture owner.
    pub expected_detection_event_ids: Vec<String>,
    /// Expected false-alarm count; None means unscorable/unavailable.
    pub expected_false_alarm_episodes: Option<u64>,
    /// Explanatory notes.
    pub notes: Vec<String>,
}

/// One matched control/candidate decision episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureEpisode {
    /// Immutable matched-pair identity.
    pub decision_episode_id: String,
    /// Fixture case kind.
    pub case_kind: String,
    /// Existing-information control arm.
    pub control: ClosureArm,
    /// Candidate policy arm.
    pub candidate: ClosureArm,
    /// Owner-declared expected semantics.
    pub expected: ExpectedClosureSemantics,
}

/// Shared conditions frozen across both arms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureCommon {
    /// Paired experiment block.
    pub block_id: String,
    /// Common initial state reference.
    pub initial_state_ref: String,
    /// Common topology reference.
    pub topology_ref: String,
    /// Common exogenous schedule reference.
    pub exogenous_schedule_ref: String,
    /// Common model family.
    pub model_family_ref: String,
    /// Common time grid.
    pub time_grid_ref: String,
    /// Horizon ticks.
    pub horizon_ticks: u64,
    /// Common resource envelope.
    pub resource_envelope_ref: String,
    /// Common evidence class.
    pub evidence_class: MetricEvidenceStatus,
}

/// Parsed and semantically qualified v0.1 closure fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureFixtureReport {
    /// Fixture identifier.
    pub fixture_id: String,
    /// Fixture contract version.
    pub contract_version: String,
    /// Declared claim ceiling.
    pub claim_ceiling: String,
    /// Shared scenario envelope.
    pub common: ClosureCommon,
    /// Parsed and validated matched episodes.
    pub episodes: Vec<ClosureEpisode>,
}

impl ClosureFixtureReport {
    /// Render stable line-oriented references and evidence statuses.
    ///
    /// This is deterministic serialization, not a signature or proof of truth.
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(&mut out, "CIVBENCH_DECISION_CLOSURE_FIXTURE_V0.1");
        let _ = writeln!(&mut out, "fixture_id={}", self.fixture_id);
        let _ = writeln!(&mut out, "contract_version={}", self.contract_version);
        let _ = writeln!(&mut out, "block_id={}", self.common.block_id);
        let _ = writeln!(&mut out, "evidence_class={}", metric_status_name(self.common.evidence_class));
        for episode in &self.episodes {
            let _ = writeln!(
                &mut out,
                "episode={},kind={},status={},claim={}",
                episode.decision_episode_id,
                episode.case_kind,
                episode.expected.expected_status,
                episode.expected.service_effect_claim
            );
            append_arm(&mut out, "control", &episode.control);
            append_arm(&mut out, "candidate", &episode.candidate);
        }
        out
    }
}

fn append_arm(out: &mut String, label: &str, arm: &ClosureArm) {
    let record = &arm.record;
    let _ = writeln!(
        out,
        "arm={},{},disposition={:?},feasibility={:?},settlement={:?},progress={:?},effect={:?}",
        label, arm.policy_ref, record.human_disposition, record.resource_feasibility,
        record.resource_settlement, record.action_progress, record.service_effect
    );
    let mut bindings = record.receipt_bindings.clone();
    bindings.sort_by(|a, b| a.owner.cmp(&b.owner).then_with(|| a.receipt_id.cmp(&b.receipt_id)));
    for binding in bindings {
        let _ = writeln!(
            out,
            "binding={:?},{},{},{},{}",
            binding.owner, binding.receipt_id, binding.decision_episode_id,
            binding.paired_block_id, binding.payload_digest
        );
    }
    let mut metrics = arm.outcomes.clone();
    metrics.sort_by(|a, b| a.name.cmp(&b.name));
    for metric in metrics {
        let _ = writeln!(
            out,
            "metric={},{},{}",
            metric.name,
            metric_status_name(metric.status),
            metric.value.map_or_else(|| "none".to_owned(), |value| value.to_string())
        );
    }
}

fn metric_status_name(status: MetricEvidenceStatus) -> &'static str {
    match status {
        MetricEvidenceStatus::Observed => "observed",
        MetricEvidenceStatus::Derived => "derived",
        MetricEvidenceStatus::Modelled => "modelled",
        MetricEvidenceStatus::NotMeasured => "not_measured",
        MetricEvidenceStatus::Unsupported => "unsupported",
        MetricEvidenceStatus::Withheld => "withheld",
        MetricEvidenceStatus::NotApplicable => "not_applicable",
    }
}

/// Parse and validate the complete five-case v0.1 fixture.
pub fn parse_closure_fixture(input: &str) -> Result<ClosureFixtureReport, FixtureError> {
    if input.len() > MAX_FIXTURE_BYTES {
        return Err(FixtureError::new("fixture_too_large", "$", "fixture exceeds the 2 MiB safety limit"));
    }
    let root = JsonParser::new(input).parse()?;
    let root = object(
        &root,
        "$",
        &["fixture_id", "contract_version", "status", "claim_ceiling", "dataset_lane", "common", "decision_episodes"],
    )?;
    let fixture_id = string_field(root, "fixture_id", "$")?;
    if fixture_id != EXPECTED_FIXTURE_ID {
        return Err(FixtureError::new("unsupported_fixture_id", "$.fixture_id", "unexpected fixture identifier"));
    }
    let contract_version = string_field(root, "contract_version", "$")?;
    if contract_version != "0.1.0" {
        return Err(FixtureError::new("unsupported_contract_version", "$.contract_version", "only 0.1.0 is supported"));
    }
    if string_field(root, "status", "$")? != "synthetic_composition_fixture_only" {
        return Err(FixtureError::new("invalid_fixture_status", "$.status", "fixture must remain explicitly synthetic"));
    }
    if string_field(root, "dataset_lane", "$")? != "illustrative_synthetic" {
        return Err(FixtureError::new("invalid_dataset_lane", "$.dataset_lane", "fixture must remain synthetic"));
    }
    let claim_ceiling = string_field(root, "claim_ceiling", "$")?.to_owned();
    let common = parse_common(field(root, "common", "$")?, "$.common")?;
    let values = array(field(root, "decision_episodes", "$")?, "$.decision_episodes")?;
    if values.len() != 5 {
        return Err(FixtureError::new("invalid_episode_count", "$.decision_episodes", "v0.1 requires exactly five frozen cases"));
    }
    let mut seen_ids = BTreeSet::new();
    let mut episodes = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let path = format!("$.decision_episodes[{index}]");
        let episode = parse_episode(value, &common, &path)?;
        if !seen_ids.insert(episode.decision_episode_id.clone()) {
            return Err(FixtureError::new("duplicate_decision_episode", path, "episode identifiers must be unique"));
        }
        episodes.push(episode);
    }
    validate_case_coverage(&episodes)?;
    Ok(ClosureFixtureReport {
        fixture_id: fixture_id.to_owned(),
        contract_version: contract_version.to_owned(),
        claim_ceiling,
        common,
        episodes,
    })
}

fn validate_case_coverage(episodes: &[ClosureEpisode]) -> Result<(), FixtureError> {
    for required_kind in [
        "positive_service_restoration",
        "no_value_sensor",
        "false_alarm_overload",
        "resource_exhaustion",
        "unverified_repair",
    ] {
        if !episodes.iter().any(|episode| episode.case_kind == required_kind) {
            return Err(FixtureError::new("missing_required_case", "$.decision_episodes", format!("missing '{required_kind}'")));
        }
    }
    Ok(())
}

fn parse_common(value: &JsonValue, path: &str) -> Result<ClosureCommon, FixtureError> {
    let object = object(
        value,
        path,
        &["block_id", "initial_state_ref", "topology_ref", "exogenous_schedule_ref", "model_family_ref", "time_grid_ref", "horizon_ticks", "resource_envelope_ref", "evidence_class"],
    )?;
    let evidence_class = parse_metric_status(string_field(object, "evidence_class", path)?, path)?;
    if evidence_class != MetricEvidenceStatus::Modelled {
        return Err(FixtureError::new("fixture_evidence_class_not_synthetic", path, "v0.1 common evidence class must be modelled"));
    }
    Ok(ClosureCommon {
        block_id: reference_field(object, "block_id", path)?,
        initial_state_ref: reference_field(object, "initial_state_ref", path)?,
        topology_ref: reference_field(object, "topology_ref", path)?,
        exogenous_schedule_ref: reference_field(object, "exogenous_schedule_ref", path)?,
        model_family_ref: reference_field(object, "model_family_ref", path)?,
        time_grid_ref: reference_field(object, "time_grid_ref", path)?,
        horizon_ticks: u64_field(object, "horizon_ticks", path)?,
        resource_envelope_ref: reference_field(object, "resource_envelope_ref", path)?,
        evidence_class,
    })
}

fn parse_episode(value: &JsonValue, common: &ClosureCommon, path: &str) -> Result<ClosureEpisode, FixtureError> {
    let object = object(value, path, &["decision_episode_id", "case_kind", "control", "candidate", "expected_semantics"])?;
    let id = reference_field(object, "decision_episode_id", path)?;
    let case_kind = string_field(object, "case_kind", path)?.to_owned();
    let expected = parse_expected(field(object, "expected_semantics", path)?, path)?;
    let control = parse_arm(
        field(object, "control", path)?,
        &id,
        common,
        &format!("{path}.control"),
        ArmRole::Control,
        None,
    )?;
    let candidate = parse_arm(
        field(object, "candidate", path)?,
        &id,
        common,
        &format!("{path}.candidate"),
        ArmRole::Candidate,
        Some(expected.service_effect_claim.as_str()),
    )?;
    validate_matched_arms(&control, &candidate, &id, common, path)?;
    validate_expected(&case_kind, &candidate, &expected, path)?;
    Ok(ClosureEpisode { decision_episode_id: id, case_kind, control, candidate, expected })
}

#[derive(Clone, Copy)]
enum ArmRole {
    Control,
    Candidate,
}

fn parse_arm(
    value: &JsonValue,
    episode_id: &str,
    common: &ClosureCommon,
    path: &str,
    role: ArmRole,
    expected_claim: Option<&str>,
) -> Result<ClosureArm, FixtureError> {
    let object = object(
        value,
        path,
        &[
            "policy_ref", "event_evaluator_receipt_ref", "acquisition_receipt_ref", "acquisition_summary",
            "human_disposition", "feasibility", "resource_ledger_receipt_ref", "workflow_receipt_ref",
            "verification_receipt_ref", "service_outcome", "human_authority_receipt_ref", "human_actor_ref",
            "service_outcome_receipt_ref", "paired_outcome_receipt_ref", "action_progress", "resource_settlement",
            "executor_identity_ref", "verifier_identity_ref", "receipt_bindings",
        ],
    )?;
    let policy_ref = reference_field(object, "policy_ref", path)?;
    let event_receipt_id = reference_field(object, "event_evaluator_receipt_ref", path)?;
    let acquisition_receipt_id = optional_reference_field(object, "acquisition_receipt_ref", path)?;
    let acquisition = parse_optional_acquisition(object, path)?;
    if acquisition_receipt_id.is_some() != acquisition.is_some() {
        return Err(FixtureError::new("acquisition_reference_summary_mismatch", path, "acquisition reference and summary must both be present or both be null"));
    }
    let human_disposition = parse_disposition(string_field(object, "human_disposition", path)?, path)?;
    let resource_feasibility = parse_feasibility(string_field(object, "feasibility", path)?, path)?;
    let resource_settlement = parse_settlement(string_field(object, "resource_settlement", path)?, path)?;
    let action_progress = parse_progress(string_field(object, "action_progress", path)?, path)?;
    let resource_receipt_id = optional_reference_field(object, "resource_ledger_receipt_ref", path)?;
    let workflow_receipt_id = optional_reference_field(object, "workflow_receipt_ref", path)?;
    let verification_receipt_id = optional_reference_field(object, "verification_receipt_ref", path)?;
    let human_authority_receipt_id = Some(reference_field(object, "human_authority_receipt_ref", path)?);
    let human_actor_ref = Some(reference_field(object, "human_actor_ref", path)?);
    let service_outcome_receipt_id = Some(reference_field(object, "service_outcome_receipt_ref", path)?);
    let paired_outcome_receipt_id = Some(reference_field(object, "paired_outcome_receipt_ref", path)?);
    let executor_identity_ref = optional_reference_field(object, "executor_identity_ref", path)?;
    let verifier_identity_ref = optional_reference_field(object, "verifier_identity_ref", path)?;
    let outcomes = parse_outcome(field(object, "service_outcome", path)?, path)?;
    let service_evidence_class = uniform_service_evidence(&outcomes, path)?;
    let (record_evidence_class, service_effect) = match role {
        // A control arm supplies the reference measurements, not a treatment
        // effect claim. Its metric evidence remains on ClosureArm::outcomes.
        ArmRole::Control => (None, ServiceEffect::Indeterminate),
        ArmRole::Candidate => {
            let effect = candidate_effect_from_claim(
                expected_claim.ok_or_else(|| {
                    FixtureError::new("missing_candidate_effect_claim", path, "candidate arm requires expected service-effect claim")
                })?,
                service_evidence_class,
                path,
            )?;
            (service_evidence_class, effect)
        }
    };
    let receipt_bindings = parse_receipt_bindings(field(object, "receipt_bindings", path)?, episode_id, &common.block_id, path)?;
    let record = DecisionClosureRecord {
        decision_episode_id: episode_id.to_owned(),
        paired_block_id: common.block_id.clone(),
        event_receipt_id,
        acquisition_receipt_id,
        decision_changed_after_acquisition: acquisition.as_ref().map(|value| value.decision_changed),
        human_disposition,
        human_authority_receipt_id,
        human_actor_ref,
        resource_feasibility,
        resource_settlement,
        executor_identity_ref,
        verifier_identity_ref,
        receipt_bindings,
        action_progress,
        resource_receipt_id,
        workflow_receipt_id,
        verification_receipt_id,
        service_outcome_receipt_id,
        paired_outcome_receipt_id,
        service_evidence_class: record_evidence_class,
        service_effect,
    };
    record.validate().map_err(|error| FixtureError::new("closure_record_invalid", path, error.to_string()))?;
    Ok(ClosureArm { policy_ref, record, outcomes, acquisition })
}

fn validate_matched_arms(
    control: &ClosureArm,
    candidate: &ClosureArm,
    episode_id: &str,
    common: &ClosureCommon,
    path: &str,
) -> Result<(), FixtureError> {
    if control.policy_ref == candidate.policy_ref {
        return Err(FixtureError::new("identical_paired_policy", path, "control and candidate policies must differ"));
    }
    for record in [&control.record, &candidate.record] {
        if record.decision_episode_id != episode_id || record.paired_block_id != common.block_id {
            return Err(FixtureError::new("paired_context_mismatch", path, "arm is not bound to the shared decision episode and block"));
        }
    }
    if control.record.paired_outcome_receipt_id != candidate.record.paired_outcome_receipt_id {
        return Err(FixtureError::new("paired_outcome_receipt_mismatch", path, "both arms must share one paired-outcome receipt"));
    }
    if control.outcomes.len() != candidate.outcomes.len() {
        return Err(FixtureError::new("paired_metric_count_mismatch", path, "matched arms must contain the same outcome metric count"));
    }
    for left in &control.outcomes {
        let right = candidate.outcomes.iter().find(|metric| metric.name == left.name).ok_or_else(|| {
            FixtureError::new("paired_metric_missing", path, format!("candidate is missing metric '{}'", left.name))
        })?;
        if left.status != right.status {
            return Err(FixtureError::new(
                "cross_evidence_status_comparison",
                path,
                format!("metric '{}' has {:?} in control and {:?} in candidate; cross-status subtraction is prohibited", left.name, left.status, right.status),
            ));
        }
    }
    Ok(())
}

fn parse_expected(value: &JsonValue, path: &str) -> Result<ExpectedClosureSemantics, FixtureError> {
    let expected_path = format!("{path}.expected_semantics");
    let object = as_object(value, &expected_path)?;
    let allowed = [
        "expected_status",
        "service_effect_claim",
        "expected_detection_event_ids",
        "expected_false_alarm_episodes",
        "notes",
    ];
    reject_unknown_keys(object, &allowed, &expected_path)?;
    for key in ["expected_status", "service_effect_claim", "expected_detection_event_ids", "notes"] {
        if !object.contains_key(key) {
            return Err(FixtureError::new(
                "missing_required_field",
                &expected_path,
                format!("required field '{key}' is absent"),
            ));
        }
    }
    let expected_status = string_field(object, "expected_status", path)?.to_owned();
    let service_effect_claim = string_field(object, "service_effect_claim", path)?.to_owned();
    let events = array(field(object, "expected_detection_event_ids", path)?, path)?;
    let mut event_ids = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, value) in events.iter().enumerate() {
        let id = nonempty_string(value, &format!("{path}.expected_detection_event_ids[{index}]"))?;
        if !seen.insert(id.clone()) {
            return Err(FixtureError::new("duplicate_expected_event_id", path, "expected event IDs must be unique"));
        }
        event_ids.push(id);
    }
    let false_alarm_count = match object.get("expected_false_alarm_episodes") {
        None | Some(JsonValue::Null) => None,
        Some(JsonValue::Number(number)) => Some(parse_u64(number, path)?),
        Some(_) => return Err(FixtureError::new("invalid_false_alarm_count", path, "expected integer or null")),
    };
    let note_values = array(field(object, "notes", path)?, path)?;
    if note_values.is_empty() {
        return Err(FixtureError::new("missing_expected_notes", path, "at least one note is required"));
    }
    let notes = note_values.iter().enumerate().map(|(index, value)| {
        nonempty_string(value, &format!("{path}.notes[{index}]"))
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(ExpectedClosureSemantics {
        expected_status,
        service_effect_claim,
        expected_detection_event_ids: event_ids,
        expected_false_alarm_episodes: false_alarm_count,
        notes,
    })
}

fn validate_expected(
    case_kind: &str,
    candidate: &ClosureArm,
    expected: &ExpectedClosureSemantics,
    path: &str,
) -> Result<(), FixtureError> {
    let record = &candidate.record;
    let (status, expected_effect) = match expected.service_effect_claim.as_str() {
        "modelled_only" => ("verified_service_outcome", ServiceEffect::ModelledImprovement),
        "no_improvement_modelled" => ("no_verified_service_improvement", ServiceEffect::ModelledNoImprovement),
        "worse_modelled_service" => ("service_regression_from_capacity_contention", ServiceEffect::ModelledRegression),
        "no_action_no_effect" => ("authorized_but_not_executable", ServiceEffect::ModelledNoImprovement),
        "not_measured" => ("unverified_no_service_claim", ServiceEffect::Indeterminate),
        _ => return Err(FixtureError::new("unknown_expected_effect_claim", path, "unsupported service-effect claim")),
    };
    if expected.expected_status != status {
        return Err(FixtureError::new("expected_status_effect_mismatch", path, "expected status and effect claim are inconsistent"));
    }
    let evidence = uniform_service_evidence(&candidate.outcomes, path)?;
    if record.service_effect != expected_effect {
        return Err(FixtureError::new(
            "candidate_effect_does_not_match_expected_claim",
            path,
            "typed closure record effect differs from fixture expected claim",
        ));
    }
    if matches!(expected_effect, ServiceEffect::ModelledImprovement | ServiceEffect::ModelledNoImprovement | ServiceEffect::ModelledRegression)
        && evidence != Some(ServiceEvidenceClass::Modelled)
    {
        return Err(FixtureError::new("modelled_effect_without_modelled_outcomes", path, "modelled effect requires modelled outcomes"));
    }
    if expected_effect == ServiceEffect::Indeterminate && evidence.is_some() {
        return Err(FixtureError::new("indeterminate_effect_with_numeric_outcomes", path, "not-measured claim has measured outcome class"));
    }
    match case_kind {
        "positive_service_restoration" => {
            if record.action_progress != ActionProgress::IndependentlyVerified
                || record.human_disposition != HumanDisposition::Authorized
                || record.service_effect != ServiceEffect::ModelledImprovement
            {
                return Err(FixtureError::new("positive_case_not_closed", path, "positive outcome requires authorization and independent verification"));
            }
        }
        "no_value_sensor" => {
            if record.action_progress != ActionProgress::NotStarted
                || record.resource_feasibility != ResourceFeasibility::RejectedNoCapacity
                || record.service_effect != ServiceEffect::ModelledNoImprovement
            {
                return Err(FixtureError::new("no_value_case_inconsistent", path, "no-value case must remain no-action/no-improvement"));
            }
        }
        "false_alarm_overload" => {
            if record.action_progress != ActionProgress::IndependentlyVerified || record.service_effect != ServiceEffect::ModelledRegression {
                return Err(FixtureError::new("capacity_externality_case_inconsistent", path, "overload case requires verified modelled regression"));
            }
        }
        "resource_exhaustion" => {
            if record.human_disposition != HumanDisposition::Authorized
                || record.resource_feasibility != ResourceFeasibility::RejectedMissingPart
                || record.action_progress != ActionProgress::NotStarted
                || record.service_effect != ServiceEffect::ModelledNoImprovement
            {
                return Err(FixtureError::new("resource_exhaustion_case_inconsistent", path, "authorized but infeasible action must not execute"));
            }
        }
        "unverified_repair" => {
            if record.action_progress != ActionProgress::RepairClaimed
                || record.verification_receipt_id.is_some()
                || record.service_effect != ServiceEffect::Indeterminate
                || candidate.outcomes.iter().any(|metric| metric.value.is_some())
            {
                return Err(FixtureError::new("unverified_repair_case_inconsistent", path, "repair claim must remain unverified and unmeasured"));
            }
        }
        _ => return Err(FixtureError::new("unknown_case_kind", path, format!("unsupported case kind '{case_kind}'"))),
    }
    Ok(())
}

fn parse_optional_acquisition(
    parent: &BTreeMap<String, JsonValue>,
    path: &str,
) -> Result<Option<AcquisitionResult>, FixtureError> {
    match field(parent, "acquisition_summary", path)? {
        JsonValue::Null => Ok(None),
        value => {
            let acquisition = object(
                value,
                &format!("{path}.acquisition_summary"),
                &["valid_observation_returned", "decision_changed", "decision_value_status", "decision_value_minor", "acquisition_cost_minor"],
            )?;
            let status = parse_acquisition_status(string_field(acquisition, "decision_value_status", path)?, path)?;
            let value_minor = optional_i64_field(acquisition, "decision_value_minor", path)?;
            let measured = matches!(status, AcquisitionValueStatus::Modelled | AcquisitionValueStatus::Observed);
            if measured != value_minor.is_some() {
                return Err(FixtureError::new("acquisition_value_status_mismatch", path, "measured values must be numeric; unmeasured values must be null"));
            }
            let valid = bool_field(acquisition, "valid_observation_returned", path)?;
            let changed = bool_field(acquisition, "decision_changed", path)?;
            if changed && !valid {
                return Err(FixtureError::new("decision_changed_without_valid_observation", path, "invalid acquisition output cannot change a decision"));
            }
            Ok(Some(AcquisitionResult {
                valid_observation_returned: valid,
                decision_changed: changed,
                decision_value_status: status,
                decision_value_minor: value_minor,
                acquisition_cost_minor: u64_field(acquisition, "acquisition_cost_minor", path)?,
            }))
        }
    }
}

fn parse_outcome(value: &JsonValue, path: &str) -> Result<Vec<OutcomeMetric>, FixtureError> {
    let outcome_object = as_object(value, &format!("{path}.service_outcome"))?;
    let names = [
        "essential_shortfall_l", "interruption_service_seconds", "worst_group_service_ppm",
        "physical_loss_l", "verified_repairs", "whole_life_cost_minor", "recovery_seconds",
        "safe_fallback_delivery_l", "false_dispatches",
    ];
    reject_unknown_keys(outcome_object, &names, &format!("{path}.service_outcome"))?;
    if outcome_object.len() != names.len() {
        return Err(FixtureError::new("incomplete_outcome_vector", path, "all nine v0.1 outcome metrics are required"));
    }
    let mut metrics = Vec::with_capacity(names.len());
    for name in names {
        let metric_path = format!("{path}.service_outcome.{name}");
        let metric = object(
            field(outcome_object, name, &metric_path)?,
            &metric_path,
            &["value", "status"],
        )?;
        let status = parse_metric_status(string_field(metric, "status", &metric_path)?, &metric_path)?;
        let value = optional_i64_field(metric, "value", &metric_path)?;
        let measured = matches!(status, MetricEvidenceStatus::Observed | MetricEvidenceStatus::Derived | MetricEvidenceStatus::Modelled);
        if measured != value.is_some() {
            return Err(FixtureError::new("metric_status_value_mismatch", metric_path, "measured statuses require a value; unmeasured statuses require null"));
        }
        if value.is_some_and(|number| number < 0) {
            return Err(FixtureError::new("negative_outcome_metric", metric_path, "outcome metrics must be non-negative"));
        }
        if name == "worst_group_service_ppm" && value.is_some_and(|number| number > 1_000_000) {
            return Err(FixtureError::new("service_ppm_out_of_range", metric_path, "service ppm cannot exceed one million"));
        }
        metrics.push(OutcomeMetric { name: name.to_owned(), value, status });
    }
    Ok(metrics)
}

fn uniform_service_evidence(
    metrics: &[OutcomeMetric],
    path: &str,
) -> Result<Option<ServiceEvidenceClass>, FixtureError> {
    let Some(first) = metrics.first() else {
        return Err(FixtureError::new("empty_outcome_vector", path, "outcome vector cannot be empty"));
    };
    if metrics.iter().any(|metric| metric.status != first.status) {
        return Err(FixtureError::new("mixed_service_evidence_classes", path, "v0.1 closure claim requires uniform outcome evidence status"));
    }
    match first.status {
        MetricEvidenceStatus::Modelled => Ok(Some(ServiceEvidenceClass::Modelled)),
        MetricEvidenceStatus::Observed => Ok(Some(ServiceEvidenceClass::Observed)),
        MetricEvidenceStatus::Derived => Ok(Some(ServiceEvidenceClass::Derived)),
        _ => Ok(None),
    }
}

fn candidate_effect_from_claim(
    claim: &str,
    evidence: Option<ServiceEvidenceClass>,
    path: &str,
) -> Result<ServiceEffect, FixtureError> {
    let effect = match (claim, evidence) {
        ("modelled_only", Some(ServiceEvidenceClass::Modelled)) => ServiceEffect::ModelledImprovement,
        ("no_improvement_modelled", Some(ServiceEvidenceClass::Modelled)) => ServiceEffect::ModelledNoImprovement,
        ("worse_modelled_service", Some(ServiceEvidenceClass::Modelled)) => ServiceEffect::ModelledRegression,
        ("no_action_no_effect", Some(ServiceEvidenceClass::Modelled)) => ServiceEffect::ModelledNoImprovement,
        ("not_measured", None) => ServiceEffect::Indeterminate,
        ("modelled_only" | "no_improvement_modelled" | "worse_modelled_service" | "no_action_no_effect", _) => {
            return Err(FixtureError::new(
                "expected_effect_evidence_mismatch",
                path,
                "modelled claim requires a modelled outcome vector",
            ));
        }
        ("not_measured", Some(_)) => {
            return Err(FixtureError::new(
                "not_measured_claim_has_evidence",
                path,
                "not_measured claim cannot carry an observed/derived/modelled outcome vector",
            ));
        }
        _ => return Err(FixtureError::new("unknown_expected_effect_claim", path, "unsupported service-effect claim")),
    };
    Ok(effect)
}

fn parse_receipt_bindings(
    value: &JsonValue,
    episode_id: &str,
    block_id: &str,
    path: &str,
) -> Result<Vec<ReceiptBinding>, FixtureError> {
    let values = array(value, &format!("{path}.receipt_bindings"))?;
    let mut bindings = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}.receipt_bindings[{index}]");
        let object = object(value, &item_path, &["owner", "receipt_id", "payload_digest", "decision_episode_id", "paired_block_id"])?;
        let owner = parse_receipt_owner(string_field(object, "owner", &item_path)?, &item_path)?;
        let digest = string_field(object, "payload_digest", &item_path)?;
        let digest_hex = digest.strip_prefix("sha256:").ok_or_else(|| FixtureError::new("invalid_receipt_digest", &item_path, "expected sha256:<64 hex digits>"))?;
        if digest_hex.len() != 64 || !digest_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) || digest_hex.bytes().all(|byte| byte == b'0') {
            return Err(FixtureError::new("invalid_receipt_digest", &item_path, "digest must be non-zero 64-character hexadecimal"));
        }
        let bound_episode = reference_field(object, "decision_episode_id", &item_path)?;
        let bound_block = reference_field(object, "paired_block_id", &item_path)?;
        if bound_episode != episode_id || bound_block != block_id {
            return Err(FixtureError::new("receipt_context_mismatch", &item_path, "receipt must bind the enclosing episode and paired block"));
        }
        bindings.push(ReceiptBinding {
            owner,
            receipt_id: reference_field(object, "receipt_id", &item_path)?,
            decision_episode_id: bound_episode,
            paired_block_id: bound_block,
            payload_digest: digest_hex.to_owned(),
        });
    }
    Ok(bindings)
}

fn parse_disposition(value: &str, path: &str) -> Result<HumanDisposition, FixtureError> {
    match value {
        "authorized" => Ok(HumanDisposition::Authorized),
        "rejected" => Ok(HumanDisposition::Rejected),
        "deferred" => Ok(HumanDisposition::Deferred),
        _ => invalid_enum("human_disposition", value, path),
    }
}

fn parse_feasibility(value: &str, path: &str) -> Result<ResourceFeasibility, FixtureError> {
    match value {
        "feasible_and_reserved" => Ok(ResourceFeasibility::FeasibleAndReserved),
        "rejected_no_capacity" => Ok(ResourceFeasibility::RejectedNoCapacity),
        "rejected_missing_part" => Ok(ResourceFeasibility::RejectedMissingPart),
        "rejected_budget" => Ok(ResourceFeasibility::RejectedBudget),
        "rejected_energy_or_access" => Ok(ResourceFeasibility::RejectedEnergyOrAccess),
        "not_attempted" => Ok(ResourceFeasibility::NotAttempted),
        _ => invalid_enum("feasibility", value, path),
    }
}

fn parse_settlement(value: &str, path: &str) -> Result<ResourceSettlement, FixtureError> {
    match value {
        "not_attempted" => Ok(ResourceSettlement::NotAttempted),
        "rejected" => Ok(ResourceSettlement::Rejected),
        "reserved" => Ok(ResourceSettlement::Reserved),
        "settled" => Ok(ResourceSettlement::Settled),
        "released" => Ok(ResourceSettlement::Released),
        _ => invalid_enum("resource_settlement", value, path),
    }
}

fn parse_progress(value: &str, path: &str) -> Result<ActionProgress, FixtureError> {
    match value {
        "not_started" => Ok(ActionProgress::NotStarted),
        "planned" => Ok(ActionProgress::Planned),
        "in_progress" => Ok(ActionProgress::InProgress),
        "repair_claimed" => Ok(ActionProgress::RepairClaimed),
        "independently_verified" => Ok(ActionProgress::IndependentlyVerified),
        "cancelled" => Ok(ActionProgress::Cancelled),
        _ => invalid_enum("action_progress", value, path),
    }
}

fn parse_receipt_owner(value: &str, path: &str) -> Result<ReceiptOwner, FixtureError> {
    match value {
        "event_evaluator" => Ok(ReceiptOwner::EventEvaluator),
        "acquisition_planner" => Ok(ReceiptOwner::AcquisitionPlanner),
        "human_authority" => Ok(ReceiptOwner::HumanAuthority),
        "resource_ledger" => Ok(ReceiptOwner::ResourceLedger),
        "repair_workflow" => Ok(ReceiptOwner::RepairWorkflow),
        "independent_verifier" => Ok(ReceiptOwner::IndependentVerifier),
        "service_outcome" => Ok(ReceiptOwner::ServiceOutcome),
        "paired_outcome" => Ok(ReceiptOwner::PairedOutcome),
        _ => invalid_enum("receipt_owner", value, path),
    }
}

fn parse_metric_status(value: &str, path: &str) -> Result<MetricEvidenceStatus, FixtureError> {
    match value {
        "observed" => Ok(MetricEvidenceStatus::Observed),
        "derived" => Ok(MetricEvidenceStatus::Derived),
        "modelled" => Ok(MetricEvidenceStatus::Modelled),
        "not_measured" => Ok(MetricEvidenceStatus::NotMeasured),
        "unsupported" => Ok(MetricEvidenceStatus::Unsupported),
        "withheld" => Ok(MetricEvidenceStatus::Withheld),
        "not_applicable" => Ok(MetricEvidenceStatus::NotApplicable),
        _ => invalid_enum("metric_status", value, path),
    }
}

fn parse_acquisition_status(value: &str, path: &str) -> Result<AcquisitionValueStatus, FixtureError> {
    match value {
        "modelled" => Ok(AcquisitionValueStatus::Modelled),
        "observed" => Ok(AcquisitionValueStatus::Observed),
        "not_measured" => Ok(AcquisitionValueStatus::NotMeasured),
        "unsupported" => Ok(AcquisitionValueStatus::Unsupported),
        _ => invalid_enum("acquisition_value_status", value, path),
    }
}

fn invalid_enum<T>(field: &'static str, value: &str, path: &str) -> Result<T, FixtureError> {
    Err(FixtureError::new("invalid_enum_value", path, format!("unsupported {field} value '{value}'")))
}

fn object<'a>(
    value: &'a JsonValue,
    path: &str,
    allowed: &[&str],
) -> Result<&'a BTreeMap<String, JsonValue>, FixtureError> {
    let object = as_object(value, path)?;
    reject_unknown_keys(object, allowed, path)?;
    for key in allowed {
        if !object.contains_key(*key) {
            return Err(FixtureError::new("missing_required_field", path, format!("required field '{key}' is absent")));
        }
    }
    Ok(object)
}

fn as_object<'a>(value: &'a JsonValue, path: &str) -> Result<&'a BTreeMap<String, JsonValue>, FixtureError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(FixtureError::new("expected_object", path, "expected JSON object")),
    }
}

fn reject_unknown_keys(
    object: &BTreeMap<String, JsonValue>,
    allowed: &[&str],
    path: &str,
) -> Result<(), FixtureError> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(FixtureError::new("unknown_manifest_field", path, format!("unknown field '{key}' is not permitted")));
    }
    Ok(())
}

fn required<'a>(object: &'a BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<&'a JsonValue, FixtureError> {
    object.get(key).ok_or_else(|| FixtureError::new("missing_required_field", path, format!("required field '{key}' is absent")))
}

fn field<'a>(object: &'a BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<&'a JsonValue, FixtureError> {
    required(object, key, path)
}

fn string_field<'a>(object: &'a BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<&'a str, FixtureError> {
    match required(object, key, path)? {
        JsonValue::String(value) => Ok(value),
        _ => Err(FixtureError::new("expected_string", format!("{path}.{key}"), "field must be a string")),
    }
}

fn reference_field(object: &BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<String, FixtureError> {
    let value = string_field(object, key, path)?;
    validate_reference(value, &format!("{path}.{key}"))?;
    Ok(value.to_owned())
}

fn optional_reference_field(object: &BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<Option<String>, FixtureError> {
    match required(object, key, path)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) => {
            validate_reference(value, &format!("{path}.{key}"))?;
            Ok(Some(value.clone()))
        }
        _ => Err(FixtureError::new("expected_optional_reference", format!("{path}.{key}"), "expected string or null")),
    }
}

fn validate_reference(value: &str, path: &str) -> Result<(), FixtureError> {
    if value.trim().is_empty() || value.trim() != value || value.chars().any(char::is_whitespace) {
        return Err(FixtureError::new("invalid_reference", path, "reference must be non-empty and whitespace-free"));
    }
    Ok(())
}

fn bool_field(object: &BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<bool, FixtureError> {
    match required(object, key, path)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(FixtureError::new("expected_boolean", format!("{path}.{key}"), "field must be a boolean")),
    }
}

fn u64_field(object: &BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<u64, FixtureError> {
    match required(object, key, path)? {
        JsonValue::Number(value) => parse_u64(value, &format!("{path}.{key}")),
        _ => Err(FixtureError::new("expected_integer", format!("{path}.{key}"), "field must be a non-negative integer")),
    }
}

fn optional_i64_field(object: &BTreeMap<String, JsonValue>, key: &str, path: &str) -> Result<Option<i64>, FixtureError> {
    match required(object, key, path)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(value) => value.parse::<i64>().map(Some).map_err(|_| {
            FixtureError::new("expected_integer", format!("{path}.{key}"), "field must be a signed integer or null")
        }),
        _ => Err(FixtureError::new("expected_optional_integer", format!("{path}.{key}"), "field must be integer or null")),
    }
}

fn parse_u64(value: &str, path: &str) -> Result<u64, FixtureError> {
    value.parse::<u64>().map_err(|_| FixtureError::new("expected_unsigned_integer", path, "expected a non-negative integer"))
}

fn array<'a>(value: &'a JsonValue, path: &str) -> Result<&'a [JsonValue], FixtureError> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(FixtureError::new("expected_array", path, "field must be an array")),
    }
}

fn nonempty_string(value: &JsonValue, path: &str) -> Result<String, FixtureError> {
    match value {
        JsonValue::String(value) => {
            validate_reference(value, path)?;
            Ok(value.clone())
        }
        _ => Err(FixtureError::new("expected_string", path, "expected a non-empty string")),
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn parser_rejects_duplicate_keys() {
        let error = JsonParser::new(r#"{"key":1,"key":2}"#).parse().unwrap_err();
        assert_eq!(error.code, "duplicate_json_key");
    }

    #[test]
    fn parser_decodes_surrogate_pairs() {
        let parsed = JsonParser::new(r#""\ud83d\ude80""#).parse().expect("valid surrogate pair");
        assert_eq!(parsed, JsonValue::String("🚀".into()));
    }

    #[test]
    fn parser_rejects_unpaired_surrogates() {
        let error = JsonParser::new(r#""\ud800""#).parse().unwrap_err();
        assert_eq!(error.code, "invalid_unicode_surrogate");
    }

    #[test]
    fn parser_rejects_leading_zero_numbers() {
        let error = JsonParser::new(r#"{"key":01}"#).parse().unwrap_err();
        assert_eq!(error.code, "invalid_json_number");
    }

    #[test]
    fn parser_rejects_trailing_data() {
        let error = JsonParser::new(r#"{} {}"#).parse().unwrap_err();
        assert_eq!(error.code, "trailing_json_data");
    }

    #[test]
    fn parser_rejects_excessive_nesting() {
        let nested = format!("{}0{}", "[".repeat(MAX_JSON_DEPTH + 2), "]".repeat(MAX_JSON_DEPTH + 2));
        let error = JsonParser::new(&nested).parse().unwrap_err();
        assert_eq!(error.code, "json_nesting_limit");
    }
}
