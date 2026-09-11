#!/usr/bin/env python3
from pathlib import Path

lib_path = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
test_path = Path("crates/domains/symtropy-spatial-decomposition/tests/incremental_equivalence.rs")
lib = lib_path.read_text()
test = test_path.read_text()

old_variant = '''pub enum IncrementalRecomputeOutcome {
    Updated(IncrementalDecompositionUpdate),
    FullRebuildRequired(IncrementalFullRebuildReason),
}
'''
new_variant = '''pub enum IncrementalRecomputeOutcome {
    Updated(Box<IncrementalDecompositionUpdate>),
    FullRebuildRequired(IncrementalFullRebuildReason),
}
'''
if lib.count(old_variant) != 1:
    raise SystemExit("expected exactly one unboxed incremental outcome enum")
lib = lib.replace(old_variant, new_variant, 1)

old_construct = '''        Ok(IncrementalRecomputeOutcome::Updated(
            IncrementalDecompositionUpdate {
                state,
                changed_cells,
                recompute_cells,
            },
        ))
'''
new_construct = '''        Ok(IncrementalRecomputeOutcome::Updated(Box::new(
            IncrementalDecompositionUpdate {
                state,
                changed_cells,
                recompute_cells,
            },
        )))
'''
if lib.count(old_construct) != 1:
    raise SystemExit("expected exactly one unboxed incremental outcome construction")
lib = lib.replace(old_construct, new_construct, 1)

old_test = '''        IncrementalRecomputeOutcome::Updated(update) => update,
'''
new_test = '''        IncrementalRecomputeOutcome::Updated(update) => *update,
'''
if test.count(old_test) != 1:
    raise SystemExit("expected exactly one incremental test helper unwrap")
test = test.replace(old_test, new_test, 1)

lib_path.write_text(lib)
test_path.write_text(test)
print("PB-04c large outcome repair applied")
