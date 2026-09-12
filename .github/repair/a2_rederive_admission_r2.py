#!/usr/bin/env python3
from pathlib import Path
import shutil
import sys


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


if len(sys.argv) != 2:
    raise SystemExit("usage: a2_rederive_admission_r2.py <legacy-a2-checkout>")

legacy = Path(sys.argv[1]).resolve()
crate = Path("crates/domains/symtropy-analysis-admission")
(crate / "src").mkdir(parents=True, exist_ok=True)
shutil.copyfile(legacy / crate / "Cargo.toml", crate / "Cargo.toml")
shutil.copyfile(legacy / crate / "src/lib.rs", crate / "src/lib.rs")

root = Path("Cargo.toml")
root_text = root.read_text()
root_text = replace_once(
    root_text,
    '    "crates/domains/symtropy-analysis-qualification",\n',
    '    "crates/domains/symtropy-analysis-qualification",\n'
    '    "crates/domains/symtropy-analysis-admission",\n',
    "workspace member",
)
root.write_text(root_text)

lib = crate / "src/lib.rs"
text = lib.read_text()

text = replace_once(
    text,
    '''use symtropy_analysis_contracts::{\n    AnalysisEvidence, AnalysisEvidenceRef, AnalysisRequest, ExactSemanticRef,\n};''',
    '''use symtropy_analysis_contracts::{\n    AnalysisEvidenceRef, ExactSemanticRef, ValidatedAnalysisEvidence,\n};''',
    "production A0 imports",
)

# Rust 1.96 strict-Clippy hygiene without changing ordering semantics.
text = replace_once(
    text,
    '''            if let Some(previous) = previous {\n                if previous >= &rule.facet_id {\n                    return if previous == &rule.facet_id {\n                        Err(AdmissionError::DuplicateFacetPolicy(rule.facet_id.clone()))\n                    } else {\n                        Err(AdmissionError::NonCanonicalOrder("policy.facet_rules"))\n                    };\n                }\n            }''',
    '''            if let Some(previous) = previous\n                && previous >= &rule.facet_id\n            {\n                return if previous == &rule.facet_id {\n                    Err(AdmissionError::DuplicateFacetPolicy(rule.facet_id.clone()))\n                } else {\n                    Err(AdmissionError::NonCanonicalOrder("policy.facet_rules"))\n                };\n            }''',
    "policy collapsible-if",
)

# Strengthen the public admission theorem with a compile-fail type-state sentinel.
admitted_doc = '''/// Deliberately **not** `Deserialize`: after persistence a consumer must replay\n/// admission from the exact request/evidence/profile/cut/policy/registry (or a\n/// future independently authenticated receipt boundary). Bytes shaped like a\n/// prior verdict do not regain authority merely by being restored.\n#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct AdmittedAnalysisQualification {'''
admitted_doc_new = '''/// Deliberately **not** `Deserialize`: after persistence a consumer must replay\n/// admission from replay-valid evidence plus the exact profile/cut/policy/registry\n/// (or a future independently authenticated receipt boundary). Bytes shaped like\n/// a prior verdict do not regain authority merely by being restored.\n///\n/// Raw/restored A0 evidence cannot enter A2 admission directly:\n///\n/// ```compile_fail\n/// use symtropy_analysis_contracts::AnalysisEvidence;\n/// use symtropy_analysis_admission::{\n///     admit_qualification, AdmissionReceiptId, QualificationAdmissionPolicy, VerificationRegistry,\n/// };\n/// use symtropy_analysis_qualification::{AnalysisQualificationCut, QualificationProfile};\n/// fn raw_evidence_cannot_admit(\n///     id: AdmissionReceiptId,\n///     evidence: &AnalysisEvidence,\n///     profile: &QualificationProfile,\n///     cut: &AnalysisQualificationCut,\n///     policy: &QualificationAdmissionPolicy,\n///     registry: &VerificationRegistry,\n/// ) {\n///     let _ = admit_qualification(id, evidence, profile, cut, policy, registry);\n/// }\n/// ```\n#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct AdmittedAnalysisQualification {'''
text = replace_once(text, admitted_doc, admitted_doc_new, "A2 compile-fail authority guard")

old_validate_current = '''    pub fn validate_current(\n        &self,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        cut: &AnalysisQualificationCut,\n        policy: &QualificationAdmissionPolicy,\n        registry: &VerificationRegistry,\n    ) -> Result<(), AdmissionError> {\n        let reproduced = build_admission(\n            self.id.clone(),\n            request,\n            evidence,\n            profile,\n            cut,\n            policy,\n            registry,\n        )?;'''
new_validate_current = '''    pub fn validate_current(\n        &self,\n        evidence: &ValidatedAnalysisEvidence<'_>,\n        profile: &QualificationProfile,\n        cut: &AnalysisQualificationCut,\n        policy: &QualificationAdmissionPolicy,\n        registry: &VerificationRegistry,\n    ) -> Result<(), AdmissionError> {\n        let reproduced = build_admission(\n            self.id.clone(),\n            evidence,\n            profile,\n            cut,\n            policy,\n            registry,\n        )?;'''
text = replace_once(text, old_validate_current, new_validate_current, "current admission validation")

old_admit = '''pub fn admit_qualification(\n    id: AdmissionReceiptId,\n    request: &AnalysisRequest,\n    evidence: &AnalysisEvidence,\n    profile: &QualificationProfile,\n    cut: &AnalysisQualificationCut,\n    policy: &QualificationAdmissionPolicy,\n    registry: &VerificationRegistry,\n) -> Result<AdmittedAnalysisQualification, AdmissionError> {\n    build_admission(id, request, evidence, profile, cut, policy, registry)\n}'''
new_admit = '''pub fn admit_qualification(\n    id: AdmissionReceiptId,\n    evidence: &ValidatedAnalysisEvidence<'_>,\n    profile: &QualificationProfile,\n    cut: &AnalysisQualificationCut,\n    policy: &QualificationAdmissionPolicy,\n    registry: &VerificationRegistry,\n) -> Result<AdmittedAnalysisQualification, AdmissionError> {\n    build_admission(id, evidence, profile, cut, policy, registry)\n}'''
text = replace_once(text, old_admit, new_admit, "public admission signature")

old_build_head = '''fn build_admission(\n    id: AdmissionReceiptId,\n    request: &AnalysisRequest,\n    evidence: &AnalysisEvidence,\n    profile: &QualificationProfile,\n    cut: &AnalysisQualificationCut,\n    policy: &QualificationAdmissionPolicy,\n    registry: &VerificationRegistry,\n) -> Result<AdmittedAnalysisQualification, AdmissionError> {'''
new_build_head = '''fn build_admission(\n    id: AdmissionReceiptId,\n    evidence: &ValidatedAnalysisEvidence<'_>,\n    profile: &QualificationProfile,\n    cut: &AnalysisQualificationCut,\n    policy: &QualificationAdmissionPolicy,\n    registry: &VerificationRegistry,\n) -> Result<AdmittedAnalysisQualification, AdmissionError> {'''
text = replace_once(text, old_build_head, new_build_head, "internal admission signature")

text = replace_once(
    text,
    '''    let decision = cut.decision(request, evidence, profile)?;''',
    '''    let decision = cut.decision(evidence, profile)?;''',
    "A1 decision replay binding",
)
text = replace_once(
    text,
    '''    let cut_digest = cut.content_digest(request, evidence, profile)?;\n    let evidence_ref = evidence.exact_ref(request).map_err(AdmissionError::Analysis)?;''',
    '''    let cut_digest = cut.content_digest(evidence, profile)?;\n    let evidence_ref = evidence.exact_ref().map_err(AdmissionError::Analysis)?;''',
    "A1 digest and A0 evidence ref binding",
)

# Tests retain raw fixtures only to exercise the replay boundary explicitly.
text = replace_once(
    text,
    '''    use symtropy_analysis_contracts::{\n        AnalysisEvidenceId, AnalysisObservation, AnalysisObservationId, AnalysisProfileId,\n        AnalysisProfileRef, AnalysisRequestId, AnalysisValue, NumericalDisposition,\n        ObservableRequest, ObservationClass, RunDisposition, SolverIdentity,\n    };''',
    '''    use symtropy_analysis_contracts::{\n        AnalysisEvidence, AnalysisEvidenceId, AnalysisObservation, AnalysisObservationId,\n        AnalysisProfileId, AnalysisProfileRef, AnalysisRequest, AnalysisRequestId, AnalysisValue,\n        NumericalDisposition, ObservableRequest, ObservationClass, RunDisposition, SolverIdentity,\n    };''',
    "test raw fixture imports",
)

old_cut_helper = '''    fn cut(\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        disposition: FacetDisposition,\n    ) -> AnalysisQualificationCut {\n        AnalysisQualificationCut::new(\n            QualificationCutId::new(id("qualification-cut:bracket")),\n            request,\n            evidence,\n            profile,\n            exact("authority:clock", "evaluation-context:1", "context"),\n            vec![\n                assessment(\n                    request,\n                    evidence,\n                    "facet:model",\n                    "authority:model-qualification",\n                    disposition,\n                ),\n                assessment(\n                    request,\n                    evidence,\n                    "facet:numerical",\n                    "authority:numerical-qualification",\n                    disposition,\n                ),\n            ],\n        )\n        .unwrap()\n    }'''
new_cut_helper = '''    fn cut(\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        disposition: FacetDisposition,\n    ) -> AnalysisQualificationCut {\n        let validated = evidence.rebind(request).unwrap();\n        AnalysisQualificationCut::new(\n            QualificationCutId::new(id("qualification-cut:bracket")),\n            &validated,\n            profile,\n            exact("authority:clock", "evaluation-context:1", "context"),\n            vec![\n                assessment(\n                    request,\n                    evidence,\n                    "facet:model",\n                    "authority:model-qualification",\n                    disposition,\n                ),\n                assessment(\n                    request,\n                    evidence,\n                    "facet:numerical",\n                    "authority:numerical-qualification",\n                    disposition,\n                ),\n            ],\n        )\n        .unwrap()\n    }'''
text = replace_once(text, old_cut_helper, new_cut_helper, "A1 cut test replay helper")

old_verification_header = '''    fn verification_records(\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        cut: &AnalysisQualificationCut,\n        policy: &QualificationAdmissionPolicy,\n        status: VerificationStatus,\n    ) -> Vec<FacetVerificationInput> {\n        let cut_digest = cut.content_digest(request, evidence, profile).unwrap();\n        let evidence_ref = evidence.exact_ref(request).unwrap();'''
new_verification_header = '''    fn verification_records(\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        cut: &AnalysisQualificationCut,\n        policy: &QualificationAdmissionPolicy,\n        status: VerificationStatus,\n    ) -> Vec<FacetVerificationInput> {\n        let validated = evidence.rebind(request).unwrap();\n        let cut_digest = cut.content_digest(&validated, profile).unwrap();\n        let evidence_ref = validated.exact_ref().unwrap();'''
text = replace_once(text, old_verification_header, new_verification_header, "verification test replay helper")

# Rewrite test-only admission calls through explicit replay helpers while leaving
# the production function name untouched.
test_marker = "#[cfg(test)]\nmod tests {"
head, tail = text.split(test_marker, 1)
tail = tail.replace("admit_qualification(", "admit_from_raw(")

helper_anchor = '''    fn admission_policy(profile: &QualificationProfile) -> QualificationAdmissionPolicy {'''
helper = '''    fn admit_from_raw(\n        id_value: AdmissionReceiptId,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        cut: &AnalysisQualificationCut,\n        policy: &QualificationAdmissionPolicy,\n        registry: &VerificationRegistry,\n    ) -> Result<AdmittedAnalysisQualification, AdmissionError> {\n        let validated = evidence.rebind(request).map_err(AdmissionError::Analysis)?;\n        admit_qualification(id_value, &validated, profile, cut, policy, registry)\n    }\n\n    fn validate_from_raw(\n        admission: &AdmittedAnalysisQualification,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        cut: &AnalysisQualificationCut,\n        policy: &QualificationAdmissionPolicy,\n        registry: &VerificationRegistry,\n    ) -> Result<(), AdmissionError> {\n        let validated = evidence.rebind(request).map_err(AdmissionError::Analysis)?;\n        admission.validate_current(&validated, profile, cut, policy, registry)\n    }\n\n'''
if helper_anchor not in tail:
    raise SystemExit("test replay helper anchor missing")
tail = tail.replace(helper_anchor, helper + helper_anchor, 1)

tail = replace_once(
    tail,
    '''        admission\n            .validate_current(&request, &evidence, &profile, &cut, &policy, &registry)\n            .unwrap();''',
    '''        validate_from_raw(\n            &admission, &request, &evidence, &profile, &cut, &policy, &registry,\n        )\n        .unwrap();''',
    "current receipt test replay",
)
tail = replace_once(
    tail,
    '''        let result = admission.validate_current(\n            &request,\n            &new_evidence,\n            &profile,\n            &new_cut,\n            &policy,\n            &registry,\n        );''',
    '''        let result = validate_from_raw(\n            &admission,\n            &request,\n            &new_evidence,\n            &profile,\n            &new_cut,\n            &policy,\n            &registry,\n        );''',
    "stale receipt test replay",
)

text = head + test_marker + tail
lib.write_text(text)
