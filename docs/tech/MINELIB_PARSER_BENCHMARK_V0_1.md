# MINE-REFERENCE-PARSER-001 — synthetic parser conformance v0.1

## What this is

A standalone Rust parser contract for two linked MineLib v1 input forms:

1. block-model descriptor rows: `id x y z attributes...`;
2. precedence rows: `block_id predecessor_count predecessor_ids...`.

The reference format says records are ASCII, comments begin with `%`, fields may be separated by spaces, tabs, or colons, block IDs begin at zero, and each precedence row identifies one block and its predecessor set. The official specification describes the block model, precedence, and optimization-model descriptor as three distinct files. See the [MineLib v1 format specification](https://minelib.org/v1/minelib_format.pdf), especially §§2–4 and §5.2.

This v0.1 parser is intentionally a **strict conformance profile**, not a complete implementation of that full specification. It accepts only contiguous zero-based block IDs in its test profile. It does not parse the optimization-model descriptor, resource coefficients, period limits, general side constraints, UPIT/CPIT/PCPSP objectives, or solve a mine-planning problem.

## Paradigm shift

Treat parsing as a scientific claim requiring evidence, not an unexamined preprocessing step. A valid-looking result can be wrong if a grade attribute is misidentified, resource limits are confused, or precedence edges are reversed. The contract therefore separates:
- raw-file identity (SHA-256),
- declared schema for otherwise user-defined block attributes,
- syntax and typed numeric interpretation,
- block/predecessor graph integrity,
- a small audit checksum from any optimization objective,
- eventual solver correctness and operational validity.

The named `block_value_usd` sum in this fixture is an arithmetic audit value only. It must not be called NPV, discounted objective, or solver objective. MineLib stores optimization objective values and other model coefficients in its separate optimization-model descriptor; reproducing an NPV requires the correct problem type, discount rate, period and objective semantics.

## Frozen synthetic fixture

Files in this repo are authored for this benchmark; they are not downloaded MineLib instances and do not represent a real deposit:
- five synthetic block rows with coordinates, tonnage, grade, and a signed fixed-decimal block value;
- five precedence rows and three directed predecessor edges;
- a manifest that declares each attribute's type and scale;
- expected block IDs `[0,1,2,3,4]`, row and edge counts, and exact scaled-integer sum `25,000,050` cents.

The parser binds the manifest and both raw files to SHA-256 hashes in a receipt, plus the exact source commit. Paths must be local relative paths without traversal. No external network retrieval occurs at run time.

## Current validation targets

- Exact synthetic answer: 5 blocks, IDs 0–4, 5 precedence rows, 3 precedence edges, exact attribute sum.
- 0 duplicate / missing block IDs admitted.
- 0 dangling, duplicate, self-referential, duplicated-row, or cyclic precedence relations admitted.
- 0 incorrect field counts or misdeclared attribute types admitted.
- Fixed decimals use exact scaled integers: no exponent notation, no floating-point conversion, and no silent rounding beyond the declared scale.
- 100% strict-schema match, manifest/file SHA-256 coverage, and exact-head receipt.
- Any malformed or unsupported field fails closed rather than being guessed.
- Identical files + manifest + commit produce an identical receipt.

## Next ladder

1. v0.1: block rows and precedence graph; synthetic known answers only.
2. v0.2: read-only parser for the MineLib optimization-model descriptor (problem type, block/period/destination/resource counts, discount rate, objective rows, resource coefficients and limits, side constraints); strict typed sections and file-to-file referential checks.
3. v0.3: licensed, source-pinned external instances; reproduce parser statistics without committing data until exact license and attribution obligations are verified.
4. v0.4: hand-checkable UPIT fixtures with exhaustive optimal solution and an independent feasibility checker.
5. v0.5: CPIT/PCPSP semantics and tiny enumerated schedule oracles; objective values only compared when discounting and formulation match exactly.
6. v1: external baseline comparison with a signed run manifest, feasible objective, best bound, optimality gap, environment and independent validation.

## Boundaries

No solver is shipped in v0.1. A successful parse does not prove the mathematical model is correctly formulated, a feasible plan is optimal, a reserve is certified, or any real mine operation is safe or profitable. This work composes with [MINE-SYSTEM-001 #1933](https://github.com/Luminous-Dynamics/symtropy/issues/1933) and [MINE-REFERENCE-001 #2015](https://github.com/Luminous-Dynamics/symtropy/issues/2015), rather than duplicating the campaign evaluator.
