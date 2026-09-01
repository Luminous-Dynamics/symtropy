# CUF v0.11 Parent Candidate — Status

Date: 2026-09-01
Status: **Authored / unqualified**

## Purpose

This branch freezes the exact descendant intended to become the parent of native CUF v0.11 observation adapters after Universal Matter v4.8 is replayed and the appropriate qualification layers pass.

It does **not** apply Universal Matter v4.8 and does **not** claim Q0, Q1, Q2, or Q3 PASS.

## Frozen ancestry

The finalized continuation head is:

- `c90b5ad1966263ff35b2eee73f8bf6655377344a`

It is a strict descendant of the frozen Universal Matter v4.8 + CUF v0.10.1 integration head:

- `2222e6c2d23b95372aa9a93763018b61e3d5351f`

The descendant relation is intentional. The continuation work did not modify Universal Matter Terrain preimages, so the retained v4.8 artifact remains structurally replayable against the same lineage subject to the local Q0 preflight.

## Frozen portable-contract blobs

The parent-candidate verifier binds the working tree to these exact source/test blobs:

- `crates/core/symtropy-sim-contracts/src/continuation.rs`
  - `b8121d7fbeffa07a11e7097ea5307f9edb4cd9c2`
- `crates/core/symtropy-sim-contracts/src/lib.rs`
  - `01609d3ecbf703edf172525332c231428bf94770`
- `crates/core/symtropy-sim-contracts/src/lineage.rs`
  - `76eee3bd1a8d5702a0be30356e842f4f16289357`
- `crates/core/symtropy-sim-contracts/src/observation.rs`
  - `392c73200b5dc0ab04178157276cc43d34a8eb13`
- `crates/core/symtropy-sim-contracts/tests/continuation_golden.rs`
  - `5c4fc814b612a0df2c801272a22013292a2e477c`

Working-tree blobs are checked rather than only committed blobs so the proof remains valid after the 275-file Universal Matter patch is staged.

## Retained Universal Matter artifact

Required artifact SHA-256:

`23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637`

Expected replay shape:

- 275 unique paths total
- 269 new paths
- 6 modified paths
- 0 deleted paths

The evidence capture requires the sorted staged-path set to equal the sorted retained-patch path set byte-for-byte; path-count equality alone is insufficient.

## Local execution sequence

From the exact parent-candidate branch in the full/private workspace:

```bash
PATCH=/path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch

bash scripts/preflight-universal-matter-v4.8.sh "$PATCH"
bash scripts/apply-universal-matter-v4.8.sh "$PATCH"

nix develop --command bash scripts/capture-cuf-v0.11-parent-candidate-evidence.sh \
  "$PATCH" \
  /tmp/cuf-v0.11-parent-candidate-evidence
```

The apply helper intentionally leaves the v4.8 replay staged rather than committing automatically.

## Candidate qualification gate

`scripts/qualify-cuf-v0.11-parent-candidate.sh` runs, on the same staged candidate tree:

1. finalized descendant/composition verification;
2. the existing Universal Matter v4.8 + CUF v0.10.1 qualification gate;
3. the finalized dependency-light world-continuation core gate;
4. descendant/composition verification again;
5. staged-tree stability;
6. `Cargo.lock` stability;
7. no unstaged or untracked qualification side effects;
8. Git diff hygiene.

A green result therefore means only:

- guarded v4.8 Q0/Q1-style build/integration gates passed on this candidate;
- the finalized continuation core passed its dependency-light Rust gate on the same candidate lineage.

It does **not** mean Q2 continuation/replay correctness has passed.

## Evidence capsule

`capture-cuf-v0.11-parent-candidate-evidence.sh` records outside the worktree:

- candidate HEAD;
- staged Git tree;
- frozen v4.8/CUF ancestor;
- frozen continuation semantic head;
- retained v4.8 patch SHA-256;
- exact patch/staged path lists;
- parent-composition proof;
- Cargo.lock identity before/after;
- toolchain identity;
- repository status before/after;
- qualification stdout/stderr;
- PASS/FAIL status;
- checksum manifest over the capsule.

The capsule explicitly records:

`q2_status=NOT_CLAIMED`

## Q2 blockers

Before this tree can be called continuation-qualified, the post-replay Q2 repair program remains required, including at minimum:

- #76 — Hydrology continuation identity / active frontier;
- #79 — authority continuation identity hardening, including Thermal and active lava/eruption propagation;
- #81 — persistent deterministic landscape-environment residuals.

The conserved groundwater-infiltration transfer tracked by #77 is also required before rainfall partitioning may be described as authoritative groundwater recharge.

## Q3 start condition

Native CUF v0.11 adapters (#72) remain blocked until a child of this lineage earns Q2 continuation/replay qualification.

Q3 must therefore inherit:

1. this exact parent-candidate lineage or a verified successor;
2. a qualified Universal Matter v4.8/repaired physical authority tree;
3. Q2 continuation evidence;
4. the unit/provenance semantics frozen in the native authority-binding contract.

## Scope

This parent-candidate tranche adds qualification/evidence machinery only.

It does not mutate Terrain, Basin, Universal Matter state, persistence formats, orbital systems, or living-world gameplay semantics.
