# Qualified Tree Promotion Contract v0.1

**Status:** Canonical release/evidence contract  
**Date:** 2026-09-01  
**Scope:** Reproducible promotion of staged scientific/simulation trees into Git commits

## Governing rule

**Qualification attaches to an exact Git tree, not to a branch name, working directory, or verbal PASS claim.**

A promoted commit is qualified only when its committed tree is byte-identical to the tree that passed the qualification gate.

## Required identities

A qualification capsule must bind at least:

- qualification parent HEAD;
- exact staged Git tree SHA;
- staged tree SHA after qualification;
- retained source artifact digest when an external patch/artifact is involved;
- lockfile identity before/after qualification;
- staged path count;
- repository status before/after qualification;
- toolchain/environment summary;
- full qualification output;
- PASS/FAIL result;
- checksum manifest for the capsule files.

## Pre-promotion rule

Before committing a qualified staged tree, verification must prove:

1. capsule checksum manifest is intact;
2. capsule status is exactly `PASS`;
3. retained source artifact identity is expected;
4. current HEAD equals the recorded qualification parent;
5. current staged tree equals the qualified staged tree;
6. staged tree did not change during qualification;
7. lockfile did not change during qualification;
8. current repository status still equals the qualified post-run status.

If any item differs, the candidate must be requalified.

## Promotion-commit rule

The code promotion commit must:

- have exactly one parent;
- use the recorded qualification parent as that parent;
- have a Git tree exactly equal to the qualified staged tree.

Commit metadata (author, committer timestamp, message, signature) may differ without changing the qualified tree identity.

A merge commit or a commit containing extra evidence/docs changes is not the qualified code promotion commit, even if it includes the same code.

## Post-promotion rule

After committing, verification must prove:

`commit^{tree} == qualified staged tree`

and:

`commit^1 == recorded qualification parent`

The committed lockfile must also match the capsule lockfile identity.

## Evidence commits

Evidence metadata may be committed after the qualified code commit as a separate child commit.

This preserves the distinction:

`qualified code tree -> code promotion commit -> evidence/receipt commit`

rather than contaminating the tree that was actually tested.

## Failure and repair

If the pristine authored tree fails qualification:

1. it may be committed as an explicitly unqualified replay parent;
2. fixes must land as later focused commits;
3. the repaired cumulative tree must be qualified again;
4. the final promoted qualified commit must still satisfy this tree-identity contract.

A historical commit existing in Git does not imply that it passed qualification.

## Universal Matter v4.8 application

For the retained Universal Matter v4.8 replay lane:

- retained patch SHA-256 is fixed by the integration gate;
- exactly 275 patch paths are expected;
- the qualification capsule is generated outside the worktree;
- `verify-universal-matter-v4.8-qualification-capsule.sh` proves the pre-promotion candidate;
- `verify-universal-matter-v4.8-promoted-commit.sh` proves the resulting commit identity.

## Generalization

This contract should be reused for future high-value deterministic/research lineages such as:

- Universal Matter releases;
- CUF qualification milestones;
- world-generation schema freezes;
- simulation/replay scientific evidence;
- renderer/capture qualification where a code tree is part of the evidence chain.
