# CUF v0.11 Cargo.lock Repair Evidence v0.1 — 2026-09-03

## Status

Execution/evidence hardening stacked on PR #118.

This tranche changes no simulation semantics and does not repair Cargo.lock itself. It adds fail-closed operator paths for preserving a failed pristine authored v4.8 tree and for generating/proving an explicit `Cargo.lock`-only repair after a real locked-metadata failure.

## Why this exists

PR #118 separates two likely dependency boundaries:

1. **Stage A — parent prerequisite:** the pre-v4.8 CUF parent may already be lock-incoherent.
2. **Stage B — authored v4.8:** the retained Terrain manifest adds dependencies/features that may require a separate post-authorship lock reconciliation.

Those repairs must never be conflated with each other, with the 275-file authored Universal Matter artifact, or with Q1/Q2 qualification.

## Core invariants

```text
A lock repair may change Cargo.lock and nothing else.
```

and:

```text
A failed authored v4.8 preservation commit may contain exactly the retained
275-path authored tree and the unchanged lockfile — nothing repaired yet.
```

The repair generator therefore requires a completely clean committed starting tree and refuses to stage, commit, reset, clean, or stash anything.

It shares the same atomic Git-directory execution lock as the #118 fresh/resume harnesses. A lock repair therefore cannot race a supported v0.11 replay/qualification harness against the same worktree/index.

## Context label versus lineage proof

The first repair-generator argument (`parent` or `v4.8`) is a human/operator context label. It is useful for evidence organization, but it is **not** treated as cryptographic lineage proof.

The evidence says this explicitly:

```text
repair_context_authority=OPERATOR_LABEL_ONLY
repair_base_identity_authority=COMMIT_AND_TREE
```

The authoritative identity is the exact committed repair base HEAD and Git tree. Separate Stage A/Stage B lineage verification must prove what that base commit represents.

This prevents a mislabeled invocation from silently becoming historical truth merely because an operator typed `v4.8`.

## Failure classification before mutation

The generator first runs, inside `nix develop` with `LC_ALL=C`:

```bash
cargo metadata --locked --no-deps --format-version 1
```

`LC_ALL=C` stabilizes the Cargo diagnostic language used by the fail-closed classifier.

Mutation is allowed only if:

- the command fails; and
- Cargo's stderr says the lock file needs to be updated; and
- the error explicitly references `--locked` preventing that update.

If locked metadata already passes, the evidence status is `NOT_NEEDED` and no mutation occurs.

If metadata fails for any other reason, the evidence status is `UNCLASSIFIED_PRECHECK_FAILURE` and no mutation occurs.

This prevents a malformed manifest, source/configuration problem, registry problem, or unrelated Cargo error from being misdiagnosed as a lockfile repair opportunity.

The Nix/toolchain readiness probe and locked precheck are also required to leave repository state unchanged before mutation is permitted.

## Repair generation

Use only after the relevant Stage A or Stage B base is committed and the worktree/index are clean:

```bash
bash scripts/prepare-cuf-v0.11-cargo-lock-repair.sh \
  parent \
  /tmp/cuf-v0.11-parent-lock-repair
```

or:

```bash
bash scripts/prepare-cuf-v0.11-cargo-lock-repair.sh \
  v4.8 \
  /tmp/cuf-v0.11-v4.8-lock-repair
```

When the locked failure is correctly classified, the generator runs inside the same Nix environment with deterministic diagnostic locale:

```bash
LC_ALL=C cargo metadata --no-deps --format-version 1
```

and then requires:

- exactly one tracked path changed: `Cargo.lock`;
- no staged changes;
- no untracked files;
- all tracked `Cargo.toml` blobs remain byte-identical to the committed repair base;
- `git diff --check -- Cargo.lock` passes;
- the generated lockfile has a new SHA-256;
- a second `LC_ALL=C cargo metadata --locked --no-deps --format-version 1` passes.

The generated repair remains **unstaged and uncommitted** for explicit review.

## Evidence capsule

A successful generation emits `STATUS.txt`:

```text
GENERATED_UNCOMMITTED
```

and checksum-binds, among other files:

- operator context label and its non-authoritative status;
- exact base commit and tree;
- pre/post `Cargo.lock` SHA-256;
- all tracked Cargo manifest paths and base blobs;
- locked precheck command, exit, stdout/stderr;
- failure classification;
- Cargo generation command and exit;
- generated `Cargo.lock` unified diff + stat;
- locked postcheck command and exit;
- Nix/Rust/Cargo toolchain identity;
- lineage claims;
- `MANIFEST.sha256` over the capsule.

Failure/interrupt paths also retain a checksum manifest when an evidence directory already exists, so partial mutation attempts do not become unstructured terminal lore.

The lineage explicitly states:

```text
repair_context_authority=OPERATOR_LABEL_ONLY
repair_base_identity_authority=COMMIT_AND_TREE
repair_generator=cargo_metadata_unlocked_LC_ALL_C
repair_scope=Cargo.lock_only
qualification_status=NOT_CLAIMED
q2_status=NOT_CLAIMED
```

## Commit verification

After reviewing the generated lock diff, stage **only** `Cargo.lock` and create one repair commit.

Then run:

```bash
bash scripts/verify-cuf-v0.11-cargo-lock-repair.sh \
  /tmp/cuf-v0.11-parent-lock-repair \
  HEAD
```

The verifier requires:

- checksum-valid repair evidence;
- `LOCK_UPDATE_REQUIRED` precheck classification;
- successful Cargo generation and locked postcheck;
- unchanged manifest inputs;
- exact recorded base commit/tree;
- a one-parent repair commit whose parent is the evidence base;
- exactly one changed path: `Cargo.lock`;
- committed pre/post lock hashes exactly equal the evidence;
- the committed `Cargo.lock` unified diff is byte-identical to the Cargo-generated evidence diff;
- `git diff --check` passes.

A verifier PASS proves only:

```text
this commit is exactly the reviewed Cargo-generated lock repair
```

It does **not** prove:

- that the operator context label is historically correct;
- parent Q1 qualification;
- authored v4.8 Q1 qualification;
- physical correctness;
- continuation/replay correctness;
- Q2.

The base commit/tree and separate replay/qualification evidence establish the historical role of the repair.

## Failed authored replay preservation

Stage B needs an additional historical boundary before a post-authorship repair may begin.

If the exact retained v4.8 replay reaches qualification and fails Q1 while the evidence wrapper's independent integrity postconditions still PASS, the authored tree may be committed **only as an explicitly unqualified historical parent**.

Use:

```bash
bash scripts/verify-cuf-v0.11-authored-failure-preservation.sh \
  /tmp/cuf-v0.11-parent-evidence \
  HEAD
```

The preservation verifier requires:

- checksum-valid FAIL evidence;
- `POSTCONDITIONS.txt=PASS` despite the overall Q1 failure;
- exact retained v4.8 patch SHA-256;
- exactly 275 retained/staged paths before and after qualification;
- unchanged candidate HEAD, staged tree, Cargo.lock, repository status, parent composition, and qualification tooling during the failed run;
- a one-parent preservation commit whose parent is the evidence candidate HEAD;
- preservation commit tree exactly equal to the failed-but-intact staged tree;
- preservation commit changed-path set exactly equal to the retained 275-path artifact;
- committed Cargo.lock exactly equal to the lockfile used during the failed Q1 attempt;
- `git diff --check` passes.

It deliberately refuses a FAIL capsule whose **postconditions also failed**. A tree that changed unexpectedly during qualification is not a trustworthy authored historical root merely because the qualification command itself failed.

A preservation PASS means only:

```text
this commit exactly preserves the authored v4.8 tree that failed Q1
```

and explicitly means:

```text
Q1 = FAIL preserved / NOT_QUALIFIED
Q2 = NOT_CLAIMED
```

It is a repair parent, not a qualified promotion.

## Stage A lineage

If #118's clean-parent locked precheck fails:

```text
#118 execution candidate
      ↓
real parent LOCK_UPDATE_REQUIRED result
      ↓
prepare lock repair (context=parent)
      ↓
review generated Cargo.lock diff
      ↓
one Cargo.lock-only repair commit
      ↓
verify repair commit against capsule
      ↓
rerun locked parent qualification/readiness
      ↓
verified parent replay candidate
```

Only then should the retained v4.8 artifact be replayed.

## Stage B lineage

If the exact authored v4.8 replay later fails Q1 with intact evidence postconditions:

```text
verified parent
      ↓
exact authored 275-file replay
      ↓
Q1 FAIL + POSTCONDITIONS PASS
      ↓
commit exact authored tree, explicitly unqualified
      ↓
verify authored-failure preservation commit
      ↓
real v4.8 LOCK_UPDATE_REQUIRED result
      ↓
prepare lock repair (context=v4.8)
      ↓
review generated Cargo.lock diff
      ↓
one Cargo.lock-only repair commit
      ↓
verify repair commit against capsule
      ↓
full locked Q1 qualification
```

The Stage B lock repair must never be folded into the retained v4.8 patch or described as pristine authored Universal Matter v4.8.

## Non-goals

This tranche does not:

- edit Cargo.lock in GitHub;
- synthesize missing lock entries;
- automatically stage or commit a generated repair;
- treat `parent`/`v4.8` labels as lineage proof;
- promote a failed authored tree as qualified;
- preserve a failed tree whose integrity postconditions also failed;
- weaken `--locked` Q1 qualification;
- treat arbitrary Cargo failures as lock failures;
- modify Universal Matter v4.8;
- implement Q1 code fixes;
- implement Q2 continuation repairs;
- claim any qualification PASS.
