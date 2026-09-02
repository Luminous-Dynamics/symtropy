# CUF v0.11 Evidence Integrity — 2026-09-02

## Status

This tranche hardens the pre-Q2 parent-candidate qualification/evidence boundary from PR #111. It does not modify Universal Matter, Terrain, Basin, world runtime, persistence formats, continuation semantics, gameplay, or orbital/stellar simulation.

Tracked by #112. Related focused requirements are #114, #115, and #116.

## Problem closed

PR #111 binds the physical/forcing/continuation source lineage, but a trustworthy qualification capsule must also bind the tooling that produced the result and independently enforce the postconditions expected from nested gates.

Without that second layer, a changed helper could theoretically return exit 0 while weakening a tree/lock/status check. The capsule would record the resulting state, but its PASS decision would still depend too heavily on the nested wrapper.

## Added tooling identity gate

`scripts/verify-cuf-v0.11-qualification-tooling.sh`:

- resolves the exact committed candidate HEAD;
- enumerates every critical script used to preflight, apply, qualify, capture, and promote the parent candidate;
- requires each path to exist at the committed HEAD;
- rejects staged or working-tree differences from that HEAD;
- compares each working-tree Git blob with its committed Git blob;
- emits a stable `<blob>\t<path>` manifest for evidence capture;
- includes itself and the candidate-specific promotion verifier in the identity set.

The v4.8 patch may therefore be staged while qualification tooling remains provably identical to the committed candidate.

## Qualification wrapper hardening

`scripts/qualify-cuf-v0.11-parent-candidate.sh` now captures tooling identity before and after the existing gates and rejects:

- tooling drift;
- staged-tree drift;
- Cargo.lock drift;
- repository-status drift;
- unstaged tracked changes;
- untracked files.

This remains Q0/Q1 + continuation-core qualification only. Q2 is not claimed.

## Evidence wrapper hardening

`scripts/capture-cuf-v0.11-parent-candidate-evidence.sh` now independently records and compares:

- candidate HEAD before/after;
- staged tree before/after;
- Cargo.lock before/after;
- repository status before/after;
- staged path set before/after;
- retained patch path set vs staged path set;
- parent-composition proof before/after;
- qualification tooling blobs before/after;
- unstaged/untracked side effects.

The nested qualification exit code and these independent postconditions must both pass before `STATUS.txt` can contain `PASS`.

New evidence files include:

- `BASE_HEAD_AFTER.txt`;
- `STAGED_PATHS_AFTER.txt`;
- `PARENT_COMPOSITION_AFTER.txt`;
- `TOOLING_BLOBS_BEFORE.txt`;
- `TOOLING_BLOBS_AFTER.txt`;
- `POSTCONDITIONS.log`;
- `POSTCONDITIONS.txt`.

All are bound by `MANIFEST.sha256`.

## Promotion boundary

`scripts/verify-cuf-v0.11-parent-promotion.sh` verifies that a committed promotion is exactly the tree proven by a PASS parent-candidate capsule.

It requires:

1. valid evidence `MANIFEST.sha256`;
2. `STATUS.txt == PASS`;
3. `POSTCONDITIONS.txt == PASS`;
4. exact retained v4.8 patch SHA-256;
5. exact 275-path patch/staged equality before and after;
6. unchanged Cargo.lock, repository status, composition proof, and tooling blob manifest;
7. lineage binding to the recorded candidate HEAD/tree;
8. `qualification_level=Q0/Q1_plus_continuation_core_only`;
9. `q2_status=NOT_CLAIMED`;
10. a one-parent promotion commit;
11. promotion first parent equal to the evidence candidate HEAD;
12. promotion commit tree equal to the qualified staged tree;
13. promoted Cargo.lock equal to the qualified lockfile.

Promotion therefore creates a committed parent for #76/#79/#81 repairs without converting Q0/Q1 evidence into a false Q2 claim.

## Intended execution order

1. checkout the evidence-integrity descendant of #111;
2. run the existing v4.8 preflight;
3. apply/stage the retained 275-path v4.8 artifact;
4. capture the hardened parent-candidate evidence capsule;
5. require PASS plus PASS postconditions;
6. create exactly one promotion commit containing the qualified staged tree;
7. verify that commit with `verify-cuf-v0.11-parent-promotion.sh`;
8. only then branch post-replay Q2 hardening (#76/#79/#81).

## Non-claims

This tranche does not claim:

- that Universal Matter v4.8 has successfully replayed;
- that Rust/Nix/Terrain qualification has passed;
- Q2 continuation/replay closure;
- Q3 native CUF authority integration;
- production-ready groundwater recharge semantics (#77 remains separate).

The architecture freeze remains in force. Further changes should be driven by actual qualification/replay failures or the already-identified Q2 authority continuation gaps.
