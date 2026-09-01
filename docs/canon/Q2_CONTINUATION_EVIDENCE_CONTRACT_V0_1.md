# Q2 Continuation Evidence Contract v0.1

**Status:** design freeze; implementation pending Q0/Q1 Universal Matter replay  
**Qualification layer:** Q2 — continuation/replay correctness  
**Tracks:** #84; consumes #76/#79/#81/#82/#83/#85/#86/#87/#88/#89/#90

## 1. Purpose

Q1 answers whether the exact authored tree builds, tests, and lints under a recorded environment.

Q2 answers a different question:

> Does the promoted tree preserve every continuation-significant state component required for exact replay/suspend-resume semantics, and can that claim be reproduced from a machine-checkable evidence capsule tied to the exact Git tree?

A Q1 PASS is not a Q2 PASS.

## 2. Q2 evidence unit

A Q2 result is one immutable evidence capsule associated with exactly one qualified Git tree.

The capsule must record enough information for an independent reviewer/tool to determine:

- what code was tested;
- what environment was used;
- what continuation contract version was tested;
- which fixtures ran;
- which canonical digests/checkpoints were observed;
- whether the repository mutated during qualification;
- whether the promoted tree is the same tree that passed.

## 3. Required capsule contents

A canonical capsule directory should include equivalents of:

```text
STATUS.txt
QUALIFICATION_LAYER.txt
CONTRACTS.txt
LINEAGE.txt
GIT_PARENT.txt
GIT_TREE.txt
GIT_HEAD.txt
TOOLCHAIN.txt
SYSTEM.txt
CARGO_LOCK.sha256
WORKTREE_BEFORE.txt
WORKTREE_AFTER.txt
TEST_MATRIX.tsv
CHECKPOINTS/
GOLDEN_VECTORS/
LOGS/
MANIFEST.sha256
```

Exact filenames may evolve before implementation, but the semantic contents are frozen below.

## 4. Tree identity

The capsule binds:

- tested parent commit;
- tested Git tree;
- optional authored replay commit when Q1/Q2 are layered;
- promoted commit/tree after qualification;
- proof that the promoted code tree equals the tested code tree.

Evidence metadata may be committed in a later child commit, but it must not change the claim that the qualified code tree itself was the tested tree.

## 5. Environment identity

Record at minimum:

- `rustc --version --verbose`;
- `cargo --version`;
- Nix version and flake/develop identity where available;
- host architecture/OS;
- relevant deterministic feature flags;
- `Cargo.lock` SHA-256;
- any required sibling/private dependency HEAD/tree identities;
- explicit environment variables that materially alter deterministic tests.

Secrets must not be written into evidence.

## 6. Required Q1 regression

Q2 is cumulative.

A Q2 capsule must include or reference a Q1 regression result for the same final code tree.

If Q2 repair commits change code after the pristine authored Q1 run, the final repaired tree must rerun the applicable format/test/clippy/build gates.

Historical pristine Q1 evidence remains valuable but cannot substitute for regression on the repaired Q2 candidate.

## 7. Continuation identity test matrix

### 7.1 Hydrology

Required after #76/#79:

1. create authorities with equal sparse water physical state;
2. vary only active frontier;
3. require equal physical water digest;
4. require unequal continuation digest;
5. checkpoint/restore both and preserve continuation identity;
6. construct a bounded fixture where differing active frontiers can select different next work;
7. require equal complete continuation identity + equal Matter/input/dt/work policy to produce equal next report/state.

### 7.2 Thermal

Repeat the same pattern for thermal state vs active frontier.

### 7.3 Active lava

Required after #79:

1. equal lava store;
2. different `next_step`;
3. equal lava physical-state digest;
4. unequal continuation digest;
5. checkpoint/restore preserves continuation digest;
6. a symmetric/tie fixture proves step-sensitive routing can diverge;
7. equal continuation identity + equal Matter/morphology/dt/work policy yields equal next result.

### 7.4 Active eruption

The enclosing eruption session/checkpoint identity must bind nested lava continuation state.

Changing only nested `next_step` must not pass as an identical exact-continuation session.

### 7.5 Landscape integration residuals

If #81 is within the promoted runtime scope:

- accumulate sub-threshold waterlogging/decay exposure;
- suspend/restore;
- continue;
- compare with uninterrupted execution;
- require equal canonical landscape state and continuation identity at checkpoints;
- reject/reset behavior that silently loses residual progress.

## 8. Derived/cache reconstruction matrix

Q2 must explicitly test important omitted runtime state rather than assuming it is harmless.

### 8.1 Structural indexes

For derived indexes such as structural node-by-cell lookup:

- restore from canonical snapshot without persisted index;
- rebuild deterministically;
- prove public lookup/behavior equivalence;
- reject malformed canonical state that makes index reconstruction ambiguous.

### 8.2 Dirty/cache rebuild

For dirty-region/render/collision caches treated as rebuildable:

- restore without them;
- rebuild or conservatively invalidate;
- prove authoritative state does not change;
- prove stale cached presentation/collision is not retained as if current.

### 8.3 Residency

After #86, either:

- restore continuation-critical representation residency state exactly; or
- reconstruct it from an explicitly digest-bound policy and prove identical residency decisions.

## 9. World continuation manifest tests

Once #83 is implemented, Q2 requires manifest-level tests.

### 9.1 Canonicality

- fixed golden vectors from #88;
- entry order independence;
- child order independence;
- option/count encoding sensitivity;
- one-field perturbation vectors;
- duplicate/conflicting entries rejected.

### 9.2 Hierarchy

- child change changes every ancestor digest to the root;
- unchanged child manifests can be reused/deduplicated;
- a child subtree can be independently verified;
- parent/child authoritative ownership conflict fails closed.

### 9.3 Snapshot artifacts

- verify content before decode;
- semantic digest mismatch fails after decode but before adoption;
- unknown required schema/codec fails;
- storage-path changes alone do not alter semantic continuation identity;
- migration requires explicit receipt and new artifact identity.

### 9.4 Same-world resume

```text
suspend manifest A
restore all required domain artifacts
rebuild declared rebuildable state
recompute manifest B
require A == B
```

No simulation step occurs between A and B.

### 9.5 Fork

- fork receives distinct `WorldInstanceId`;
- ancestry references parent manifest;
- identical content-addressed child snapshots may be shared at fork genesis;
- fork cannot validate as same-world continuation.

## 10. Behavioral suspend/resume proof

Q2 requires at least one bounded multi-domain deterministic continuation fixture.

The canonical shape is:

```text
initial state S0

reference:
  advance N steps
  checkpoint CN
  advance M steps
  checkpoint CREF

resumed:
  advance N steps from S0
  checkpoint CN2
  require CN == CN2
  suspend
  restore
  require continuation manifest unchanged
  advance M steps
  checkpoint CRESUME

require CREF == CRESUME
```

The compared checkpoint should include relevant continuation identities, not merely visible metrics.

The first suitable Q2 fixture may be small and synthetic. Q4 remains responsible for the larger Living Watershed end-to-end causal vertical slice.

## 11. Inactive-time policy tests

After #85:

### Paused

Host/wall-clock delay has no effect on simulation state.

### Deterministic catch-up

Equal source continuation identity + equal time policy + equal forcing + equal target instant yields equal result regardless of host scheduling.

If work is chunked:

```text
catch_up(source, target, budget A across K calls)
```

must produce the same final authoritative identities as another legal scheduling partition when the policy declares budgets scheduling-only.

### Interrupted catch-up

Checkpointing partway through a long catch-up and resuming must equal uninterrupted catch-up.

### Coarse evolution

Any analytical/coarse advance requires the declared domain representation/equivalence receipt.

## 12. Deterministic forcing tests

Where stateless forcing affects the fixture:

- record model identity/config/seed/cursor;
- prove same forcing context gives repeatable samples;
- changing forcing identity/config changes the manifest/evidence basis;
- missing forcing context fails closed;
- forcing evidence is not mislabeled as authority state.

## 13. Causal journal tests

If a canonical journal head is part of Q2, use the serializer-independent v2 event identity tracked by #82.

The current v1 JSON/serde chain may still be tested for its own integrity semantics, but its head must not be silently relabeled as a cross-version canonical event identity.

## 14. Worktree purity

Qualification fails if unexpected repository mutation occurs.

At minimum compare before/after:

- tracked unstaged changes;
- staged changes;
- untracked files;
- `Cargo.lock` identity;
- generated source/evidence inside the worktree.

Evidence output should be directed outside the source tree unless an explicit later evidence-commit step is being performed.

## 15. Skips and ignored tests

A required Q2 fixture may not pass merely because it is marked ignored/skipped.

The Q2 runner should maintain an explicit list of required test names or a machine-readable test registry.

If a required fixture is missing, renamed unexpectedly, skipped, filtered out, or reports zero executed cases, qualification fails.

## 16. Test matrix format

`TEST_MATRIX.tsv` or equivalent should record at least:

```text
id	layer	domain	fixture	result	evidence_ref
```

Example semantic IDs:

```text
Q2-HYDRO-001
Q2-THERMAL-001
Q2-LAVA-001
Q2-ERUPTION-001
Q2-LANDSCAPE-001
Q2-CACHE-001
Q2-MANIFEST-001
Q2-RESUME-001
Q2-TIME-001
```

IDs should remain stable after introduction; semantics may be superseded only by an explicit contract version change.

## 17. Checkpoint evidence

The capsule should retain compact machine-readable checkpoint identity files for the main behavioral tests.

A checkpoint record should bind:

- fixture ID;
- simulation instant;
- world continuation manifest digest where available;
- relevant authority physical-state digests;
- relevant continuation digests;
- lineage/journal head where required;
- report/receipt digests where useful.

Do not rely only on log prose.

## 18. PASS rules

A Q2 PASS requires all of:

1. exact tree/environment recorded;
2. final repaired tree passes cumulative Q1 gates;
3. all required Q2 fixtures execute and pass;
4. hidden-state sensitivity tests pass;
5. continuation-sufficiency tests pass;
6. snapshot/restore tests pass;
7. cache/rebuild tests pass;
8. world manifest tests pass once required by the candidate contract;
9. worktree/lockfile remains within declared invariants;
10. evidence capsule checksum manifest verifies;
11. promoted code tree equals tested code tree.

One failed required condition means Q2 FAIL.

## 19. FAIL is evidence

A Q2 FAIL capsule is still valuable.

It should preserve:

- exact tested tree;
- failing fixture(s);
- logs;
- environment;
- checkpoint mismatch data;
- capsule checksum.

Do not delete or rewrite a meaningful failure merely to present a green lineage.

A repair creates a new candidate tree and a new evidence capsule.

## 20. Promotion

After PASS:

1. verify the qualified tree before commit/promotion if qualification used a staged tree;
2. commit/promote exactly that code tree;
3. verify post-promotion tree equality;
4. store/reference the evidence capsule;
5. only then mark the tree Q2-qualified.

Evidence metadata may live in a later child commit, but Q2 status always names the exact qualified code tree.

## 21. Relationship to Q3/Q4/Q5

### Q3

CUF v0.11 native Universal Matter adapters run on the Q2-qualified parent and prove cross-domain observation/forcing integration.

### Q4

Living Watershed proves a real bounded causal vertical slice across native physical authorities, CUF relevance/provenance, Basin/ecology response, receipts, suspend/resume, and replay.

### Q5

Planetary/stellar qualification stresses scale, long time horizons, inactive-body evolution, hierarchical manifests, numerical stability, and performance budgets.

Q2 is the bridge that makes those later claims meaningful rather than merely reproducible-looking.
