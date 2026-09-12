#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/domains/symtropy-analysis-contracts/src/lib.rs")
text = path.read_text()
old = '''/// Immutable execution evidence for one exact request.\n#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct AnalysisEvidence {\n'''
new = '''/// Immutable execution evidence for one exact request.\n///\n/// Raw/restored evidence deliberately has no affirmative result-qualification API.\n/// Consumers must replay it against the exact request and retain the validated\n/// wrapper before asking whether requested results may enter downstream qualification.\n///\n/// ```compile_fail\n/// use symtropy_analysis_contracts::AnalysisEvidence;\n/// fn raw_evidence_cannot_authorize(evidence: &AnalysisEvidence) {\n///     let _ = evidence.may_enter_result_qualification();\n/// }\n/// ```\n#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct AnalysisEvidence {\n'''
if text.count(old) != 1:
    raise SystemExit(f"raw evidence API guard: expected one anchor, found {text.count(old)}")
path.write_text(text.replace(old, new, 1))
