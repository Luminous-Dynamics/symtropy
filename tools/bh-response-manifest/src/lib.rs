//! Qualification contract for causal black-hole response experiments.
//! This validates evidence metadata; it does not solve or authenticate physics.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "0.2.0";
pub const INPUT_SET_CANONICALIZATION: &str = "utf8_bytewise_name_tab_lowercase_sha256_lf_v1";
pub const REQUIRED_NEGATIVE_CONTROLS: [&str; 12] = [
    "wrong_fourier_sign",
    "swapped_boundary_conditions",
    "wrong_wronskian_orientation",
    "wrong_matsubara_spacing",
    "qnm_sum_outside_convergence_region",
    "matsubara_sum_outside_convergence_region",
    "omitted_branch_cut_or_direct_term",
    "pre_source_response",
    "source_change_mislabeled_as_geometry",
    "unmatched_state_scaling",
    "source_overlap_change_must_not_move_poles",
    "precision_loss_near_cancellation",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Qualification {
    Unresolved,
    AnalyticOracle,
    NumericallyReproduced,
    IndependentlyCrossChecked,
    Converged,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyIdentity {
    pub study_id: String,
    pub model_id: String,
    pub equation_id: String,
    pub convention_id: String,
    pub coordinate_system: String,
    pub source_id: String,
    pub observer_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleReference {
    pub reference_id: String,
    pub version: String,
    pub locator: String,
    pub artifact_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MethodEvidence {
    pub method_id: String,
    pub implementation_id: String,
    pub implementation_sha256: String,
    /// Digest of the canonical, ordered input bundle actually consumed by this method.
    pub input_set_sha256: String,
    pub response_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedDigest {
    pub name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub git_commit: String,
    pub dependency_lock_sha256: String,
    /// Versioned byte-level canonicalization rule for the input bundle.
    pub canonical_input_set_format: String,
    /// SHA-256 of the canonical input bundle; null only when no inputs are declared.
    pub canonical_input_set_sha256: Option<String>,
    pub input_digests: Vec<NamedDigest>,
    pub reproduction_command: String,
    pub environment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericalSettings {
    pub solver_id: String,
    pub arithmetic: String,
    pub precision_bits: u32,
    pub parameters: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorDisposition {
    Measured,
    Bounded,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorBudgetComponent {
    pub component_id: String,
    pub disposition: ErrorDisposition,
    pub bound: Option<f64>,
    pub units: String,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseWindow {
    pub window_id: String,
    pub causal_region: String,
    pub norm_id: String,
    pub observed_error: f64,
    pub acceptance_limit: f64,
}

/// A paired, same-input comparison between two independently implemented methods.
///
/// The alignment digest identifies the canonical sample/time/frequency grid and
/// weighting used for the comparison. The comparison artifact digest binds the
/// actual aligned values and metric computation. The validator checks identities
/// and internal consistency; it does not recompute either digest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonEvidence {
    pub comparison_id: String,
    pub window_id: String,
    pub left_method_id: String,
    pub right_method_id: String,
    /// Exact method response artifacts consumed by this comparison.
    pub left_response_sha256: String,
    pub right_response_sha256: String,
    /// Digest of the versioned alignment procedure and its configuration.
    pub alignment_definition_sha256: String,
    /// Digest of the versioned norm/metric definition and its configuration.
    pub metric_definition_sha256: String,
    /// Digest of the actual aligned samples/weights used by the comparison.
    pub aligned_samples_sha256: String,
    /// Digest of aligned values, comparison output and per-window result artifact.
    pub comparison_artifact_sha256: String,
    pub norm_id: String,
    pub observed_error: f64,
    pub acceptance_limit: f64,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefinementPoint {
    pub resolution: u64,
    pub error_bound: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConvergenceSeries {
    pub quantity_id: String,
    pub norm_id: String,
    pub acceptance_limit: f64,
    pub refinements: Vec<RefinementPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NegativeControl {
    pub control_id: String,
    pub expected_failure_category: String,
    pub observed_failure_category: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseManifest {
    pub schema_version: String,
    pub study: StudyIdentity,
    pub qualification: Qualification,
    pub oracle_reference: Option<OracleReference>,
    pub provenance: Provenance,
    pub numerical: NumericalSettings,
    pub methods: Vec<MethodEvidence>,
    pub error_budget: Vec<ErrorBudgetComponent>,
    pub response_windows: Vec<ResponseWindow>,
    pub comparisons: Vec<ComparisonEvidence>,
    pub convergence: Vec<ConvergenceSeries>,
    pub negative_controls: Vec<NegativeControl>,
}

#[derive(Debug)]
pub enum ManifestError {
    Json(serde_json::Error),
    Invalid(Vec<String>),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid manifest JSON: {error}"),
            Self::Invalid(errors) => write!(f, "manifest failed validation: {}", errors.join("; ")),
        }
    }
}
impl std::error::Error for ManifestError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Compute the deterministic v1 digest of a named input set.
/// Entries are sorted by UTF-8 bytewise name order. Each line is
/// name<TAB>lowercase_sha256<LF>.
pub fn canonical_input_set_sha256(input_digests: &[NamedDigest]) -> String {
    let mut entries: Vec<&NamedDigest> = input_digests.iter().collect();
    entries.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
    let mut canonical = Vec::new();
    for entry in entries {
        canonical.extend_from_slice(entry.name.as_bytes());
        canonical.push(b'\t');
        canonical.extend_from_slice(entry.sha256.to_ascii_lowercase().as_bytes());
        canonical.push(b'\n');
    }
    sha256_hex(&canonical)
}
fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}
fn hex(value: &str, lengths: &[usize]) -> bool {
    lengths.contains(&value.len()) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn unique<'a>(values: impl IntoIterator<Item = &'a str>, name: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !non_empty(value) {
            errors.push(format!("{name}: empty identifier"));
        } else if !seen.insert(value) {
            errors.push(format!("{name}: duplicate '{value}'"));
        }
    }
}

impl ResponseManifest {
    /// Validate format, cross-field relationships and qualification gates.
    /// A successful validation is not a solver reproduction or physical proof.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut e = Vec::new();
        if self.schema_version != SCHEMA_VERSION {
            e.push(format!("schema_version must be {SCHEMA_VERSION}"));
        }
        for (k, v) in [
            ("study_id", self.study.study_id.as_str()),
            ("model_id", self.study.model_id.as_str()),
            ("equation_id", self.study.equation_id.as_str()),
            ("convention_id", self.study.convention_id.as_str()),
            ("coordinate_system", self.study.coordinate_system.as_str()),
            ("source_id", self.study.source_id.as_str()),
            ("observer_id", self.study.observer_id.as_str()),
            ("solver_id", self.numerical.solver_id.as_str()),
            ("arithmetic", self.numerical.arithmetic.as_str()),
            (
                "reproduction_command",
                self.provenance.reproduction_command.as_str(),
            ),
            ("environment", self.provenance.environment.as_str()),
        ] {
            if !non_empty(v) {
                e.push(format!("{k} must be non-empty"));
            }
        }
        if !hex(&self.provenance.git_commit, &[40, 64]) {
            e.push("git_commit must be 40/64 hex characters".into());
        }
        if !hex(&self.provenance.dependency_lock_sha256, &[64]) {
            e.push("dependency_lock_sha256 must be SHA-256".into());
        }
        if self.provenance.canonical_input_set_format != INPUT_SET_CANONICALIZATION {
            e.push(format!("canonical_input_set_format must be {INPUT_SET_CANONICALIZATION}"));
        }
        if self.numerical.precision_bits == 0 {
            e.push("precision_bits must be positive".into());
        }
        for (k, v) in &self.numerical.parameters {
            if !non_empty(k) || !v.is_finite() {
                e.push(format!("invalid numerical parameter '{k}'"));
            }
        }

        unique(
            self.provenance
                .input_digests
                .iter()
                .map(|x| x.name.as_str()),
            "input_digests",
            &mut e,
        );
        for x in &self.provenance.input_digests {
            if !hex(&x.sha256, &[64]) {
                e.push(format!("input '{}' has invalid SHA-256", x.name));
            }
            if x.name.contains('\t') || x.name.contains('\r') || x.name.contains('\n') || x.name.contains('\0') {
                e.push(format!("input '{}' name contains a forbidden canonicalization delimiter", x.name));
            }
        }
        match (self.provenance.input_digests.is_empty(), self.provenance.canonical_input_set_sha256.as_deref()) {
            (true, None) => {}
            (true, Some(_)) => e.push("canonical_input_set_sha256 must be null when no named inputs are declared".into()),
            (false, None) => e.push("canonical_input_set_sha256 is required when named inputs are declared".into()),
            (false, Some(value)) => {
                if !hex(value, &[64]) {
                    e.push("canonical_input_set_sha256 must be SHA-256".into());
                } else if value.to_ascii_lowercase() != canonical_input_set_sha256(&self.provenance.input_digests) {
                    e.push("canonical_input_set_sha256 does not match the declared input_digests".into());
                }
            }
        }
        unique(
            self.methods.iter().map(|x| x.method_id.as_str()),
            "method_id",
            &mut e,
        );
        for x in &self.methods {
            if self.provenance.canonical_input_set_sha256.as_deref().is_some_and(|expected| x.input_set_sha256.to_ascii_lowercase() != expected.to_ascii_lowercase()) {
                e.push(format!(
                    "method '{}' input-set digest differs from manifest canonical input set",
                    x.method_id
                ));
            }
            if !non_empty(&x.implementation_id) {
                e.push(format!(
                    "method '{}' has empty implementation_id",
                    x.method_id
                ));
            }
            if !hex(&x.implementation_sha256, &[64]) {
                e.push(format!(
                    "method '{}' implementation digest invalid",
                    x.method_id
                ));
            }
            if !hex(&x.input_set_sha256, &[64]) {
                e.push(format!("method '{}' input-set digest invalid", x.method_id));
            }
            if !hex(&x.response_sha256, &[64]) {
                e.push(format!("method '{}' response digest invalid", x.method_id));
            }
        }
        if let Some(x) = &self.oracle_reference {
            if !non_empty(&x.reference_id) || !non_empty(&x.version) || !non_empty(&x.locator) {
                e.push("oracle reference identity is incomplete".into());
            }
            if x.artifact_sha256.as_deref().is_some_and(|v| !hex(v, &[64])) {
                e.push("oracle artifact digest invalid".into());
            }
        }

        unique(
            self.error_budget.iter().map(|x| x.component_id.as_str()),
            "error component",
            &mut e,
        );
        for x in &self.error_budget {
            if !non_empty(&x.units) {
                e.push(format!("error '{}' units missing", x.component_id));
            }
            match x.disposition {
                ErrorDisposition::Measured | ErrorDisposition::Bounded => {
                    if !x.bound.is_some_and(|v| v.is_finite() && v >= 0.0) {
                        e.push(format!(
                            "error '{}' needs finite non-negative bound",
                            x.component_id
                        ));
                    }
                }
                ErrorDisposition::Unknown | ErrorDisposition::NotApplicable => {
                    if x.bound.is_some() {
                        e.push(format!(
                            "error '{}' cannot carry bound for {:?}",
                            x.component_id, x.disposition
                        ));
                    }
                    if !x.rationale.as_deref().is_some_and(non_empty) {
                        e.push(format!(
                            "error '{}' requires disposition rationale",
                            x.component_id
                        ));
                    }
                }
            }
        }

        unique(
            self.response_windows.iter().map(|x| x.window_id.as_str()),
            "response window",
            &mut e,
        );
        for x in &self.response_windows {
            if !non_empty(&x.causal_region) || !non_empty(&x.norm_id) {
                e.push(format!("window '{}' lacks region/norm", x.window_id));
            }
            if !x.observed_error.is_finite()
                || x.observed_error < 0.0
                || !x.acceptance_limit.is_finite()
                || x.acceptance_limit < 0.0
            {
                e.push(format!("window '{}' has invalid errors", x.window_id));
            } else if self.qualification == Qualification::Converged
                && x.observed_error > x.acceptance_limit
            {
                e.push(format!("window '{}' exceeds acceptance", x.window_id));
            }
        }

        unique(
            self.comparisons.iter().map(|x| x.comparison_id.as_str()),
            "comparison",
            &mut e,
        );
        let method_by_id = |id: &str| self.methods.iter().find(|m| m.method_id == id);
        let window_by_id = |id: &str| self.response_windows.iter().find(|w| w.window_id == id);
        unique(
            self.comparisons.iter().map(|x| x.window_id.as_str()),
            "comparison window",
            &mut e,
        );
        for comparison in &self.comparisons {
            if !non_empty(&comparison.norm_id) {
                e.push(format!("comparison '{}' norm is missing", comparison.comparison_id));
            }
            if !hex(&comparison.left_response_sha256, &[64]) {
                e.push(format!("comparison '{}' left-response digest invalid", comparison.comparison_id));
            }
            if !hex(&comparison.right_response_sha256, &[64]) {
                e.push(format!("comparison '{}' right-response digest invalid", comparison.comparison_id));
            }
            if !hex(&comparison.alignment_definition_sha256, &[64]) {
                e.push(format!("comparison '{}' alignment-definition digest invalid", comparison.comparison_id));
            }
            if !hex(&comparison.metric_definition_sha256, &[64]) {
                e.push(format!("comparison '{}' metric-definition digest invalid", comparison.comparison_id));
            }
            if !hex(&comparison.aligned_samples_sha256, &[64]) {
                e.push(format!("comparison '{}' aligned-sample digest invalid", comparison.comparison_id));
            }
            if !hex(&comparison.comparison_artifact_sha256, &[64]) {
                e.push(format!("comparison '{}' artifact digest invalid", comparison.comparison_id));
            }
            if !comparison.observed_error.is_finite()
                || comparison.observed_error < 0.0
                || !comparison.acceptance_limit.is_finite()
                || comparison.acceptance_limit < 0.0
            {
                e.push(format!("comparison '{}' has invalid error/limit", comparison.comparison_id));
            } else if comparison.accepted != (comparison.observed_error <= comparison.acceptance_limit) {
                e.push(format!("comparison '{}' accepted flag disagrees with error/limit", comparison.comparison_id));
            }
            if comparison.left_method_id == comparison.right_method_id {
                e.push(format!("comparison '{}' must name two different methods", comparison.comparison_id));
            }
            match (method_by_id(&comparison.left_method_id), method_by_id(&comparison.right_method_id)) {
                (Some(left), Some(right)) => {
                    if left.implementation_sha256 == right.implementation_sha256 {
                        e.push(format!("comparison '{}' methods do not have distinct implementation digests", comparison.comparison_id));
                    }
                    if left.input_set_sha256 != right.input_set_sha256 {
                        e.push(format!("comparison '{}' methods did not consume the same canonical input set", comparison.comparison_id));
                    }
                    if left.response_sha256.to_ascii_lowercase() != comparison.left_response_sha256.to_ascii_lowercase() {
                        e.push(format!("comparison '{}' left response digest is stale for method '{}'", comparison.comparison_id, left.method_id));
                    }
                    if right.response_sha256.to_ascii_lowercase() != comparison.right_response_sha256.to_ascii_lowercase() {
                        e.push(format!("comparison '{}' right response digest is stale for method '{}'", comparison.comparison_id, right.method_id));
                    }
                }
                _ => e.push(format!("comparison '{}' references an unknown method", comparison.comparison_id)),
            }
            match window_by_id(&comparison.window_id) {
                Some(window) => {
                    if comparison.norm_id != window.norm_id {
                        e.push(format!("comparison '{}' norm differs from response window", comparison.comparison_id));
                    }
                    if comparison.observed_error != window.observed_error
                        || comparison.acceptance_limit != window.acceptance_limit
                    {
                        e.push(format!("comparison '{}' error/limit differs from response window summary", comparison.comparison_id));
                    }
                }
                None => e.push(format!("comparison '{}' references an unknown response window", comparison.comparison_id)),
            }
        }

        unique(
            self.convergence.iter().map(|x| x.quantity_id.as_str()),
            "convergence quantity",
            &mut e,
        );
        for x in &self.convergence {
            if !non_empty(&x.norm_id) || !x.acceptance_limit.is_finite() || x.acceptance_limit < 0.0
            {
                e.push(format!(
                    "convergence '{}' norm/limit invalid",
                    x.quantity_id
                ));
            }
            let (mut last_n, mut last_err) = (None, None);
            for p in &x.refinements {
                if p.resolution == 0 || !p.error_bound.is_finite() || p.error_bound < 0.0 {
                    e.push(format!("convergence '{}' point invalid", x.quantity_id));
                    continue;
                }
                if last_n.is_some_and(|n| p.resolution <= n) {
                    e.push(format!(
                        "convergence '{}' resolution must increase",
                        x.quantity_id
                    ));
                }
                if last_err.is_some_and(|v| p.error_bound > v) {
                    e.push(format!(
                        "convergence '{}' error must be non-increasing",
                        x.quantity_id
                    ));
                }
                last_n = Some(p.resolution);
                last_err = Some(p.error_bound);
            }
            if self.qualification == Qualification::Converged {
                if x.refinements.len() < 3 {
                    e.push(format!(
                        "convergence '{}' needs >=3 refinements",
                        x.quantity_id
                    ));
                }
                if x.refinements
                    .last()
                    .is_none_or(|p| p.error_bound > x.acceptance_limit)
                {
                    e.push(format!("convergence '{}' misses acceptance", x.quantity_id));
                }
            }
        }

        unique(
            self.negative_controls.iter().map(|x| x.control_id.as_str()),
            "negative control",
            &mut e,
        );
        for x in &self.negative_controls {
            if !non_empty(&x.expected_failure_category) || !non_empty(&x.observed_failure_category)
            {
                e.push(format!(
                    "control '{}' failure categories missing",
                    x.control_id
                ));
            }
            if x.passed != (x.expected_failure_category == x.observed_failure_category) {
                e.push(format!(
                    "control '{}' pass flag disagrees with categories",
                    x.control_id
                ));
            }
        }
        self.validate_qualification(&mut e);
        if e.is_empty() { Ok(()) } else { Err(e) }
    }

    fn validate_qualification(&self, e: &mut Vec<String>) {
        match self.qualification {
            Qualification::Unresolved => {}
            Qualification::AnalyticOracle => {
                if self.oracle_reference.is_none() {
                    e.push("analytic_oracle requires pinned oracle".into());
                }
            }
            Qualification::NumericallyReproduced => {
                self.require_oracle_and_input(e);
                if self.methods.is_empty() {
                    e.push("numerically_reproduced requires a method".into());
                }
            }
            Qualification::IndependentlyCrossChecked | Qualification::Converged => {
                self.require_oracle_and_input(e);
                if self.response_windows.is_empty() {
                    e.push("cross-check requires at least one named response window".into());
                }
                if self.methods.len() < 2 {
                    e.push("cross-check requires >=2 methods".into());
                }
                let distinct: BTreeSet<&str> = self
                    .methods
                    .iter()
                    .map(|x| x.implementation_sha256.as_str())
                    .collect();
                if distinct.len() < 2 {
                    e.push("cross-check requires distinct implementation digests".into());
                }

                for window in &self.response_windows {
                    match self.comparisons.iter().find(|comparison| comparison.window_id == window.window_id) {
                        None => e.push(format!("cross-check missing paired comparison for window '{}'", window.window_id)),
                        Some(comparison) if !comparison.accepted => {
                            e.push(format!("cross-check comparison for window '{}' is outside its declared limit", window.window_id));
                        }
                        _ => {}
                    }
                }
                for method in &self.methods {
                    if !self.comparisons.iter().any(|comparison| {
                        comparison.accepted
                            && (comparison.left_method_id == method.method_id
                                || comparison.right_method_id == method.method_id)
                    }) {
                        e.push(format!(
                            "cross-check method '{}' does not participate in any accepted paired comparison",
                            method.method_id
                        ));
                    }
                }
                // A cross-check over several representations must be one
                // connected comparison graph. Separate, disconnected pairs
                // cannot qualify the entire declared method ensemble.
                let mut connected: BTreeSet<&str> = BTreeSet::new();
                if let Some(first) = self.methods.first() {
                    connected.insert(first.method_id.as_str());
                }
                loop {
                    let mut changed = false;
                    for comparison in self.comparisons.iter().filter(|x| x.accepted) {
                        let left = comparison.left_method_id.as_str();
                        let right = comparison.right_method_id.as_str();
                        if connected.contains(left) && connected.insert(right) {
                            changed = true;
                        }
                        if connected.contains(right) && connected.insert(left) {
                            changed = true;
                        }
                    }
                    if !changed {
                        break;
                    }
                }
                if connected.len() != self.methods.len() {
                    let unreachable: Vec<&str> = self.methods.iter()
                        .map(|method| method.method_id.as_str())
                        .filter(|id| !connected.contains(id))
                        .collect();
                    e.push(format!(
                        "cross-check comparison graph is disconnected; methods unreachable from the first method: {}",
                        unreachable.join(", ")
                    ));
                }
            }
        }
        if self.qualification == Qualification::Converged {
            if self.response_windows.is_empty() {
                e.push("converged requires response windows".into());
            }
            if self.convergence.is_empty() {
                e.push("converged requires refinements".into());
            }
            const REQUIRED_ERRORS: [&str; 7] = [
                "formula_transcription",
                "frequency_root_residue",
                "time_domain_discretization",
                "spectral_truncation",
                "contour_quadrature",
                "finite_precision",
                "source_observer_sampling",
            ];
            for id in REQUIRED_ERRORS {
                match self.error_budget.iter().find(|x| x.component_id == id) {
                    None => e.push(format!("converged error budget missing '{id}'")),
                    Some(x) if x.disposition == ErrorDisposition::Unknown => {
                        e.push(format!("converged error '{id}' remains unknown"))
                    }
                    _ => {}
                }
            }
            for id in REQUIRED_NEGATIVE_CONTROLS {
                match self.negative_controls.iter().find(|x| x.control_id == id) {
                    None => e.push(format!("converged missing negative control '{id}'")),
                    Some(x) if !x.passed => {
                        e.push(format!("converged negative control '{id}' failed"))
                    }
                    _ => {}
                }
            }
        }
    }

    fn require_oracle_and_input(&self, e: &mut Vec<String>) {
        if self.oracle_reference.is_none() {
            e.push("numerical result requires pinned oracle".into());
        }
        if self.provenance.input_digests.is_empty() || self.provenance.canonical_input_set_sha256.is_none() {
            e.push("numerical result requires named inputs and a canonical input-set digest".into());
        }
    }
}

pub fn parse_and_validate_json(input: &str) -> Result<ResponseManifest, ManifestError> {
    let m: ResponseManifest = serde_json::from_str(input).map_err(ManifestError::Json)?;
    m.validate().map_err(ManifestError::Invalid)?;
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(c: char) -> String {
        std::iter::repeat_n(c, 64).collect()
    }
    fn base() -> ResponseManifest {
        ResponseManifest {
            schema_version: SCHEMA_VERSION.into(),
            study: StudyIdentity {
                study_id: "pt-response-v0".into(),
                model_id: "poschl-teller-1d".into(),
                equation_id: "wave-sech2-v1".into(),
                convention_id: "fourier-minus-iwt-v1".into(),
                coordinate_system: "x-over-b".into(),
                source_id: "pulse-01".into(),
                observer_id: "observer-right".into(),
            },
            qualification: Qualification::Unresolved,
            oracle_reference: None,
            provenance: Provenance {
                git_commit: "a".repeat(40),
                dependency_lock_sha256: d('b'),
                canonical_input_set_format: INPUT_SET_CANONICALIZATION.into(),
            canonical_input_set_sha256: None,
                input_digests: vec![],
                reproduction_command: "cargo test".into(),
                environment: "rust-1.96".into(),
            },
            numerical: NumericalSettings {
                solver_id: "reference-v0".into(),
                arithmetic: "binary64".into(),
                precision_bits: 53,
                parameters: [("dx".into(), 0.01), ("dt".into(), 0.005)]
                    .into_iter()
                    .collect(),
            },
            methods: vec![],
            error_budget: vec![],
            response_windows: vec![],
            comparisons: vec![],
            convergence: vec![],
            negative_controls: vec![],
        }
    }
    fn bind_canonical_input_set(m: &mut ResponseManifest) {
        m.provenance.canonical_input_set_sha256 =
            Some(canonical_input_set_sha256(&m.provenance.input_digests));
    }

    fn oracle(m: &mut ResponseManifest) {
        m.oracle_reference = Some(OracleReference {
            reference_id: "arxiv-2510.18956".into(),
            version: "published-2026-07-09".into(),
            locator: "https://doi.org/10.1103/1bcl-q79x".into(),
            artifact_sha256: None,
        });
    }
    fn method(id: &str, i: char, r: char) -> MethodEvidence {
        MethodEvidence {
            method_id: id.into(),
            implementation_id: format!("impl-{id}"),
            implementation_sha256: d(i),
            input_set_sha256: canonical_input_set_sha256(&[NamedDigest {
                name: "initial-data".into(),
                sha256: d('e'),
            }]),
            response_sha256: d(r),
        }
    }

    fn pair_comparison(window_id: &str, left: &str, right: &str, left_response: char, right_response: char, error: f64, limit: f64) -> ComparisonEvidence {
        ComparisonEvidence {
            comparison_id: format!("cmp-{window_id}"),
            window_id: window_id.into(),
            left_method_id: left.into(),
            right_method_id: right.into(),
            left_response_sha256: d(left_response),
            right_response_sha256: d(right_response),
            alignment_definition_sha256: d('b'),
            metric_definition_sha256: d('d'),
            aligned_samples_sha256: d('a'),
            comparison_artifact_sha256: d('c'),
            norm_id: "complex-l2-relative".into(),
            observed_error: error,
            acceptance_limit: limit,
            accepted: error <= limit,
        }
    }

    #[test]
    fn unresolved_is_valid_without_physical_claim() {
        assert!(base().validate().is_ok());
    }
    #[test]
    fn malformed_hashes_fail_closed() {
        let mut m = base();
        m.provenance.git_commit = "bad".into();
        m.provenance.dependency_lock_sha256 = "bad".into();
        let e = m.validate().unwrap_err();
        assert!(e.iter().any(|x| x.contains("git_commit")));
        assert!(e.iter().any(|x| x.contains("dependency_lock_sha256")));
    }
    #[test]
    fn numerical_reproduction_needs_oracle_and_input() {
        let mut m = base();
        m.qualification = Qualification::NumericallyReproduced;
        m.methods.push(method("fdtd", 'c', 'd'));
        let e = m.validate().unwrap_err();
        assert!(e.iter().any(|x| x.contains("pinned oracle")));
        assert!(e.iter().any(|x| x.contains("input digests")));
    }
    #[test]
    fn independent_cross_check_needs_distinct_implementations() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", 'f', '2'));
        assert!(
            m.validate()
                .unwrap_err()
                .iter()
                .any(|x| x.contains("distinct implementation"))
        );
    }
    #[test]
    fn converged_fixture_requires_and_accepts_full_evidence() {
        let mut m = base();
        m.qualification = Qualification::Converged;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
        m.comparisons.push(pair_comparison("late", "fd", "freq", '1', '2', 1e-9, 1e-8));
        m.convergence.push(ConvergenceSeries {
            quantity_id: "late-error".into(),
            norm_id: "complex-l2-relative".into(),
            acceptance_limit: 1e-8,
            refinements: vec![
                RefinementPoint {
                    resolution: 64,
                    error_bound: 1e-4,
                },
                RefinementPoint {
                    resolution: 128,
                    error_bound: 1e-6,
                },
                RefinementPoint {
                    resolution: 256,
                    error_bound: 1e-9,
                },
            ],
        });
        for id in [
            "formula_transcription",
            "frequency_root_residue",
            "time_domain_discretization",
            "spectral_truncation",
            "contour_quadrature",
            "finite_precision",
            "source_observer_sampling",
        ] {
            m.error_budget.push(ErrorBudgetComponent {
                component_id: id.into(),
                disposition: ErrorDisposition::Bounded,
                bound: Some(1e-10),
                units: "relative".into(),
                rationale: None,
            });
        }
        for id in REQUIRED_NEGATIVE_CONTROLS {
            m.negative_controls.push(NegativeControl {
                control_id: id.into(),
                expected_failure_category: "expected".into(),
                observed_failure_category: "expected".into(),
                passed: true,
            });
        }
        assert!(m.validate().is_ok());
    }

    #[test]
    fn independent_cross_check_requires_a_window_bound_paired_comparison() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.response_windows.push(ResponseWindow {
            window_id: "pre-front".into(),
            causal_region: "before-first-arrival".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 0.0,
            acceptance_limit: 1e-8,
        });
        let errors = m.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("missing paired comparison")));
    }

    #[test]
    fn paired_comparison_rejects_different_input_sets() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        let left = method("fd", 'f', '1');
        let mut right = method("freq", '0', '2');
        right.input_set_sha256 = d('d');
        m.methods.extend([left, right]);
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
        m.comparisons.push(pair_comparison("late", "fd", "freq", '1', '2', 1e-9, 1e-8));
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("same canonical input set")));
    }

    #[test]
    fn paired_comparison_rejects_wrong_window_norm_or_digest() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
        let mut comparison = pair_comparison("late", "fd", "freq", '1', '2', 1e-9, 1e-8);
        comparison.left_response_sha256 = "bad".into();
        comparison.right_response_sha256 = "bad".into();
        comparison.alignment_definition_sha256 = "bad".into();
        comparison.metric_definition_sha256 = "bad".into();
        comparison.aligned_samples_sha256 = "bad".into();
        comparison.norm_id = "wrong-norm".into();
        m.comparisons.push(comparison);
        let errors = m.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("left-response digest invalid")));
        assert!(errors.iter().any(|e| e.contains("right-response digest invalid")));
        assert!(errors.iter().any(|e| e.contains("alignment-definition digest invalid")));
        assert!(errors.iter().any(|e| e.contains("metric-definition digest invalid")));
        assert!(errors.iter().any(|e| e.contains("aligned-sample digest invalid")));
        assert!(errors.iter().any(|e| e.contains("norm differs")));
    }

    #[test]
    fn cross_check_requires_a_connected_method_comparison_graph() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.extend([
            method("fd", 'f', '1'),
            method("freq", '0', '2'),
            method("green-a", '3', '4'),
            method("green-b", '5', '6'),
        ]);
        for window_id in ["fd-freq", "green-a-green-b"] {
            m.response_windows.push(ResponseWindow {
                window_id: window_id.into(),
                causal_region: "late-time".into(),
                norm_id: "complex-l2-relative".into(),
                observed_error: 1e-9,
                acceptance_limit: 1e-8,
            });
        }
        m.comparisons.push(pair_comparison("fd-freq", "fd", "freq", '1', '2', 1e-9, 1e-8));
        m.comparisons.push(pair_comparison("green-a-green-b", "green-a", "green-b", '4', '6', 1e-9, 1e-8));
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("comparison graph is disconnected")));

        m.response_windows.push(ResponseWindow {
            window_id: "bridge".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
        m.comparisons.push(pair_comparison("bridge", "freq", "green-a", '2', '4', 1e-9, 1e-8));
        assert!(m.validate().is_ok());
    }

    #[test]
    fn cross_check_rejects_method_not_used_by_any_accepted_pair() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.methods.push(method("unused-third-method", '9', '8'));
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
        m.comparisons.push(pair_comparison("late", "fd", "freq", '1', '2', 1e-9, 1e-8));
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("does not participate")));
    }

    #[test]
    fn method_cannot_claim_an_input_set_not_bound_by_manifest_provenance() {
        let mut m = base();
        m.methods.push(method("fd", 'f', '1'));
        m.methods[0].input_set_sha256 = d('9');
        let errors = m.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("differs from manifest canonical input set")));
    }

    #[test]
    fn method_input_set_digest_must_be_sha256() {
        let mut m = base();
        m.methods.push(method("fd", 'f', '1'));
        m.methods[0].input_set_sha256 = "not-a-digest".into();
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("input-set digest invalid")));
    }

    #[test]
    fn paired_comparison_rejects_stale_method_response_digests() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
        let mut comparison = pair_comparison("late", "fd", "freq", '1', '2', 1e-9, 1e-8);
        comparison.left_response_sha256 = d('9');
        comparison.right_response_sha256 = d('8');
        m.comparisons.push(comparison);
        let errors = m.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("left response digest is stale")));
        assert!(errors.iter().any(|e| e.contains("right response digest is stale")));
    }

    #[test]
    fn paired_comparison_rejects_unaccepted_error() {
        let mut m = base();
        m.qualification = Qualification::IndependentlyCrossChecked;
        oracle(&mut m);
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-3,
            acceptance_limit: 1e-8,
        });
        m.comparisons.push(pair_comparison("late", "fd", "freq", '1', '2', 1e-3, 1e-8));
        let errors = m.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.contains("outside its declared limit")));
    }

    #[test]
    fn unknown_error_blocks_convergence() {
        let mut m = base();
        m.qualification = Qualification::Converged;
        m.error_budget.push(ErrorBudgetComponent {
            component_id: "formula_transcription".into(),
            disposition: ErrorDisposition::Unknown,
            bound: None,
            units: "relative".into(),
            rationale: Some("not yet checked".into()),
        });
        assert!(
            m.validate()
                .unwrap_err()
                .iter()
                .any(|x| x.contains("remains unknown"))
        );
    }
    #[test]
    fn nonmonotone_refinement_is_rejected() {
        let mut m = base();
        m.convergence.push(ConvergenceSeries {
            quantity_id: "q".into(),
            norm_id: "l2".into(),
            acceptance_limit: 1.0,
            refinements: vec![
                RefinementPoint {
                    resolution: 16,
                    error_bound: 0.1,
                },
                RefinementPoint {
                    resolution: 32,
                    error_bound: 0.2,
                },
            ],
        });
        assert!(
            m.validate()
                .unwrap_err()
                .iter()
                .any(|x| x.contains("non-increasing"))
        );
    }
    #[test]
    fn negative_control_category_must_match() {
        let mut m = base();
        m.negative_controls.push(NegativeControl {
            control_id: "wrong_fourier_sign".into(),
            expected_failure_category: "phase".into(),
            observed_failure_category: "no-failure".into(),
            passed: true,
        });
        assert!(
            m.validate()
                .unwrap_err()
                .iter()
                .any(|x| x.contains("pass flag disagrees"))
        );
    }
    #[test]
    fn parses_full_v0_2_paired_comparison_fixture() {
        let input = include_str!("../examples/paired-comparison-v0.2.json");
        let manifest = parse_and_validate_json(input).unwrap();
        assert_eq!(manifest.schema_version, "0.2.0");
        assert_eq!(manifest.methods.len(), 2);
        assert_eq!(manifest.response_windows.len(), 1);
        assert_eq!(manifest.comparisons.len(), 1);
        assert!(manifest.comparisons[0].accepted);
    }

    #[test]
    fn unknown_json_fields_are_rejected() {
        let mut v = serde_json::to_value(base()).unwrap();
        v["green_check"] = serde_json::Value::Bool(true);
        assert!(matches!(
            parse_and_validate_json(&serde_json::to_string(&v).unwrap()),
            Err(ManifestError::Json(_))
        ));
    }
    #[test]
    fn canonical_input_set_is_order_independent_and_lowercases_hashes() {
        let left = vec![
            NamedDigest { name: "b".into(), sha256: "B".repeat(64) },
            NamedDigest { name: "a".into(), sha256: "A".repeat(64) },
        ];
        let right = vec![
            NamedDigest { name: "a".into(), sha256: "a".repeat(64) },
            NamedDigest { name: "b".into(), sha256: "b".repeat(64) },
        ];
        assert_eq!(
            canonical_input_set_sha256(&left),
            canonical_input_set_sha256(&right)
        );
    }

    #[test]
    fn canonical_input_set_digest_is_recomputed_from_declared_inputs() {
        let mut m = base();
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        assert!(m.validate().is_ok());
        m.provenance.canonical_input_set_sha256 = Some(d('9'));
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("does not match the declared input_digests")));
    }

    #[test]
    fn canonical_input_set_requires_a_digest_when_inputs_are_declared() {
        let mut m = base();
        m.provenance.input_digests.push(NamedDigest {
            name: "initial-data".into(),
            sha256: d('e'),
        });
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("is required when named inputs are declared")));
    }

    #[test]
    fn canonical_input_set_must_be_null_when_no_inputs_are_declared() {
        let mut m = base();
        m.provenance.canonical_input_set_sha256 = Some(d('e'));
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("must be null when no named inputs are declared")));
    }

    #[test]
    fn canonical_input_names_reject_line_and_tab_delimiters() {
        let mut m = base();
        m.provenance.input_digests.push(NamedDigest {
            name: "unsafe\tname".into(),
            sha256: d('e'),
        });
        bind_canonical_input_set(&mut m);
        assert!(m.validate().unwrap_err().iter().any(|e| e.contains("forbidden canonicalization delimiter")));
    }

    #[test]
    fn sha256_helper_matches_empty_input() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
