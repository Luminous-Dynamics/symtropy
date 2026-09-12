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
    raise SystemExit("usage: a0_rederive_analysis_r2.py <legacy-a0-checkout>")

legacy = Path(sys.argv[1]).resolve()
crate = Path("crates/domains/symtropy-analysis-contracts")
(crate / "src").mkdir(parents=True, exist_ok=True)

shutil.copyfile(legacy / crate / "Cargo.toml", crate / "Cargo.toml")
shutil.copyfile(legacy / crate / "src/lib.rs", crate / "src/lib.rs")

root = Path("Cargo.toml")
root_text = root.read_text()
root_text = replace_once(
    root_text,
    '    "crates/domains/symtropy-design-requirements",\n',
    '    "crates/domains/symtropy-design-requirements",\n'
    '    "crates/domains/symtropy-analysis-contracts",\n',
    "workspace member",
)
root.write_text(root_text)

lib = crate / "src/lib.rs"
text = lib.read_text()

# Rust 1.96 strict Clippy requires these nested canonical-order checks to be
# expressed as let-chains. This is a control-flow-only refactor: the exact
# request/evidence golden digests below must remain unchanged.
text = replace_once(
    text,
    '''        if let Some(previous) = previous {\n            if previous >= &artifact.id {\n                return if previous == &artifact.id {\n                    Err(AnalysisError::DuplicateArtifact(artifact.id.clone()))\n                } else {\n                    Err(AnalysisError::NonCanonicalOrder("artifacts"))\n                };\n            }\n        }\n''',
    '''        if let Some(previous) = previous\n            && previous >= &artifact.id\n        {\n            return if previous == &artifact.id {\n                Err(AnalysisError::DuplicateArtifact(artifact.id.clone()))\n            } else {\n                Err(AnalysisError::NonCanonicalOrder("artifacts"))\n            };\n        }\n''',
    "artifact canonical-order Clippy refactor",
)
text = replace_once(
    text,
    '''        if let Some(previous) = previous {\n            if previous >= &observable.observable_id {\n                return if previous == &observable.observable_id {\n                    Err(AnalysisError::DuplicateObservable(\n                        observable.observable_id.clone(),\n                    ))\n                } else {\n                    Err(AnalysisError::NonCanonicalOrder("requested_observables"))\n                };\n            }\n        }\n''',
    '''        if let Some(previous) = previous\n            && previous >= &observable.observable_id\n        {\n            return if previous == &observable.observable_id {\n                Err(AnalysisError::DuplicateObservable(\n                    observable.observable_id.clone(),\n                ))\n            } else {\n                Err(AnalysisError::NonCanonicalOrder("requested_observables"))\n            };\n        }\n''',
    "observable canonical-order Clippy refactor",
)
text = replace_once(
    text,
    '''        if let Some(previous) = previous {\n            if previous >= reference {\n                return if previous == reference {\n                    Err(AnalysisError::DuplicateExactSemanticRef(reference.clone()))\n                } else {\n                    Err(AnalysisError::NonCanonicalOrder(field))\n                };\n            }\n        }\n''',
    '''        if let Some(previous) = previous\n            && previous >= reference\n        {\n            return if previous == reference {\n                Err(AnalysisError::DuplicateExactSemanticRef(reference.clone()))\n            } else {\n                Err(AnalysisError::NonCanonicalOrder(field))\n            };\n        }\n''',
    "exact-ref canonical-order Clippy refactor",
)
text = replace_once(
    text,
    '''        if let Some(previous) = previous {\n            if previous >= &observation.id {\n                return if previous == &observation.id {\n                    Err(AnalysisError::DuplicateObservation(observation.id.clone()))\n                } else {\n                    Err(AnalysisError::NonCanonicalOrder("observations"))\n                };\n            }\n        }\n''',
    '''        if let Some(previous) = previous\n            && previous >= &observation.id\n        {\n            return if previous == &observation.id {\n                Err(AnalysisError::DuplicateObservation(observation.id.clone()))\n            } else {\n                Err(AnalysisError::NonCanonicalOrder("observations"))\n            };\n        }\n''',
    "observation canonical-order Clippy refactor",
)

anchor = '''    #[test]\n    fn changing_exact_design_changes_request_identity() {\n'''
goldens = '''    #[test]\n    fn canonical_analysis_request_digest_has_frozen_golden_vector() {\n        let request = deflection_request(subject("design"));\n        let digest = request.content_digest().unwrap();\n        assert_eq!(digest.algorithm, id("sha256"));\n        assert_eq!(\n            digest.value,\n            "3b0ce5c820d1dd550b164c7691ba9804ecc1f4a672d8615fa7f732e372829037"\n        );\n    }\n\n    #[test]\n    fn canonical_analysis_evidence_digest_has_frozen_golden_vector() {\n        let request = deflection_request(subject("design"));\n        let evidence = evidence(\n            &request,\n            RunDisposition::Completed,\n            NumericalDisposition::Converged,\n            vec![result_observation()],\n            "solver-a",\n        )\n        .unwrap();\n        let digest = evidence.content_digest(&request).unwrap();\n        assert_eq!(digest.algorithm, id("sha256"));\n        assert_eq!(\n            digest.value,\n            "501e1ec4aabe3ebcbc49b0db12ae9a0963415df32dfa040e05e45ae3cf514dc5"\n        );\n    }\n\n'''
text = replace_once(text, anchor, goldens + anchor, "A0 canonical digest goldens")
lib.write_text(text)
