# Universal Matter v4.8 Replay and Qualification Runbook

Status: operational handoff
Date: 2026-09-01

## Objective

Replay the retained Universal Matter v4.8 cumulative artifact onto the exact compatible CUF lineage, determine whether the authored tree itself passes local Rust gates, and preserve a clear evidence distinction if qualification fixes are required.

Qualification is attached to an exact Git tree. A branch name, commit message, or terminal PASS line is not sufficient evidence by itself.

## Inputs

Required retained artifact:

`SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE(1).patch`

Expected SHA-256:

`23f6baf3545bace49252eee190f181fa8a88c650d2994b72b65bdaf83cc74637`

Expected patch shape:

- 275 changed paths;
- 269 new files;
- 6 modified files;
- 0 deleted files.

## Phase A — non-mutating preflight

From the intended integration branch/worktree with a clean working tree:

`bash scripts/preflight-universal-matter-v4.8.sh /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch`

The preflight must verify:

- artifact checksum;
- patch shape;
- exact six preimages;
- absence of all new-file targets;
- `git apply --check`.

Failure here means **do not apply the patch**.

## Phase B — guarded replay

Run:

`bash scripts/apply-universal-matter-v4.8.sh /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch`

Expected state afterward:

- exactly 275 staged paths;
- six known postimage blob prefixes match;
- no commit has been created.

Inspect:

- `git status --short`
- `git diff --cached --stat`
- `git diff --cached --check`

## Phase C — pristine authored-tree qualification with evidence capture

Prefer the evidence-capturing wrapper rather than invoking the gate directly:

`bash scripts/capture-universal-matter-v4.8-qualification-evidence.sh /path/to/SYMTROPY_UNIVERSAL_MATTER_V4.8_CUMULATIVE.patch /tmp/um-v48-evidence`

The wrapper runs:

`nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf.sh`

and records an external checksum-manifested capsule containing the qualification parent, staged tree before/after, patch identity, Cargo.lock identity, toolchain, repository status, full logs, and PASS/FAIL result.

### Green path

If the pristine replay passes:

1. verify the capsule while the qualified tree is still staged on its recorded parent:
   `bash scripts/verify-universal-matter-v4.8-qualification-capsule.sh /tmp/um-v48-evidence`;
2. inspect the staged diff one final time;
3. commit **only** the qualified staged v4.8 code tree, with no evidence/docs side effects added to that commit;
4. verify the resulting commit against the capsule:
   `bash scripts/verify-universal-matter-v4.8-promoted-commit.sh /tmp/um-v48-evidence HEAD`;
5. record/tag/status that exact commit as the qualified v4.8 + CUF integration baseline;
6. add any repository-resident evidence/receipt metadata as a later child commit;
7. begin CUF v0.11 from the exact qualified code commit (or from a clearly documented child that preserves it as ancestor).

The promotion verifier requires:

- exactly one commit parent;
- the recorded qualification parent;
- `commit^{tree}` equal to the qualified staged tree;
- retained v4.8 patch identity;
- exactly 275 qualified patch paths;
- unchanged qualification-time repository status;
- unchanged Cargo.lock during qualification;
- committed Cargo.lock identical to the qualified lockfile.

This makes the relationship explicit:

`qualified staged tree -> code promotion commit -> optional evidence child commit`

## Phase D — red path

If the pristine authored replay fails compile/test/clippy:

**Do not edit the retained patch artifact. Do not call the authored v4.8 tree qualified.**

Preserve the distinction:

1. retain the checksum-manifested failing evidence capsule;
2. record the staged-tree identity and failing toolchain;
3. create an explicitly named **unqualified replay commit** containing only the exact 275-file authored patch, for example:
   `terrain: replay retained Universal Matter v4.8 (unqualified)`;
4. make every compile/test/clippy repair as one or more subsequent focused commits;
5. never squash those repair commits into the authored replay during qualification work;
6. rerun the complete v4.8 + CUF gate at the cumulative repair head with a new evidence capsule;
7. if green, record that repaired cumulative tree as the qualified integration baseline while retaining the unqualified authored-replay parent in history.

A Git commit records lineage; it does not itself assert scientific/software qualification. The status/evidence documents must state this explicitly.

For a repaired-green lineage the simple pristine-promotion parent rule no longer applies directly because the qualified tree is already committed across multiple repair commits. Record both the unqualified replay parent and final qualified repair head explicitly in the evidence receipt. Do not pretend the original authored replay commit passed.

## Cargo.lock rule

The pristine qualification gate rejects an unexpected `Cargo.lock` mutation.

If the replay genuinely requires a lockfile change:

1. retain the pristine gate failure evidence;
2. follow the red path;
3. commit the exact authored replay as unqualified;
4. regenerate/review `Cargo.lock` in a dedicated follow-up commit;
5. rerun all gates and create a new evidence capsule.

Do not fold an unexplained lockfile regeneration into the retained patch replay.

## Qualification repairs

Good repair commits are narrow and explain the failure they address, for example:

- `fix(terrain): update v4.8 Bevy 0.19 API call`
- `fix(terrain): satisfy clippy ownership lint in hydrology proof`
- `chore(lock): regenerate lockfile for v4.8 dependency features`

Avoid broad cleanup or unrelated refactoring until the retained lineage is qualified.

## Evidence packet

For either green path or repaired-green path, retain at minimum:

- patch SHA-256;
- base Git head/tree;
- exact qualified staged tree or final qualified repair tree;
- unqualified replay commit/tree if the red path was used;
- final qualified commit/tree;
- Rust/Cargo/Nix versions;
- `cargo fmt` result;
- `cargo test -p symtropy-terrain` result;
- `cargo clippy -p symtropy-terrain --all-targets -- -D warnings` result;
- CUF qualification result;
- exact test counts where available;
- `Cargo.lock` identity;
- repository diff/status hygiene result;
- capsule `MANIFEST.sha256`.

## v0.10.1 forcing-evidence relationship

CUF v0.10.1 is an independent core contract that distinguishes deterministic forcing from authority-backed observation.

If v0.10.1 is accepted before the v4.8 replay is qualified, the eventual combined integration head must rerun the full qualification gate with both lineages present. Do not assume two separately green branches imply a green combined tree.

## v0.11 start condition

Do not begin the production Universal Matter observation adapter on a merely authored or failing v4.8 tree.

The adapter branch must name an exact **qualified** v4.8 + CUF parent. It must also include the accepted deterministic-forcing contract if weather forcing is used.

This ensures later Living Watershed evidence cannot accidentally depend on an unqualified physical authority lineage or confuse model forcing with authoritative state.
