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
    raise SystemExit("usage: a1_rederive_qualification_r2.py <legacy-a1-checkout>")

legacy = Path(sys.argv[1]).resolve()
crate = Path("crates/domains/symtropy-analysis-qualification")
(crate / "src").mkdir(parents=True, exist_ok=True)
shutil.copyfile(legacy / crate / "Cargo.toml", crate / "Cargo.toml")
shutil.copyfile(legacy / crate / "src/lib.rs", crate / "src/lib.rs")

root = Path("Cargo.toml")
root_text = root.read_text()
root_text = replace_once(
    root_text,
    '    "crates/domains/symtropy-analysis-contracts",\n',
    '    "crates/domains/symtropy-analysis-contracts",\n'
    '    "crates/domains/symtropy-analysis-qualification",\n',
    "workspace member",
)
root.write_text(root_text)

lib = crate / "src/lib.rs"
text = lib.read_text()

text = replace_once(
    text,
    '''use symtropy_analysis_contracts::{\n    AnalysisEvidence, AnalysisEvidenceRef, AnalysisRequest, AnalysisRequestRef, ExactSemanticRef,\n};''',
    '''use symtropy_analysis_contracts::{\n    AnalysisEvidenceRef, AnalysisRequestRef, ExactSemanticRef, ValidatedAnalysisEvidence,\n};''',
    "production A0 imports",
)

# Rust 1.96 strict-Clippy hygiene without changing ordering semantics.
text = replace_once(
    text,
    '''            if let Some(previous) = previous {\n                if previous >= &rule.facet_id {\n                    return if previous == &rule.facet_id {\n                        Err(QualificationError::DuplicateFacet(rule.facet_id.clone()))\n                    } else {\n                        Err(QualificationError::NonCanonicalOrder("profile.rules"))\n                    };\n                }\n            }''',
    '''            if let Some(previous) = previous\n                && previous >= &rule.facet_id\n            {\n                return if previous == &rule.facet_id {\n                    Err(QualificationError::DuplicateFacet(rule.facet_id.clone()))\n                } else {\n                    Err(QualificationError::NonCanonicalOrder("profile.rules"))\n                };\n            }''',
    "profile collapsible-if",
)

cut_doc = '''/// One coherent immutable evidence cut for a consequential analysis\n/// qualification decision.\n#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct AnalysisQualificationCut {'''
cut_doc_new = '''/// One coherent immutable evidence cut for a consequential analysis\n/// qualification decision.\n///\n/// A1 deliberately accepts only request-replayed A0 evidence for consequential\n/// operations. Raw/restored `AnalysisEvidence` cannot be passed directly.\n///\n/// ```compile_fail\n/// use symtropy_analysis_contracts::AnalysisEvidence;\n/// use symtropy_analysis_qualification::{AnalysisQualificationCut, QualificationProfile};\n/// fn raw_evidence_cannot_qualify(\n///     cut: &AnalysisQualificationCut,\n///     evidence: &AnalysisEvidence,\n///     profile: &QualificationProfile,\n/// ) {\n///     let _ = cut.decision(evidence, profile);\n/// }\n/// ```\n#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct AnalysisQualificationCut {'''
text = replace_once(text, cut_doc, cut_doc_new, "compile-fail A0 to A1 authority guard")

old_impl_head = '''    pub fn new(\n        id: QualificationCutId,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        evaluation_context: ExactSemanticRef,\n        mut assessments: Vec<FacetAssessment>,\n    ) -> Result<Self, QualificationError> {\n        assessments.sort_by(|left, right| left.facet_id.cmp(&right.facet_id));\n        let cut = Self {\n            schema_version: QUALIFICATION_CUT_SCHEMA_VERSION,\n            id,\n            request: request.exact_ref().map_err(QualificationError::Analysis)?,\n            evidence: evidence\n                .exact_ref(request)\n                .map_err(QualificationError::Analysis)?,\n            profile: profile.exact_ref()?,\n            evaluation_context,\n            assessments,\n        };\n        cut.validate_against(request, evidence, profile)?;\n        Ok(cut)\n    }'''
new_impl_head = '''    pub fn new(\n        id: QualificationCutId,\n        evidence: &ValidatedAnalysisEvidence<'_>,\n        profile: &QualificationProfile,\n        evaluation_context: ExactSemanticRef,\n        mut assessments: Vec<FacetAssessment>,\n    ) -> Result<Self, QualificationError> {\n        let request = evidence.request();\n        assessments.sort_by(|left, right| left.facet_id.cmp(&right.facet_id));\n        let cut = Self {\n            schema_version: QUALIFICATION_CUT_SCHEMA_VERSION,\n            id,\n            request: request.exact_ref().map_err(QualificationError::Analysis)?,\n            evidence: evidence.exact_ref().map_err(QualificationError::Analysis)?,\n            profile: profile.exact_ref()?,\n            evaluation_context,\n            assessments,\n        };\n        cut.validate_against(evidence, profile)?;\n        Ok(cut)\n    }'''
text = replace_once(text, old_impl_head, new_impl_head, "qualification cut constructor")

old_decision_sig = '''    pub fn decision(\n        &self,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n    ) -> Result<QualificationDecision, QualificationError> {\n        self.validate_against(request, evidence, profile)?;'''
new_decision_sig = '''    pub fn decision(\n        &self,\n        evidence: &ValidatedAnalysisEvidence<'_>,\n        profile: &QualificationProfile,\n    ) -> Result<QualificationDecision, QualificationError> {\n        self.validate_against(evidence, profile)?;'''
text = replace_once(text, old_decision_sig, new_decision_sig, "qualification decision signature")

old_digest_validate = '''    pub fn content_digest(\n        &self,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n    ) -> Result<ContentDigest, QualificationError> {\n        self.validate_against(request, evidence, profile)?;\n        sha256_digest(&self.canonical_preimage()?)\n    }\n\n    pub fn validate_against(\n        &self,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n    ) -> Result<(), QualificationError> {'''
new_digest_validate = '''    pub fn content_digest(\n        &self,\n        evidence: &ValidatedAnalysisEvidence<'_>,\n        profile: &QualificationProfile,\n    ) -> Result<ContentDigest, QualificationError> {\n        self.validate_against(evidence, profile)?;\n        sha256_digest(&self.canonical_preimage()?)\n    }\n\n    pub fn validate_against(\n        &self,\n        evidence: &ValidatedAnalysisEvidence<'_>,\n        profile: &QualificationProfile,\n    ) -> Result<(), QualificationError> {\n        let request = evidence.request();'''
text = replace_once(text, old_digest_validate, new_digest_validate, "digest and validation signatures")

text = replace_once(
    text,
    '''        let expected_evidence = evidence\n            .exact_ref(request)\n            .map_err(QualificationError::Analysis)?;''',
    '''        let expected_evidence = evidence.exact_ref().map_err(QualificationError::Analysis)?;''',
    "validated exact evidence ref",
)

text = replace_once(
    text,
    '''            if let Some(previous) = previous {\n                if previous >= &assessment.facet_id {\n                    return if previous == &assessment.facet_id {\n                        Err(QualificationError::DuplicateAssessment(\n                            assessment.facet_id.clone(),\n                        ))\n                    } else {\n                        Err(QualificationError::NonCanonicalOrder(\n                            "cut.assessments",\n                        ))\n                    };\n                }\n            }''',
    '''            if let Some(previous) = previous\n                && previous >= &assessment.facet_id\n            {\n                return if previous == &assessment.facet_id {\n                    Err(QualificationError::DuplicateAssessment(\n                        assessment.facet_id.clone(),\n                    ))\n                } else {\n                    Err(QualificationError::NonCanonicalOrder(\n                        "cut.assessments",\n                    ))\n                };\n            }''',
    "assessment collapsible-if",
)

text = replace_once(
    text,
    '''        if let Some(previous) = previous {\n            if previous >= reference {\n                return if previous == reference {\n                    Err(QualificationError::DuplicateSupportingEvidence(\n                        reference.clone(),\n                    ))\n                } else {\n                    Err(QualificationError::NonCanonicalOrder(field))\n                };\n            }\n        }''',
    '''        if let Some(previous) = previous\n            && previous >= reference\n        {\n            return if previous == reference {\n                Err(QualificationError::DuplicateSupportingEvidence(\n                    reference.clone(),\n                ))\n            } else {\n                Err(QualificationError::NonCanonicalOrder(field))\n            };\n        }''',
    "supporting evidence collapsible-if",
)

# Test module needs raw fixture types explicitly now that production no longer imports them.
text = replace_once(
    text,
    '''    use symtropy_analysis_contracts::{\n        AnalysisEvidenceId, AnalysisObservation, AnalysisObservationId, AnalysisProfileId,\n        AnalysisProfileRef, AnalysisRequestId, AnalysisValue, NumericalDisposition,\n        ObservableRequest, ObservationClass, RunDisposition, SolverIdentity,\n    };''',
    '''    use symtropy_analysis_contracts::{\n        AnalysisEvidence, AnalysisEvidenceId, AnalysisObservation, AnalysisObservationId,\n        AnalysisProfileId, AnalysisProfileRef, AnalysisRequest, AnalysisRequestId, AnalysisValue,\n        NumericalDisposition, ObservableRequest, ObservationClass, RunDisposition, SolverIdentity,\n    };''',
    "test raw fixture imports",
)

old_cut_helper = '''    fn cut(\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        assessments: Vec<FacetAssessment>,\n    ) -> Result<AnalysisQualificationCut, QualificationError> {\n        AnalysisQualificationCut::new(\n            QualificationCutId::new(id("qualification-cut:bracket")),\n            request,\n            evidence,\n            profile,\n            exact("authority:clock", "evaluation-context:1", "context"),\n            assessments,\n        )\n    }'''
new_cut_helper = '''    fn cut(\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n        assessments: Vec<FacetAssessment>,\n    ) -> Result<AnalysisQualificationCut, QualificationError> {\n        let validated = evidence.rebind(request).map_err(QualificationError::Analysis)?;\n        AnalysisQualificationCut::new(\n            QualificationCutId::new(id("qualification-cut:bracket")),\n            &validated,\n            profile,\n            exact("authority:clock", "evaluation-context:1", "context"),\n            assessments,\n        )\n    }\n\n    fn decision(\n        cut: &AnalysisQualificationCut,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n    ) -> Result<QualificationDecision, QualificationError> {\n        let validated = evidence.rebind(request).map_err(QualificationError::Analysis)?;\n        cut.decision(&validated, profile)\n    }\n\n    fn cut_digest(\n        cut: &AnalysisQualificationCut,\n        request: &AnalysisRequest,\n        evidence: &AnalysisEvidence,\n        profile: &QualificationProfile,\n    ) -> Result<ContentDigest, QualificationError> {\n        let validated = evidence.rebind(request).map_err(QualificationError::Analysis)?;\n        cut.content_digest(&validated, profile)\n    }'''
text = replace_once(text, old_cut_helper, new_cut_helper, "test replay helpers")

text = text.replace(
    "cut.decision(&request, &evidence, &profile)",
    "decision(&cut, &request, &evidence, &profile)",
)
text = text.replace(
    "left.content_digest(&request, &left_evidence, &profile)",
    "cut_digest(&left, &request, &left_evidence, &profile)",
)
text = text.replace(
    "right\n                .content_digest(&request, &right_evidence, &profile)",
    "cut_digest(&right, &request, &right_evidence, &profile)",
)
text = text.replace(
    "left.content_digest(&request, &evidence, &profile).unwrap()",
    "cut_digest(&left, &request, &evidence, &profile).unwrap()",
)
text = text.replace(
    "right.content_digest(&request, &evidence, &profile).unwrap()",
    "cut_digest(&right, &request, &evidence, &profile).unwrap()",
)

# Freeze the exact legacy schema-v1 identities recovered by an independent fixture.
golden_anchor = '''    #[test]\n    fn complete_accepted_facets_establish_qualification() {'''
golden_tests = '''    #[test]\n    fn canonical_qualification_profile_digest_has_frozen_golden_vector() {\n        assert_eq!(\n            profile().content_digest().unwrap().value,\n            "e09131561217af2c85f6ed4665df938d9389909ace1c2413de48f7bab93bf618"\n        );\n    }\n\n    #[test]\n    fn canonical_qualification_cut_digest_has_frozen_golden_vector() {\n        let request = request();\n        let evidence = evidence(&request, true, "solver-a");\n        let profile = profile();\n        let cut = cut(\n            &request,\n            &evidence,\n            &profile,\n            vec![\n                assessment(\n                    &request,\n                    &evidence,\n                    "facet:numerical",\n                    "authority:numerical-qualification",\n                    FacetDisposition::Established,\n                ),\n                assessment(\n                    &request,\n                    &evidence,\n                    "facet:model-applicability",\n                    "authority:model-qualification",\n                    FacetDisposition::Established,\n                ),\n            ],\n        )\n        .unwrap();\n        assert_eq!(\n            cut_digest(&cut, &request, &evidence, &profile)\n                .unwrap()\n                .value,\n            "ab4ad7408f6e03d203ff7ccc660816df4214baad798f9bb01ab7d24125cba03d"\n        );\n    }\n\n'''
text = replace_once(text, golden_anchor, golden_tests + golden_anchor, "A1 schema-v1 golden tests")

lib.write_text(text)
