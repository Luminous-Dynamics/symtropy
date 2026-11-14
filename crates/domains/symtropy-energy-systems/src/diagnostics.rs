// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic Power Fabric diagnostics and compilation qualification reports.
//!
//! A completed report means only that this compilation pass emitted no blocking
//! diagnostics. It is not proof of electrical safety, conformity, installation
//! existence, physical isolation, or permission to energize equipment.

use crate::PowerFabricDocument;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DiagnosticCode {
    #[serde(rename = "PF-DIAG-0001")]
    RequiredClaimUnknown,
    #[serde(rename = "PF-DIAG-0002")]
    ContradictoryClaim,
    #[serde(rename = "PF-DIAG-0003")]
    EvidenceReferenceUnresolved,
    #[serde(rename = "PF-DIAG-0004")]
    StaleConfiguration,
    #[serde(rename = "PF-DIAG-0005")]
    MappingUnsupported,
    #[serde(rename = "PF-DIAG-0006")]
    IncompleteSourceMap,
    #[serde(rename = "PF-DIAG-0007")]
    ChangeImpactIncomplete,
    #[serde(rename = "PF-DIAG-0008")]
    ProjectionStale,
}

impl DiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequiredClaimUnknown => "PF-DIAG-0001",
            Self::ContradictoryClaim => "PF-DIAG-0002",
            Self::EvidenceReferenceUnresolved => "PF-DIAG-0003",
            Self::StaleConfiguration => "PF-DIAG-0004",
            Self::MappingUnsupported => "PF-DIAG-0005",
            Self::IncompleteSourceMap => "PF-DIAG-0006",
            Self::ChangeImpactIncomplete => "PF-DIAG-0007",
            Self::ProjectionStale => "PF-DIAG-0008",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilationEffect {
    BlockCompilation,
    Warn,
    Inform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum DiagnosticTarget {
    Document,
    Object { object_id: String },
    Relationship { relationship_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    diagnostic_id: String,
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    effect: CompilationEffect,
    target: DiagnosticTarget,
    source_pass: String,
    summary: String,
    remediation: String,
    related_object_ids: Vec<String>,
    evidence_refs: Vec<String>,
}

impl Diagnostic {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        diagnostic_id: impl Into<String>,
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        effect: CompilationEffect,
        target: DiagnosticTarget,
        source_pass: impl Into<String>,
        summary: impl Into<String>,
        remediation: impl Into<String>,
        mut related_object_ids: Vec<String>,
        mut evidence_refs: Vec<String>,
    ) -> Result<Self, DiagnosticError> {
        let diagnostic_id = diagnostic_id.into();
        let source_pass = source_pass.into();
        let summary = summary.into();
        let remediation = remediation.into();

        require_non_empty("diagnostic_id", &diagnostic_id)?;
        require_non_empty("source_pass", &source_pass)?;
        require_non_empty("summary", &summary)?;
        require_non_empty("remediation", &remediation)?;

        match (&target, effect, severity) {
            (DiagnosticTarget::Object { object_id }, _, _) => {
                require_non_empty("target object_id", object_id)?;
            }
            (DiagnosticTarget::Relationship { relationship_id }, _, _) => {
                require_non_empty("target relationship_id", relationship_id)?;
            }
            (DiagnosticTarget::Document, _, _) => {}
        }

        match severity {
            DiagnosticSeverity::Critical | DiagnosticSeverity::Error
                if effect != CompilationEffect::BlockCompilation =>
            {
                return Err(DiagnosticError::SeverityEffectMismatch { severity, effect });
            }
            DiagnosticSeverity::Warning if effect == CompilationEffect::Inform => {
                return Err(DiagnosticError::SeverityEffectMismatch { severity, effect });
            }
            DiagnosticSeverity::Info if effect != CompilationEffect::Inform => {
                return Err(DiagnosticError::SeverityEffectMismatch { severity, effect });
            }
            _ => {}
        }

        sort_unique_references("related_object_ids", &mut related_object_ids)?;
        sort_unique_references("evidence_refs", &mut evidence_refs)?;

        Ok(Self {
            diagnostic_id,
            code,
            severity,
            effect,
            target,
            source_pass,
            summary,
            remediation,
            related_object_ids,
            evidence_refs,
        })
    }

    pub fn diagnostic_id(&self) -> &str { &self.diagnostic_id }
    pub const fn code(&self) -> DiagnosticCode { self.code }
    pub const fn severity(&self) -> DiagnosticSeverity { self.severity }
    pub const fn effect(&self) -> CompilationEffect { self.effect }
    pub fn target(&self) -> &DiagnosticTarget { &self.target }
    pub fn source_pass(&self) -> &str { &self.source_pass }
    pub fn summary(&self) -> &str { &self.summary }
    pub fn remediation(&self) -> &str { &self.remediation }
    pub fn related_object_ids(&self) -> &[String] { &self.related_object_ids }
    pub fn evidence_refs(&self) -> &[String] { &self.evidence_refs }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilationStatus {
    CompletedWithoutFindings,
    CompletedWithInformation,
    CompletedWithWarnings,
    Blocked,
}

/// A report is bound to one exact IR document/revision and explicit compiler
/// and profile versions. Status is derived from diagnostics, never caller-set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticReport {
    report_id: String,
    document_id: String,
    revision_id: String,
    input_digest: String,
    compiler_version: String,
    profile_version: String,
    status: CompilationStatus,
    #[serde(serialize_with = "serialize_map_values")]
    diagnostics: BTreeMap<String, Diagnostic>,
}

impl DiagnosticReport {
    pub fn new(
        report_id: impl Into<String>,
        document: &PowerFabricDocument,
        input_digest: impl Into<String>,
        compiler_version: impl Into<String>,
        profile_version: impl Into<String>,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, DiagnosticError> {
        let report_id = report_id.into();
        let input_digest = input_digest.into();
        let compiler_version = compiler_version.into();
        let profile_version = profile_version.into();

        require_non_empty("report_id", &report_id)?;
        require_non_empty("input_digest", &input_digest)?;
        require_non_empty("compiler_version", &compiler_version)?;
        require_non_empty("profile_version", &profile_version)?;

        let mut by_id = BTreeMap::new();
        for diagnostic in diagnostics {
            validate_target(document, &diagnostic.target)?;
            for object_id in &diagnostic.related_object_ids {
                if !document.objects().contains_key(object_id) {
                    return Err(DiagnosticError::UnknownRelatedObject {
                        object_id: object_id.clone(),
                    });
                }
            }

            let id = diagnostic.diagnostic_id.clone();
            if by_id.insert(id.clone(), diagnostic).is_some() {
                return Err(DiagnosticError::DuplicateDiagnosticId { diagnostic_id: id });
            }
        }

        let status = derive_status(by_id.values());
        Ok(Self {
            report_id,
            document_id: document.document_id().to_owned(),
            revision_id: document.revision_id().to_owned(),
            input_digest,
            compiler_version,
            profile_version,
            status,
            diagnostics: by_id,
        })
    }

    pub fn report_id(&self) -> &str { &self.report_id }
    pub fn document_id(&self) -> &str { &self.document_id }
    pub fn revision_id(&self) -> &str { &self.revision_id }
    pub fn input_digest(&self) -> &str { &self.input_digest }
    pub fn compiler_version(&self) -> &str { &self.compiler_version }
    pub fn profile_version(&self) -> &str { &self.profile_version }
    pub const fn status(&self) -> CompilationStatus { self.status }
    pub fn diagnostics(&self) -> &BTreeMap<String, Diagnostic> { &self.diagnostics }

    pub fn by_code(&self, code: DiagnosticCode) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.values().filter(move |diagnostic| diagnostic.code == code)
    }
}

fn validate_target(
    document: &PowerFabricDocument,
    target: &DiagnosticTarget,
) -> Result<(), DiagnosticError> {
    match target {
        DiagnosticTarget::Document => Ok(()),
        DiagnosticTarget::Object { object_id } => {
            if document.objects().contains_key(object_id) {
                Ok(())
            } else {
                Err(DiagnosticError::UnknownTarget { target_id: object_id.clone() })
            }
        }
        DiagnosticTarget::Relationship { relationship_id } => {
            if document.relationships().contains_key(relationship_id) {
                Ok(())
            } else {
                Err(DiagnosticError::UnknownTarget { target_id: relationship_id.clone() })
            }
        }
    }
}

fn derive_status<'a>(
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
) -> CompilationStatus {
    let mut has_warning = false;
    let mut has_information = false;
    for diagnostic in diagnostics {
        match diagnostic.effect {
            CompilationEffect::BlockCompilation => return CompilationStatus::Blocked,
            CompilationEffect::Warn => has_warning = true,
            CompilationEffect::Inform => has_information = true,
        }
    }
    if has_warning {
        CompilationStatus::CompletedWithWarnings
    } else if has_information {
        CompilationStatus::CompletedWithInformation
    } else {
        CompilationStatus::CompletedWithoutFindings
    }
}

fn sort_unique_references(
    field: &'static str,
    values: &mut Vec<String>,
) -> Result<(), DiagnosticError> {
    for value in values.iter() {
        require_non_empty(field, value)?;
    }
    values.sort();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(DiagnosticError::DuplicateReference { field });
    }
    Ok(())
}

fn serialize_map_values<K, V, S>(
    values: &BTreeMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    K: Ord,
    V: Serialize,
    S: Serializer,
{
    values.values().collect::<Vec<_>>().serialize(serializer)
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), DiagnosticError> {
    if value.trim().is_empty() {
        return Err(DiagnosticError::EmptyField { field });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    EmptyField { field: &'static str },
    SeverityEffectMismatch { severity: DiagnosticSeverity, effect: CompilationEffect },
    DuplicateReference { field: &'static str },
    DuplicateDiagnosticId { diagnostic_id: String },
    UnknownTarget { target_id: String },
    UnknownRelatedObject { object_id: String },
}

impl core::fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DiagnosticError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ClaimDomain, ClaimProvenance, IrObject, IrSchemaVersion, NamespacedType,
        ObjectIdentity, PowerFabricDocument,
    };
    use std::collections::BTreeMap;

    fn document() -> PowerFabricDocument {
        let object = IrObject::new(
            ObjectIdentity::new("asset:one", BTreeMap::new()).unwrap(),
            NamespacedType::new("urn:power-fabric:test", "asset").unwrap(),
            Vec::new(),
        )
        .unwrap();
        PowerFabricDocument::new(
            IrSchemaVersion::current(),
            "document:test",
            "revision:test:1",
            vec![object],
            Vec::new(),
            Vec::new(),
        )
        .unwrap()
    }

    fn diagnostic(
        id: &str,
        severity: DiagnosticSeverity,
        effect: CompilationEffect,
    ) -> Diagnostic {
        Diagnostic::new(
            id,
            DiagnosticCode::RequiredClaimUnknown,
            severity,
            effect,
            DiagnosticTarget::Object { object_id: "asset:one".into() },
            "claim_qualification",
            "A required claim is unknown.",
            "Reconcile the claim using admissible evidence.",
            Vec::new(),
            Vec::new(),
        )
        .unwrap()
    }

    fn report(items: Vec<Diagnostic>) -> Result<DiagnosticReport, DiagnosticError> {
        DiagnosticReport::new(
            "report:test:1",
            &document(),
            "sha256:synthetic-input-digest",
            "power-fabric-compiler:0.2.0",
            "test-profile:0.1",
            items,
        )
    }

    #[test]
    fn blocking_diagnostic_derives_blocked_status() {
        let report = report(vec![diagnostic(
            "diag:001",
            DiagnosticSeverity::Critical,
            CompilationEffect::BlockCompilation,
        )]).unwrap();
        assert_eq!(report.status(), CompilationStatus::Blocked);
    }

    #[test]
    fn warning_diagnostic_derives_warning_status() {
        let report = report(vec![diagnostic(
            "diag:001",
            DiagnosticSeverity::Warning,
            CompilationEffect::Warn,
        )]).unwrap();
        assert_eq!(report.status(), CompilationStatus::CompletedWithWarnings);
    }

    #[test]
    fn informational_diagnostic_does_not_look_like_a_clean_report() {
        let report = report(vec![diagnostic(
            "diag:001",
            DiagnosticSeverity::Info,
            CompilationEffect::Inform,
        )]).unwrap();
        assert_eq!(report.status(), CompilationStatus::CompletedWithInformation);
    }

    #[test]
    fn empty_report_has_narrow_no_findings_status() {
        let report = report(Vec::new()).unwrap();
        assert_eq!(report.status(), CompilationStatus::CompletedWithoutFindings);
    }

    #[test]
    fn blocking_diagnostic_dominates_warning_and_information() {
        let report = report(vec![
            diagnostic("diag:003", DiagnosticSeverity::Info, CompilationEffect::Inform),
            diagnostic("diag:002", DiagnosticSeverity::Warning, CompilationEffect::Warn),
            diagnostic("diag:001", DiagnosticSeverity::Error, CompilationEffect::BlockCompilation),
        ]).unwrap();
        assert_eq!(report.status(), CompilationStatus::Blocked);
    }

    #[test]
    fn diagnostic_order_is_deterministic_by_id() {
        let report = report(vec![
            diagnostic("diag:z", DiagnosticSeverity::Warning, CompilationEffect::Warn),
            diagnostic("diag:a", DiagnosticSeverity::Warning, CompilationEffect::Warn),
        ]).unwrap();
        assert_eq!(
            report.diagnostics().keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["diag:a", "diag:z"]
        );
    }

    #[test]
    fn duplicate_diagnostic_identity_is_rejected() {
        assert!(matches!(
            report(vec![
                diagnostic("same", DiagnosticSeverity::Warning, CompilationEffect::Warn),
                diagnostic("same", DiagnosticSeverity::Warning, CompilationEffect::Warn),
            ]),
            Err(DiagnosticError::DuplicateDiagnosticId { .. })
        ));
    }

    #[test]
    fn unknown_target_is_rejected() {
        let invalid = Diagnostic::new(
            "diag:bad",
            DiagnosticCode::StaleConfiguration,
            DiagnosticSeverity::Error,
            CompilationEffect::BlockCompilation,
            DiagnosticTarget::Object { object_id: "asset:missing".into() },
            "identity_reconciliation",
            "The target object does not exist in this revision.",
            "Reconcile the object identity against the exact input revision.",
            Vec::new(),
            Vec::new(),
        ).unwrap();
        assert!(matches!(report(vec![invalid]), Err(DiagnosticError::UnknownTarget { .. })));
    }

    #[test]
    fn unknown_related_object_is_rejected() {
        let invalid = Diagnostic::new(
            "diag:related",
            DiagnosticCode::ChangeImpactIncomplete,
            DiagnosticSeverity::Warning,
            CompilationEffect::Warn,
            DiagnosticTarget::Document,
            "dependency_analysis",
            "A dependency references an object outside this revision.",
            "Reconcile the dependency inventory.",
            vec!["asset:missing".into()],
            Vec::new(),
        ).unwrap();
        assert!(matches!(report(vec![invalid]), Err(DiagnosticError::UnknownRelatedObject { .. })));
    }

    #[test]
    fn critical_and_error_severity_must_block_compilation() {
        assert!(matches!(
            Diagnostic::new(
                "diag:1", DiagnosticCode::RequiredClaimUnknown, DiagnosticSeverity::Critical,
                CompilationEffect::Warn, DiagnosticTarget::Document, "pass", "summary", "remediation",
                Vec::new(), Vec::new()
            ),
            Err(DiagnosticError::SeverityEffectMismatch { .. })
        ));
    }

    #[test]
    fn informational_severity_must_be_informational() {
        assert!(matches!(
            Diagnostic::new(
                "diag:1", DiagnosticCode::RequiredClaimUnknown, DiagnosticSeverity::Info,
                CompilationEffect::Warn, DiagnosticTarget::Document, "pass", "summary", "remediation",
                Vec::new(), Vec::new()
            ),
            Err(DiagnosticError::SeverityEffectMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_related_ids_and_evidence_refs_are_rejected() {
        let related = Diagnostic::new(
            "diag:1", DiagnosticCode::RequiredClaimUnknown, DiagnosticSeverity::Warning,
            CompilationEffect::Warn, DiagnosticTarget::Document, "pass", "summary", "remediation",
            vec!["asset:one".into(), "asset:one".into()], Vec::new()
        );
        assert!(matches!(related, Err(DiagnosticError::DuplicateReference { .. })));

        let evidence = Diagnostic::new(
            "diag:2", DiagnosticCode::EvidenceReferenceUnresolved, DiagnosticSeverity::Warning,
            CompilationEffect::Warn, DiagnosticTarget::Document, "pass", "summary", "remediation",
            Vec::new(), vec!["evidence:1".into(), "evidence:1".into()]
        );
        assert!(matches!(evidence, Err(DiagnosticError::DuplicateReference { .. })));
    }

    #[test]
    fn report_is_bound_to_exact_document_revision_and_compiler_profile() {
        let report = report(Vec::new()).unwrap();
        assert_eq!(report.document_id(), "document:test");
        assert_eq!(report.revision_id(), "revision:test:1");
        assert_eq!(report.compiler_version(), "power-fabric-compiler:0.2.0");
        assert_eq!(report.profile_version(), "test-profile:0.1");
    }

    #[test]
    fn relationship_target_must_exist() {
        let invalid = Diagnostic::new(
            "diag:rel", DiagnosticCode::StaleConfiguration, DiagnosticSeverity::Error,
            CompilationEffect::BlockCompilation,
            DiagnosticTarget::Relationship { relationship_id: "rel:missing".into() },
            "topology_check", "Relationship missing.", "Reconcile the relationship.",
            Vec::new(), Vec::new()
        ).unwrap();
        assert!(matches!(report(vec![invalid]), Err(DiagnosticError::UnknownTarget { .. })));
    }

    #[test]
    fn related_and_evidence_references_are_sorted() {
        let item = Diagnostic::new(
            "diag:sort", DiagnosticCode::RequiredClaimUnknown, DiagnosticSeverity::Warning,
            CompilationEffect::Warn, DiagnosticTarget::Document, "pass", "summary", "remediation",
            vec!["asset:z".into(), "asset:a".into()],
            vec!["evidence:z".into(), "evidence:a".into()]
        ).unwrap();
        assert_eq!(item.related_object_ids(), &["asset:a".to_owned(), "asset:z".to_owned()]);
        assert_eq!(item.evidence_refs(), &["evidence:a".to_owned(), "evidence:z".to_owned()]);
    }
}
