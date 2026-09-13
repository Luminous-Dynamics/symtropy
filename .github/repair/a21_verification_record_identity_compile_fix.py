#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/domains/symtropy-analysis-admission/src/lib.rs")
text = path.read_text()
old = '''        let evidence = evidence(&request, "solver-a", true);\n        let profile = profile();\n        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);\n        let policy = admission_policy(&profile);\n        let base = verification_records(\n            &request,\n            &evidence,'''
new = '''        let base_evidence = evidence(&request, "solver-a", true);\n        let profile = profile();\n        let cut = cut(\n            &request,\n            &base_evidence,\n            &profile,\n            FacetDisposition::Established,\n        );\n        let policy = admission_policy(&profile);\n        let base = verification_records(\n            &request,\n            &base_evidence,'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"A2.1 shadowing anchor: expected exactly one match, found {count}")
path.write_text(text.replace(old, new, 1))
