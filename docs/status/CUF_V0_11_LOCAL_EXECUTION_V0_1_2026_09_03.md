# CUF v0.11 Local Execution v0.1 — 2026-09-03

## Status

Operator-execution hardening stacked on PR #117.

This tranche changes no simulation semantics. It adds fail-safe fresh-run and staged-resume harnesses and hardens the exact-tree Q1 dependency boundary used by qualification/promotion.

## Goal

Reduce the remaining local replay workflow to one explicit command while preserving every red result for inspection and attributing failures to the correct lineage.

The harnesses do not commit, reset, clean, stash, unstage, or silently repair anything.

## Fresh-run entry point

```bash
bash scripts/run-cuf-v0.11-parent-candidate-local.sh \
  /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch \
  /tmp/cuf-v0.11-parent-evidence
```

The second argument is optional. If omitted, a UTC-timestamped `/tmp` evidence path is selected.

## Fresh-run preconditions

The fresh-run harness fails before v4.8 mutation unless:

- HEAD descends from the frozen PR #117 evidence-integrity head;
- the retained v4.8 patch SHA-256 is exact;
- `../mycelix-multiworld-sim` exists;
- the worktree and index are completely clean;
- the evidence output is outside the repository and does not already exist;
- candidate composition verifies;
- all promotion-critical qualification tooling is byte-identical to committed HEAD;
- `nix develop` exposes `rustc` and `cargo` without changing repository state;
- the **pre-v4.8 parent** resolves under the committed `Cargo.lock` with `cargo metadata --locked --no-deps`;
- the retained patch passes the existing non-mutating Q0 preflight.

The parent lock check intentionally happens **before** the 275-file replay. If it fails, the harness does not stage v4.8 and does not misattribute the older parent-lock defect to Universal Matter.

Filesystem and memory snapshots are printed for operator visibility but no arbitrary resource threshold is invented.

## Mutual exclusion

Fresh and resume execution share one atomic Git-directory lock:

```text
$(git rev-parse --git-path cuf-v0.11-parent-candidate.lock)
```

The lock records PID, mode, UTC start time, and candidate HEAD. A second fresh/resume invocation refuses to start while the lock exists.

Normal exit and handled interruption remove the lock. A hard process or machine kill may leave a stale lock. In that case the operator must inspect the recorded owner/process state and remove the lock manually only after confirming no qualification process is still active.

This lock is intentionally outside tracked repository content. It prevents two supported operator harnesses from racing on the same Git index; it is not presented as a universal lock against arbitrary unrelated Git commands.

## Mutating step

Only after all parent/readiness checks pass does the fresh-run harness invoke the guarded replay helper.

The helper:

- reruns Q0 preflight;
- applies the exact retained patch with `git apply --index`;
- stages exactly 275 paths;
- does not commit.

The harness then rejects any unstaged or untracked side effect.

## Exact dependency identity

Q1 is an **exact repository-tree** claim, so its Cargo dependency identity is the committed/staged repository `Cargo.lock`.

Promotion-critical workspace commands use Cargo `--locked`, and the combined post-replay gate performs:

```bash
cargo metadata --locked --no-deps --format-version 1
```

before expensive Terrain compilation. Workspace/license metadata checks are lock-bound too.

There are therefore two distinct lock failure classes:

1. **Parent prerequisite failure** — the clean pre-v4.8 parent cannot resolve under its committed lock. Stop before replay and repair the parent lineage explicitly.
2. **Pristine v4.8 Q1 failure** — the parent was lock-coherent, but the exact retained replay introduces a locked-tree incompatibility or another authored-tree Q1 failure. Preserve the authored replay/failure before any repair.

These must never be collapsed into the same historical claim.

Cargo is not allowed to reconcile `Cargo.lock` during Q1 and then have the evidence wrapper complain only after the build. Any required reconciliation is a **separate explicit repair lineage**. It must not be added to the retained 275-file artifact and described as pristine authored v4.8.

The dependency-light standalone Tier A continuation check remains useful for portability/CI, but its independently resolved semver graph is explicitly:

```text
SUPPLEMENTARY_NOT_PROMOTION
```

It is not part of exact-tree Q1 promotion authority. The promotion-critical continuation tests/golden vectors run under the repository lock instead.

The evidence `LINEAGE.txt` binds:

```text
dependency_resolution=repository_Cargo.lock_locked
tier_a_portability=SUPPLEMENTARY_NOT_PROMOTION
```

and the promotion verifier requires those exact rules.

## Qualification/evidence

The evidence capture itself remains in the host shell. It explicitly queries `rustc` and `cargo` from `nix develop` for `TOOLCHAIN.txt`, then enters `nix develop` for the actual qualification gate.

This avoids a redundant nested `nix develop -> capture -> nix develop -> qualification` path while still proving the toolchain that actually qualifies the candidate.

The evidence-integrity layer independently requires:

- exact candidate/tooling identity before and after;
- unchanged HEAD;
- unchanged staged tree;
- unchanged `Cargo.lock`;
- unchanged repository status;
- exact retained patch/staged path equality;
- no unstaged or untracked side effects;
- PASS from lock-bound Q0/Q1 + continuation-core gates.

## Failure semantics

A failure **before replay** leaves the repository clean and is classified as a prerequisite/readiness failure, not a v4.8 Q1 result.

On interruption or failure **after replay**, the fresh-run harness intentionally preserves:

- the staged 275-path candidate;
- any generated FAIL evidence capsule;
- the current repository state.

An EXIT guard prints the preserved candidate/evidence state after any non-zero exit once mutation has begun, including unexpected shell-command failures—not only expected qualification failures.

It never runs `git reset`, `git clean`, `git stash`, or `git commit`.

This is deliberate: failed qualification is evidence, not trash to erase automatically.

## Resume exact staged candidate

If the terminal, machine, or qualification process stops after the exact v4.8 replay has already been staged, do **not** reset and replay solely to restart evidence capture.

Use a new evidence directory:

```bash
bash scripts/resume-cuf-v0.11-parent-candidate-local.sh \
  /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch \
  /tmp/cuf-v0.11-parent-resume-evidence
```

The resume harness refuses to proceed unless:

- HEAD still descends from the frozen evidence-integrity lineage;
- the patch SHA-256 is exact;
- the private sibling exists;
- exactly 275 paths are staged;
- the sorted staged path set exactly equals the retained patch path set;
- there are no unstaged tracked changes;
- there are no untracked files;
- candidate composition still verifies;
- every promotion-critical qualification tool is still byte-identical to committed HEAD.

It does **not** call the replay helper again.

Before evidence capture it also snapshots the exact staged Git tree, probes the Nix development environment, and requires the staged tree to remain byte-identical afterward.

A previous FAIL capsule is retained separately; resume always writes to a new evidence directory rather than overwriting earlier evidence.

## PASS semantics

A PASS from either fresh or resumed execution means only the pre-Q2 parent candidate has passed the exact lock-bound Q0/Q1 + continuation-core evidence lane.

It still does **not** claim Q2.

The staged tree remains uncommitted until explicit review.

If the evidence is accepted, the operator may create a single promotion commit and then must run:

```bash
bash scripts/verify-cuf-v0.11-parent-promotion.sh /path/to/evidence HEAD
```

The promotion verifier must pass before #76, #79, or #81 begins.

## Qualification hierarchy

```text
fresh local readiness
    ↓
atomic fresh/resume lock
    ↓
pre-v4.8 parent Cargo.lock --locked check
    ├── FAIL ──→ Stage A parent repair (#119), no v4.8 replay
    │
    ▼
Q0 structural patch preflight
    ↓
exact staged v4.8 replay
    ↓
post-replay Cargo.lock --locked Q1 check
    ├── FAIL ──→ preserve pristine v4.8 Q1 failure, Stage B repair (#119)
    │
    ▼
lock-bound Q1 + continuation evidence
    ├──────── interruption/failure ────────┐
    │                                      │
    ▼                                      ▼
 explicit operator review            exact-staged resume
    │                                      │
    └──────────────────┬───────────────────┘
                       ▼
            exact-tree promotion verification
                       ↓
               pre-Q2 committed parent
                       ↓
                #76 / #79 / #81
                       ↓
             Q2 continuation/replay evidence
```

## Non-goals

This tranche does not:

- modify Universal Matter v4.8;
- silently reconcile Cargo.lock;
- hand-edit a missing workspace lock entry;
- implement either Q1 repair lineage from #119;
- implement Q2 repairs;
- implement native CUF v0.11 adapters;
- add orbital/stellar runtime code;
- change persistence schemas;
- auto-promote a passing tree;
- erase or overwrite failed qualification evidence;
- claim the local lock protects against arbitrary unrelated Git processes.
