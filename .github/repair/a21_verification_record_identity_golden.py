#!/usr/bin/env python3
from pathlib import Path

GOLDEN = "99bfd0f638d56a2053968186b48b0e2c98fbeb66658ac341a1dd8b6dbc89759f"
path = Path("crates/domains/symtropy-analysis-admission/src/lib.rs")
text = path.read_text()
old = '        println!("A21_RECORD_GOLDEN={}", base_digest.value);\n'
new = (
    '        println!("A21_RECORD_GOLDEN={}", base_digest.value);\n'
    f'        assert_eq!(base_digest.value, "{GOLDEN}");\n'
)
count = text.count(old)
if count != 1:
    raise SystemExit(f"A2.1 golden anchor: expected exactly one match, found {count}")
path.write_text(text.replace(old, new, 1))
