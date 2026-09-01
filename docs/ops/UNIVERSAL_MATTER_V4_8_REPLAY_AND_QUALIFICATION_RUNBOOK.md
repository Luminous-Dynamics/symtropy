# Universal Matter v4.8 Replay and Qualification Runbook

Status: operational handoff
Date: 2026-09-01

## Objective

Replay the retained Universal Matter v4.8 cumulative artifact onto the exact compatible CUF lineage, determine whether the authored tree itself passes local Rust gates, and preserve a clear evidence distinction if qualification fixes are required.

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

## Phase C — pristine authored-tree qualification

Run:

`nix develop --command bash scripts/qualify-universal-matter-v4.8-cuf.sh`

### Green path

If the pristine replay passes:

1. capture stdout/stderr as qualification evidence;
2. record Rust/Cargo/Nix/toolchain identity;
3. record the staged tree identity;
4. commit the exact qualified replay;
5. tag/status it as the qualified v4.8 + CUF integration baseline;
6. begin CUF v0.11 from that exact head.

The resulting commit is both the authored v4.8 replay and the qualified baseline.

## Phase D — red path

If the pristine authored replay fails compile/test/clippy:

**Do not edit the retained patch artifact. Do not call the authored v4.8 tree qualified.**

Preserve the distinction:

1. save the full failing gate output;
2. record the staged-tree identity and failing toolchain;
3. create an explicitly named **unqualified replay commit** containing only the exact 275-file authored patch, for example:
   `terrain: replay retained Universal Matter v4.8 (unqualified)`;
4. make every compile/test/clippy repair as one or more subsequent focused commits;
5. never squash those repair commits into the authored replay during qualification work;
6. rerun the complete v4.8 + CUF gate at the cumulative repair head;
7. if green, record that repaired head as the qualified integration baseline while retaining the unqualified authored-replay parent in history.

A Git commit records lineage; it does not itself assert scientific/software qualification. The status/evidence documents must state this explicitly.

## Cargo.lock rule

The pristine qualification gate rejects an unexpected `Cargo.lock` mutation.

If the replay genuinely requires a lockfile change:

1. retain the pristine gate failure evidence;
2. follow the red path;
3. commit the exact authored replay as unqualified;
4. regenerate/review `Cargo.lock` in a dedicated follow-up commit;
5. rerun all gates.

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
- unqualified replay commit/tree if the red path was used;
- final qualified head/tree;
- Rust/Cargo/Nix versions;
- `cargo fmt` result;
- `cargo test -p symtropy-terrain` result;
- `cargo clippy -p symtropy-terrain --all-targets -- -D warnings` result;
- CUF v0.10 qualification result;
- exact test counts where available;
- `Cargo.lock` identity;
- repository diff hygiene result.

## v0.11 start condition

Do not begin the production Universal Matter observation adapter on a merely authored or failing v4.8 tree.

The adapter branch must name an exact **qualified** v4.8 + CUF parent. This ensures later Living Watershed evidence cannot accidentally depend on an unqualified physical authority lineage.
