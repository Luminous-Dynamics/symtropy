#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
text = path.read_text()
old = '''    if let (Some(left), Some(right)) = (left, right) {
        if left.plane != right.plane {
            return Err(DecompositionError::AdjacentDifferentCutPlanesUnsupported {
                first: cell,
                second: other,
            });
        }
    }
'''
new = '''    if let (Some(left), Some(right)) = (left, right)
        && left.plane != right.plane
    {
        return Err(DecompositionError::AdjacentDifferentCutPlanesUnsupported {
            first: cell,
            second: other,
        });
    }
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"expected exactly one PB-04b collapsible-if site, found {count}")
path.write_text(text.replace(old, new, 1))
print("PB04B_CLIPPY_HYGIENE_REPLACEMENTS=1")
