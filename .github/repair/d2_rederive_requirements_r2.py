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
    raise SystemExit("usage: d2_rederive_requirements_r2.py <legacy-d2-checkout>")

legacy = Path(sys.argv[1]).resolve()
crate = Path("crates/domains/symtropy-design-requirements")
crate.mkdir(parents=True, exist_ok=True)
(crate / "src").mkdir(parents=True, exist_ok=True)

shutil.copyfile(legacy / crate / "Cargo.toml", crate / "Cargo.toml")
shutil.copyfile(legacy / crate / "src/lib.rs", crate / "src/lib.rs")

root = Path("Cargo.toml")
root_text = root.read_text()
root_text = replace_once(
    root_text,
    '    "crates/domains/symtropy-design",\n',
    '    "crates/domains/symtropy-design",\n'
    '    "crates/domains/symtropy-design-requirements",\n',
    "workspace member",
)
root.write_text(root_text)

lib = crate / "src/lib.rs"
text = lib.read_text()
anchor = '''    #[test]\n    fn changing_verification_target_changes_semantic_identity() {\n'''
golden = '''    #[test]\n    fn canonical_requirement_set_digest_has_frozen_golden_vector() {\n        let root = requirement(\n            "requirement:service-load",\n            Vec::new(),\n            vec![obligation("obligation:deflection", "observable:deflection")],\n        );\n        let child = requirement(\n            "requirement:mass",\n            vec![root.id.clone()],\n            vec![obligation("obligation:mass", "observable:mass")],\n        );\n        let fixture = set(vec![root, child]);\n        let digest = fixture.content_digest().unwrap();\n\n        assert_eq!(digest.algorithm, id("sha256"));\n        assert_eq!(\n            digest.value,\n            "7c8287d75adf614b6364ae2adbf32932ff1ea95a51da081f975d52963e51997c"\n        );\n    }\n\n'''
text = replace_once(text, anchor, golden + anchor, "D2 canonical digest golden")
lib.write_text(text)
