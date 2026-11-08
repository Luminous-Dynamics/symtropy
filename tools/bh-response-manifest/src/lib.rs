//! Qualification contract for causal black-hole response experiments.
//! This validates evidence metadata; it does not solve or authenticate physics.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "0.1.0";
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
        }
        unique(
            self.methods.iter().map(|x| x.method_id.as_str()),
            "method_id",
            &mut e,
        );
        for x in &self.methods {
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
        if self.provenance.input_digests.is_empty() {
            e.push("numerical result requires named input digests".into());
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
            convergence: vec![],
            negative_controls: vec![],
        }
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
            response_sha256: d(r),
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
        m.methods.push(method("fd", 'f', '1'));
        m.methods.push(method("freq", '0', '2'));
        m.response_windows.push(ResponseWindow {
            window_id: "late".into(),
            causal_region: "late-time".into(),
            norm_id: "complex-l2-relative".into(),
            observed_error: 1e-9,
            acceptance_limit: 1e-8,
        });
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
    fn unknown_json_fields_are_rejected() {
        let mut v = serde_json::to_value(base()).unwrap();
        v["green_check"] = serde_json::Value::Bool(true);
        assert!(matches!(
            parse_and_validate_json(&serde_json::to_string(&v).unwrap()),
            Err(ManifestError::Json(_))
        ));
    }
    #[test]
    fn sha256_helper_matches_empty_input() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
