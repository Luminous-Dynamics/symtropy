// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dependency-free evidence contract for solver-work coverage.
//!
//! This crate validates what an execution reports; it does not run a solver,
//! certify physical truth, authenticate caller-supplied digests, or grant
//! engineering/actuation authority.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub mod capacity;

pub const SCHEMA_VERSION: &str = "solver-coverage-v0.1";
const SHA256_HEX_LEN: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationError {
    pub path: String,
    pub reason: String,
}

impl ValidationError {
    fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.reason)
    }
}

impl Error for ValidationError {}

fn is_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LEN && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn checked_sum(values: &[u64], path: &str) -> Result<u64, ValidationError> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| ValidationError::new(path, "row count arithmetic overflow"))
    })
}

/// Exact source/run bindings. The SHA-256 values are caller-supplied claims;
/// consumers must hash the canonical preimage and compare them independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionIdentity {
    pub run_id: String,
    pub scenario_sha256: String,
    pub engine_build_sha256: String,
    pub solver_profile_sha256: String,
    pub capability_census_sha256: String,
    pub request_set_sha256: String,
}

impl ExecutionIdentity {
    fn validate(&self) -> Result<(), ValidationError> {
        if !non_empty(&self.run_id) {
            return Err(ValidationError::new("identity.run_id", "must not be empty"));
        }
        for (name, value) in [
            ("scenario_sha256", &self.scenario_sha256),
            ("engine_build_sha256", &self.engine_build_sha256),
            ("solver_profile_sha256", &self.solver_profile_sha256),
            ("capability_census_sha256", &self.capability_census_sha256),
            ("request_set_sha256", &self.request_set_sha256),
        ] {
            if !is_sha256(value) {
                return Err(ValidationError::new(
                    format!("identity.{name}"),
                    "must be exactly 64 hexadecimal characters",
                ));
            }
        }
        Ok(())
    }

    fn append_without_request_digest(&self, out: &mut Vec<u8>) {
        push_string(out, &self.run_id);
        push_string(out, &self.scenario_sha256.to_ascii_lowercase());
        push_string(out, &self.engine_build_sha256.to_ascii_lowercase());
        push_string(out, &self.solver_profile_sha256.to_ascii_lowercase());
        push_string(out, &self.capability_census_sha256.to_ascii_lowercase());
    }

    fn append_all(&self, out: &mut Vec<u8>) {
        self.append_without_request_digest(out);
        push_string(out, &self.request_set_sha256.to_ascii_lowercase());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum ConstraintRowFamily {
    Articulation = 1,
    FreeBody = 2,
    Propagation = 3,
    Contact = 4,
    ContactReduction = 5,
    SoftContact = 6,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ObservableKey {
    pub entity_id: String,
    /// One semantic component per key, e.g. contact.normal_impulse or contact.torsion.
    pub observable_id: String,
}

impl ObservableKey {
    fn validate(&self, path: &str) -> Result<(), ValidationError> {
        if !non_empty(&self.entity_id) {
            return Err(ValidationError::new(
                format!("{path}.entity_id"),
                "must not be empty",
            ));
        }
        if !non_empty(&self.observable_id) {
            return Err(ValidationError::new(
                format!("{path}.observable_id"),
                "must not be empty",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservableSpec {
    pub key: ObservableKey,
    /// Every catalogued key has a record. False keys must be NotRequested.
    pub requested: bool,
    /// Required outputs must be requested and Produced for full-work completion.
    pub required: bool,
    pub unit: String,
    pub frame_id: String,
    pub row_family: Option<ConstraintRowFamily>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ObservableDisposition {
    Produced = 1,
    NotRequested = 2,
    Unsupported = 3,
    CapacityDropped = 4,
    SleepingSkipped = 5,
    NotConverged = 6,
    Invalid = 7,
    /// Explicit evidence gap. A receipt containing this status is rejected.
    Missing = 8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UncertaintySemantics {
    /// A value was produced, but the producer did not quantify uncertainty.
    NotQuantified { reason: String },
    /// A producer-defined interval in the declared value unit.
    Interval {
        lower: f64,
        upper: f64,
        method_id: String,
    },
}

impl UncertaintySemantics {
    fn validate(&self, value: f64, path: &str) -> Result<(), ValidationError> {
        match self {
            Self::NotQuantified { reason } => {
                if !non_empty(reason) {
                    return Err(ValidationError::new(
                        path,
                        "NotQuantified requires a reason",
                    ));
                }
            }
            Self::Interval {
                lower,
                upper,
                method_id,
            } => {
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(ValidationError::new(
                        path,
                        "interval must be finite and lower <= upper",
                    ));
                }
                if value < *lower || value > *upper {
                    return Err(ValidationError::new(
                        path,
                        "produced value lies outside its declared interval",
                    ));
                }
                if !non_empty(method_id) {
                    return Err(ValidationError::new(
                        path,
                        "interval requires a method identity",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservableResult {
    pub key: ObservableKey,
    pub disposition: ObservableDisposition,
    /// Scalar component only. Vectors/tensors are represented by multiple stable keys.
    pub value: Option<f64>,
    /// Producer path/adapter identity. Required only when a value was produced.
    pub producer_id: Option<String>,
    /// Required for Produced; NotQuantified is explicit and is not zero uncertainty.
    pub uncertainty: Option<UncertaintySemantics>,
    /// Required for every requested non-produced disposition.
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintRowCoverage {
    pub family: ConstraintRowFamily,
    pub capacity_rows: u64,
    pub reserved_rows: u64,
    pub requested_rows: u64,
    pub admitted_rows: u64,
    pub solved_rows: u64,
    pub not_converged_rows: u64,
    pub sleeping_skipped_rows: u64,
    pub capacity_dropped_rows: u64,
    pub unsupported_rows: u64,
    pub execution_aborted_rows: u64,
}

impl ConstraintRowCoverage {
    fn validate(&self, path: &str) -> Result<bool, ValidationError> {
        if self.reserved_rows > self.capacity_rows {
            return Err(ValidationError::new(
                path,
                "reserved rows exceed configured capacity",
            ));
        }
        if self.admitted_rows > self.reserved_rows {
            return Err(ValidationError::new(
                path,
                "admitted rows exceed reserved rows",
            ));
        }

        let requested_partition = checked_sum(
            &[
                self.admitted_rows,
                self.sleeping_skipped_rows,
                self.capacity_dropped_rows,
                self.unsupported_rows,
            ],
            path,
        )?;
        if requested_partition != self.requested_rows {
            return Err(ValidationError::new(
                path,
                "requested_rows must equal admitted + sleeping_skipped + capacity_dropped + unsupported",
            ));
        }

        let admitted_partition = checked_sum(
            &[
                self.solved_rows,
                self.not_converged_rows,
                self.execution_aborted_rows,
            ],
            path,
        )?;
        if admitted_partition != self.admitted_rows {
            return Err(ValidationError::new(
                path,
                "admitted_rows must equal solved + not_converged + execution_aborted",
            ));
        }

        Ok(self.solved_rows == self.requested_rows
            && self.not_converged_rows == 0
            && self.sleeping_skipped_rows == 0
            && self.capacity_dropped_rows == 0
            && self.unsupported_rows == 0
            && self.execution_aborted_rows == 0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoverageRequest {
    pub identity: ExecutionIdentity,
    /// Frozen catalog for this exact run. Unrequested entries must say NotRequested.
    pub observables: Vec<ObservableSpec>,
    /// Every declared row family must have exactly one aggregate coverage record.
    pub required_row_families: Vec<ConstraintRowFamily>,
}

impl CoverageRequest {
    /// Canonical SHA-256 preimage for request_set_sha256.
    ///
    /// The hash function is intentionally outside this dependency-free crate.
    /// The caller must compute SHA-256 over these bytes and compare the result
    /// to identity.request_set_sha256 before treating the identity as verified.
    pub fn canonical_request_preimage(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"SYMCOV-REQUEST\0");
        push_string(&mut out, SCHEMA_VERSION);
        self.identity.append_without_request_digest(&mut out);

        let mut observables = self.observables.clone();
        observables.sort_by(|a, b| a.key.cmp(&b.key));
        push_u64(&mut out, observables.len() as u64);
        for spec in observables {
            push_string(&mut out, &spec.key.entity_id);
            push_string(&mut out, &spec.key.observable_id);
            out.push(u8::from(spec.requested));
            out.push(u8::from(spec.required));
            push_string(&mut out, &spec.unit);
            push_string(&mut out, &spec.frame_id);
            out.push(spec.row_family.map_or(0, |family| family as u8));
        }

        let mut families = self.required_row_families.clone();
        families.sort();
        push_u64(&mut out, families.len() as u64);
        for family in families {
            out.push(family as u8);
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoverageReceipt {
    pub request: CoverageRequest,
    pub observable_results: Vec<ObservableResult>,
    pub row_coverage: Vec<ConstraintRowCoverage>,
    /// Geometric manifold lineage and temporally scoped capacity diagnostics.
    pub capacity_evidence: capacity::CapacityEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageReport {
    pub protocol_complete: bool,
    pub catalogued_observables: usize,
    pub requested_observables: usize,
    pub required_observables: usize,
    pub produced_observables: usize,
    pub explicit_nonproduced_observables: usize,
    pub not_requested_observables: usize,
    pub required_observables_produced: bool,
    pub requested_rows: u64,
    pub solved_rows: u64,
    pub capacity_dropped_rows: u64,
    pub sleeping_skipped_rows: u64,
    pub unsupported_rows: u64,
    pub not_converged_rows: u64,
    pub execution_aborted_rows: u64,
    pub required_rows_solved: bool,
    /// False when aggregate family counts cannot attribute a produced value to a specific row.
    pub row_attribution_complete: bool,
    /// True only when every required observable and required row was produced/solved.
    pub complete_for_declared_required_work: bool,
    pub capacity_evidence: capacity::CapacityEvidenceReport,
}

/// Validate protocol completeness and return a quality-neutral coverage report.
///
/// Explicitly recorded omissions are valid evidence about an incomplete run.
/// Missing, duplicate, unexpected, malformed, or arithmetically inconsistent
/// records are protocol errors and are rejected.
/// Validate a receipt after the caller independently computes SHA-256 over
/// request.canonical_request_preimage().
///
/// This binds the caller-computed digest to the request identity. The caller
/// remains responsible for using a correct SHA-256 implementation.
pub fn validate_receipt_with_computed_request_digest(
    receipt: &CoverageReceipt,
    computed_request_sha256: &str,
) -> Result<CoverageReport, ValidationError> {
    receipt.request.identity.validate()?;
    if !is_sha256(computed_request_sha256) {
        return Err(ValidationError::new(
            "computed_request_sha256",
            "must be exactly 64 hexadecimal characters",
        ));
    }
    if !receipt
        .request
        .identity
        .request_set_sha256
        .eq_ignore_ascii_case(computed_request_sha256)
    {
        return Err(ValidationError::new(
            "identity.request_set_sha256",
            "does not match the caller-computed canonical request digest",
        ));
    }
    validate_receipt(receipt)
}

pub fn validate_receipt(receipt: &CoverageReceipt) -> Result<CoverageReport, ValidationError> {
    let request = &receipt.request;
    request.identity.validate()?;
    let capacity_evidence = receipt.capacity_evidence.validate()?;

    let mut specs = BTreeMap::new();
    for (index, spec) in request.observables.iter().enumerate() {
        let path = format!("request.observables[{index}]");
        spec.key.validate(&path)?;
        if !non_empty(&spec.unit) || !non_empty(&spec.frame_id) {
            return Err(ValidationError::new(
                &path,
                "unit and frame_id are required",
            ));
        }
        if spec.required && !spec.requested {
            return Err(ValidationError::new(
                &path,
                "a required observable must be requested",
            ));
        }
        if specs.insert(spec.key.clone(), spec).is_some() {
            return Err(ValidationError::new(&path, "duplicate observable key"));
        }
    }

    let mut required_families = BTreeSet::new();
    for family in &request.required_row_families {
        if !required_families.insert(*family) {
            return Err(ValidationError::new(
                "request.required_row_families",
                "duplicate row family",
            ));
        }
    }
    for spec in request
        .observables
        .iter()
        .filter(|spec| spec.row_family.is_some())
    {
        if !required_families.contains(&spec.row_family.expect("checked Some")) {
            return Err(ValidationError::new(
                "request.observables.row_family",
                "observable row family is absent from required_row_families",
            ));
        }
    }

    let mut results = BTreeMap::new();
    for (index, result) in receipt.observable_results.iter().enumerate() {
        let path = format!("observable_results[{index}]");
        result.key.validate(&path)?;
        let spec = specs
            .get(&result.key)
            .ok_or_else(|| ValidationError::new(&path, "unexpected observable result key"))?;
        if results.insert(result.key.clone(), result).is_some() {
            return Err(ValidationError::new(
                &path,
                "duplicate observable result key",
            ));
        }
        validate_observable_result(spec, result, &path)?;
    }

    for key in specs.keys() {
        if !results.contains_key(key) {
            return Err(ValidationError::new(
                format!(
                    "observable_results[{}:{}]",
                    key.entity_id, key.observable_id
                ),
                "requested/catalogued observable has no disposition",
            ));
        }
    }
    if results.len() != specs.len() {
        return Err(ValidationError::new(
            "observable_results",
            "result count does not match frozen catalog",
        ));
    }

    let mut row_records = BTreeMap::new();
    let mut required_rows_solved = true;
    let mut requested_rows = 0_u64;
    let mut solved_rows = 0_u64;
    let mut capacity_dropped_rows = 0_u64;
    let mut sleeping_skipped_rows = 0_u64;
    let mut unsupported_rows = 0_u64;
    let mut not_converged_rows = 0_u64;
    let mut execution_aborted_rows = 0_u64;

    for (index, rows) in receipt.row_coverage.iter().enumerate() {
        let path = format!("row_coverage[{index}]");
        if !required_families.contains(&rows.family) {
            return Err(ValidationError::new(&path, "unexpected row family"));
        }
        if row_records.insert(rows.family, rows).is_some() {
            return Err(ValidationError::new(
                &path,
                "duplicate row-family coverage record",
            ));
        }
        if !rows.validate(&path)? {
            required_rows_solved = false;
        }
        requested_rows = checked_sum(&[requested_rows, rows.requested_rows], &path)?;
        solved_rows = checked_sum(&[solved_rows, rows.solved_rows], &path)?;
        capacity_dropped_rows =
            checked_sum(&[capacity_dropped_rows, rows.capacity_dropped_rows], &path)?;
        sleeping_skipped_rows =
            checked_sum(&[sleeping_skipped_rows, rows.sleeping_skipped_rows], &path)?;
        unsupported_rows = checked_sum(&[unsupported_rows, rows.unsupported_rows], &path)?;
        not_converged_rows = checked_sum(&[not_converged_rows, rows.not_converged_rows], &path)?;
        execution_aborted_rows = checked_sum(
            &[execution_aborted_rows, rows.execution_aborted_rows],
            &path,
        )?;
    }

    for family in &request.required_row_families {
        if !row_records.contains_key(family) {
            return Err(ValidationError::new(
                "row_coverage",
                "required row family has no accounting record",
            ));
        }
    }
    if row_records.len() != required_families.len() {
        return Err(ValidationError::new(
            "row_coverage",
            "row-family record count mismatch",
        ));
    }

    let mut row_attribution_complete = true;
    for (key, spec) in &specs {
        let result = results.get(key).expect("all result keys checked");
        if result.disposition != ObservableDisposition::Produced {
            continue;
        }
        if let Some(family) = spec.row_family {
            let rows = row_records.get(&family).expect("row family checked above");
            if rows.requested_rows > 0 && rows.solved_rows == 0 {
                return Err(ValidationError::new(
                    format!(
                        "observable_results[{}:{}]",
                        key.entity_id, key.observable_id
                    ),
                    "cannot report a row-linked value as Produced when no requested rows in its family were solved",
                ));
            }
            if rows.solved_rows != rows.requested_rows {
                // Aggregate-only counts cannot prove which specific row produced this value.
                // Keep protocol completeness, but expose attribution uncertainty explicitly.
                row_attribution_complete = false;
            }
        }
    }

    let mut requested_observables = 0;
    let mut required_observables = 0;
    let mut produced_observables = 0;
    let mut explicit_nonproduced_observables = 0;
    let mut not_requested_observables = 0;
    let mut required_observables_produced = true;

    for (key, spec) in &specs {
        let result = results.get(key).expect("all result keys checked");
        if !spec.requested {
            not_requested_observables += 1;
            continue;
        }
        requested_observables += 1;
        if spec.required {
            required_observables += 1;
        }
        if result.disposition == ObservableDisposition::Produced {
            produced_observables += 1;
        } else {
            explicit_nonproduced_observables += 1;
            if spec.required {
                required_observables_produced = false;
            }
        }
    }

    let complete_for_declared_required_work = required_observables_produced && required_rows_solved;
    Ok(CoverageReport {
        protocol_complete: true,
        catalogued_observables: specs.len(),
        requested_observables,
        required_observables,
        produced_observables,
        explicit_nonproduced_observables,
        not_requested_observables,
        required_observables_produced,
        requested_rows,
        solved_rows,
        capacity_dropped_rows,
        sleeping_skipped_rows,
        unsupported_rows,
        not_converged_rows,
        execution_aborted_rows,
        required_rows_solved,
        row_attribution_complete,
        complete_for_declared_required_work,
        capacity_evidence,
    })
}

fn validate_observable_result(
    spec: &ObservableSpec,
    result: &ObservableResult,
    path: &str,
) -> Result<(), ValidationError> {
    match result.disposition {
        ObservableDisposition::Produced => {
            if !spec.requested {
                return Err(ValidationError::new(
                    path,
                    "unrequested observable cannot be Produced",
                ));
            }
            let value = result
                .value
                .ok_or_else(|| ValidationError::new(path, "Produced requires a value"))?;
            if !value.is_finite() {
                return Err(ValidationError::new(path, "Produced value must be finite"));
            }
            if !result.producer_id.as_deref().is_some_and(non_empty) {
                return Err(ValidationError::new(path, "Produced requires producer_id"));
            }
            let uncertainty = result.uncertainty.as_ref().ok_or_else(|| {
                ValidationError::new(path, "Produced requires explicit uncertainty semantics")
            })?;
            uncertainty.validate(value, &format!("{path}.uncertainty"))?;
            if result.reason.is_some() {
                return Err(ValidationError::new(
                    path,
                    "Produced cannot carry an omission reason",
                ));
            }
        }
        ObservableDisposition::NotRequested => {
            if spec.requested {
                return Err(ValidationError::new(
                    path,
                    "requested observable cannot be NotRequested",
                ));
            }
            if result.value.is_some()
                || result.producer_id.is_some()
                || result.uncertainty.is_some()
                || result.reason.is_some()
            {
                return Err(ValidationError::new(
                    path,
                    "NotRequested cannot carry a value or production metadata",
                ));
            }
        }
        ObservableDisposition::Missing => {
            return Err(ValidationError::new(
                path,
                "Missing is a protocol-completeness failure",
            ));
        }
        disposition => {
            if !spec.requested {
                return Err(ValidationError::new(
                    path,
                    "unrequested observable must be NotRequested",
                ));
            }
            if result.value.is_some() || result.uncertainty.is_some() {
                return Err(ValidationError::new(
                    path,
                    "non-produced disposition cannot carry a value or uncertainty",
                ));
            }
            if !result.reason.as_deref().is_some_and(non_empty) {
                return Err(ValidationError::new(
                    path,
                    "non-produced disposition requires a reason",
                ));
            }
            if result
                .producer_id
                .as_deref()
                .is_some_and(|producer| !non_empty(producer))
            {
                return Err(ValidationError::new(
                    path,
                    "producer_id, when present, must not be empty",
                ));
            }
            let _ = disposition;
        }
    }
    Ok(())
}

/// Canonical result preimage. Order of input vectors does not affect these bytes.
/// A caller must hash this preimage with SHA-256 and bind the digest externally.
pub fn canonical_receipt_preimage(receipt: &CoverageReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"SYMCOV-RECEIPT\0");
    push_string(&mut out, SCHEMA_VERSION);
    receipt.request.identity.append_all(&mut out);
    out.extend_from_slice(&receipt.request.canonical_request_preimage());
    out.extend_from_slice(&receipt.capacity_evidence.canonical_preimage());

    let mut results = receipt.observable_results.clone();
    results.sort_by(|a, b| a.key.cmp(&b.key));
    push_u64(&mut out, results.len() as u64);
    for result in results {
        push_string(&mut out, &result.key.entity_id);
        push_string(&mut out, &result.key.observable_id);
        out.push(result.disposition as u8);
        match result.value {
            Some(value) => {
                out.push(1);
                push_f64(&mut out, value);
            }
            None => out.push(0),
        }
        push_optional_string(&mut out, result.producer_id.as_deref());
        push_optional_string(&mut out, result.reason.as_deref());
        match result.uncertainty {
            Some(UncertaintySemantics::NotQuantified { reason }) => {
                out.push(1);
                push_string(&mut out, &reason);
            }
            Some(UncertaintySemantics::Interval {
                lower,
                upper,
                method_id,
            }) => {
                out.push(2);
                push_f64(&mut out, lower);
                push_f64(&mut out, upper);
                push_string(&mut out, &method_id);
            }
            None => out.push(0),
        }
    }

    let mut rows = receipt.row_coverage.clone();
    rows.sort_by_key(|row| row.family);
    push_u64(&mut out, rows.len() as u64);
    for row in rows {
        out.push(row.family as u8);
        for count in [
            row.capacity_rows,
            row.reserved_rows,
            row.requested_rows,
            row.admitted_rows,
            row.solved_rows,
            row.not_converged_rows,
            row.sleeping_skipped_rows,
            row.capacity_dropped_rows,
            row.unsupported_rows,
            row.execution_aborted_rows,
        ] {
            push_u64(&mut out, count);
        }
    }
    out
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// Canonicalize signed zero because +0.0 and -0.0 denote the same real value.
fn push_f64(out: &mut Vec<u8>, value: f64) {
    let canonical = if value == 0.0 { 0.0 } else { value };
    out.extend_from_slice(&canonical.to_bits().to_be_bytes());
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    push_u64(out, value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

fn push_optional_string(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            out.push(1);
            push_string(out, value);
        }
        None => out.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ExecutionIdentity {
        ExecutionIdentity {
            run_id: "run-001".into(),
            scenario_sha256: "1".repeat(64),
            engine_build_sha256: "2".repeat(64),
            solver_profile_sha256: "3".repeat(64),
            capability_census_sha256: "4".repeat(64),
            request_set_sha256: "5".repeat(64),
        }
    }

    fn contact_spec() -> ObservableSpec {
        ObservableSpec {
            key: ObservableKey {
                entity_id: "body-1".into(),
                observable_id: "contact.normal_impulse".into(),
            },
            requested: true,
            required: true,
            unit: "N*s".into(),
            frame_id: "world".into(),
            row_family: Some(ConstraintRowFamily::Contact),
        }
    }

    fn contact_rows() -> ConstraintRowCoverage {
        ConstraintRowCoverage {
            family: ConstraintRowFamily::Contact,
            capacity_rows: 8,
            reserved_rows: 1,
            requested_rows: 1,
            admitted_rows: 1,
            solved_rows: 1,
            not_converged_rows: 0,
            sleeping_skipped_rows: 0,
            capacity_dropped_rows: 0,
            unsupported_rows: 0,
            execution_aborted_rows: 0,
        }
    }

    fn produced_zero() -> ObservableResult {
        ObservableResult {
            key: contact_spec().key,
            disposition: ObservableDisposition::Produced,
            value: Some(0.0),
            producer_id: Some("contact-solver-v1".into()),
            uncertainty: Some(UncertaintySemantics::NotQuantified {
                reason: "synthetic exact-zero control; uncertainty not measured".into(),
            }),
            reason: None,
        }
    }

    fn receipt() -> CoverageReceipt {
        CoverageReceipt {
            request: CoverageRequest {
                identity: identity(),
                observables: vec![contact_spec()],
                required_row_families: vec![ConstraintRowFamily::Contact],
            },
            observable_results: vec![produced_zero()],
            row_coverage: vec![contact_rows()],
            capacity_evidence: capacity::CapacityEvidence::default(),
        }
    }

    #[test]
    fn computed_exact_zero_is_a_produced_measurement() {
        let report = validate_receipt(&receipt()).expect("valid exact-zero receipt");
        assert!(report.protocol_complete);
        assert_eq!(report.produced_observables, 1);
        assert_eq!(report.explicit_nonproduced_observables, 0);
        assert!(report.complete_for_declared_required_work);
    }

    #[test]
    fn capacity_dropped_zero_must_be_disposition_not_numeric_zero() {
        let mut value = receipt();
        value.observable_results[0] = ObservableResult {
            key: contact_spec().key,
            disposition: ObservableDisposition::CapacityDropped,
            value: None,
            producer_id: None,
            uncertainty: None,
            reason: Some("contact row exceeded configured capacity".into()),
        };
        value.row_coverage[0].reserved_rows = 0;
        value.row_coverage[0].requested_rows = 1;
        value.row_coverage[0].admitted_rows = 0;
        value.row_coverage[0].solved_rows = 0;
        value.row_coverage[0].capacity_dropped_rows = 1;
        let report = validate_receipt(&value).expect("explicit drop is valid evidence");
        assert!(report.protocol_complete);
        assert_eq!(report.capacity_dropped_rows, 1);
        assert!(!report.required_observables_produced);
        assert!(!report.complete_for_declared_required_work);
        assert!(report.row_attribution_complete);
    }

    #[test]
    fn all_dropped_rows_cannot_be_reported_as_a_produced_zero() {
        let mut value = receipt();
        value.row_coverage[0].reserved_rows = 0;
        value.row_coverage[0].admitted_rows = 0;
        value.row_coverage[0].solved_rows = 0;
        value.row_coverage[0].capacity_dropped_rows = 1;
        assert!(validate_receipt(&value).is_err());
    }

    #[test]
    fn aggregate_partial_omission_marks_row_attribution_incomplete() {
        let mut value = receipt();
        value.row_coverage[0].requested_rows = 2;
        value.row_coverage[0].capacity_dropped_rows = 1;
        let report = validate_receipt(&value).expect("aggregate omission is explicit");
        assert!(report.protocol_complete);
        assert!(!report.row_attribution_complete);
        assert!(!report.required_rows_solved);
        assert!(!report.complete_for_declared_required_work);
    }

    #[test]
    fn sleeping_skip_is_not_a_produced_zero() {
        let mut value = receipt();
        value.observable_results[0] = ObservableResult {
            key: contact_spec().key,
            disposition: ObservableDisposition::SleepingSkipped,
            value: None,
            producer_id: None,
            uncertainty: None,
            reason: Some("sleep policy skipped this island".into()),
        };
        value.row_coverage[0].reserved_rows = 0;
        value.row_coverage[0].admitted_rows = 0;
        value.row_coverage[0].solved_rows = 0;
        value.row_coverage[0].sleeping_skipped_rows = 1;
        let report = validate_receipt(&value).expect("explicit sleep skip is valid evidence");
        assert_eq!(report.sleeping_skipped_rows, 1);
        assert!(!report.complete_for_declared_required_work);
    }

    #[test]
    fn missing_or_duplicate_dispositions_fail_closed() {
        let mut missing = receipt();
        missing.observable_results.clear();
        assert!(validate_receipt(&missing).is_err());

        let mut duplicate = receipt();
        duplicate.observable_results.push(produced_zero());
        assert!(validate_receipt(&duplicate).is_err());
    }

    #[test]
    fn not_requested_is_explicit_and_excluded_from_coverage() {
        let mut value = receipt();
        value.request.observables[0].requested = false;
        value.request.observables[0].required = false;
        value.observable_results[0] = ObservableResult {
            key: contact_spec().key,
            disposition: ObservableDisposition::NotRequested,
            value: None,
            producer_id: None,
            uncertainty: None,
            reason: None,
        };
        let report = validate_receipt(&value).expect("not-requested catalog entry");
        assert_eq!(report.not_requested_observables, 1);
        assert_eq!(report.requested_observables, 0);
        assert_eq!(report.produced_observables, 0);
    }

    #[test]
    fn optional_unsupported_output_does_not_invalidate_required_work() {
        let mut value = receipt();
        let mut optional = contact_spec();
        optional.key.observable_id = "contact.torsional_moment".into();
        optional.required = false;
        value.request.observables.push(optional.clone());
        value.observable_results.push(ObservableResult {
            key: optional.key,
            disposition: ObservableDisposition::Unsupported,
            value: None,
            producer_id: Some("backend-steel-profile".into()),
            uncertainty: None,
            reason: Some("selected backend does not expose torsional moment".into()),
        });
        let report = validate_receipt(&value).expect("optional unsupported is explicit");
        assert_eq!(report.explicit_nonproduced_observables, 1);
        assert!(report.required_observables_produced);
        assert!(report.complete_for_declared_required_work);
    }

    #[test]
    fn count_accounting_and_capacity_are_fail_closed() {
        let mut value = receipt();
        value.row_coverage[0].capacity_rows = 0;
        assert!(validate_receipt(&value).is_err());

        let mut value = receipt();
        value.row_coverage[0].requested_rows = 2;
        assert!(validate_receipt(&value).is_err());
    }

    #[test]
    fn canonical_preimages_ignore_input_order() {
        let first = receipt();
        let mut second = first.clone();
        second.request.observables.reverse();
        second.request.required_row_families.reverse();
        second.observable_results.reverse();
        second.row_coverage.reverse();
        assert_eq!(
            first.request.canonical_request_preimage(),
            second.request.canonical_request_preimage()
        );
        assert_eq!(
            canonical_receipt_preimage(&first),
            canonical_receipt_preimage(&second)
        );
    }

    #[test]
    fn canonical_receipt_preimage_normalizes_signed_zero() {
        let first = receipt();
        let mut second = first.clone();
        second.observable_results[0].value = Some(-0.0);
        assert_eq!(
            canonical_receipt_preimage(&first),
            canonical_receipt_preimage(&second)
        );

        second.observable_results[0].uncertainty = Some(UncertaintySemantics::Interval {
            lower: -0.0,
            upper: 1.0,
            method_id: "interval-v1".into(),
        });
        let mut positive_zero = second.clone();
        positive_zero.observable_results[0].uncertainty = Some(UncertaintySemantics::Interval {
            lower: 0.0,
            upper: 1.0,
            method_id: "interval-v1".into(),
        });
        assert_eq!(
            canonical_receipt_preimage(&second),
            canonical_receipt_preimage(&positive_zero)
        );
    }

    #[test]
    fn caller_computed_request_digest_must_match_identity() {
        let value = receipt();
        let expected = value.request.identity.request_set_sha256.clone();
        assert!(validate_receipt_with_computed_request_digest(&value, &expected).is_ok());
        assert!(validate_receipt_with_computed_request_digest(&value, &"6".repeat(64)).is_err());
        assert!(validate_receipt_with_computed_request_digest(&value, "bad-digest").is_err());
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut value = receipt();
        value.request.identity.engine_build_sha256 = "not-a-digest".into();
        assert!(validate_receipt(&value).is_err());
    }

    #[test]
    fn capacity_lineage_is_part_of_the_canonical_run_receipt() {
        let mut first = receipt();
        let mut second = receipt();
        second
            .capacity_evidence
            .manifold_lineages
            .push(capacity::ContactManifoldLineage {
                pair_id: "body-a/body-b".into(),
                reduction_policy_id: None,
                point_capacity: Some(8),
                generated_points: vec![capacity::GeometricPointRecord {
                    point_id: "point-0".into(),
                    disposition: capacity::GeometricPointDisposition::Retained,
                    reason: None,
                }],
                solver_contacts: vec![capacity::SolverContactGroup {
                    solver_contact_id: "solver-contact-0".into(),
                    source_point_ids: vec!["point-0".into()],
                    disposition: capacity::SolverContactDisposition::Solved,
                    reason: None,
                }],
                solver_mapping_complete: true,
            });
        assert_ne!(
            canonical_receipt_preimage(&first),
            canonical_receipt_preimage(&second)
        );

        let report = validate_receipt(&second).expect("valid composed capacity evidence");
        assert_eq!(report.capacity_evidence.manifold_pairs, 1);
        assert_eq!(report.capacity_evidence.geometric_points_generated, 1);
        assert_eq!(report.capacity_evidence.solver_mapping_incomplete_pairs, 0);

        first.capacity_evidence = second.capacity_evidence.clone();
        assert_eq!(
            canonical_receipt_preimage(&first),
            canonical_receipt_preimage(&second)
        );
    }

    #[test]
    fn interval_uncertainty_must_be_finite_and_contain_value() {
        let mut value = receipt();
        value.observable_results[0].uncertainty = Some(UncertaintySemantics::Interval {
            lower: 1.0,
            upper: 2.0,
            method_id: "test-interval".into(),
        });
        assert!(validate_receipt(&value).is_err());
    }
}
