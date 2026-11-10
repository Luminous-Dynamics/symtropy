# RoboCasa-Mem current-release qualification gate v0.1

**Status:** typed Rust preflight contract with synthetic tests. No upstream wheel was built or installed by this change. No RoboCasa-Mem task/policy was executed and no benchmark score is claimed.

## 1. Why this gate exists

A source audit, a successful historical wheel test, a candidate asset verification receipt and a current release qualification are different evidence types. They must not collapse into one boolean.

A changed Python package/runtime invalidates the applicability of a historical installed-wheel pass. An asset archive inventory does not prove a minimal task dependency closure. A policy-family smoke does not qualify other policy checkpoints. Rights and training-overlap disclosure are independent of technical reproducibility.

This module defines the conditions required before a native RoboCasa-Mem policy evaluation may begin. It is a *release/evaluation preflight*, not an evaluator, simulator adapter, policy runner or physical-safety mechanism.

## 2. Upstream source audit — pinned snapshot

Repository: https://github.com/SiHhangChen/RoboCasa-Mem

Inspected source revision: `2c7d498e589d1a5a62b5b30c9d786513fa86bfa2`

Release candidate metadata:
- `release_candidate.json` blob: `1b1158019e31e8959dd0f6dddc52c22d77ac5b4e`
- candidate package: `0.1.0rc1`
- declared candidate source revision: `b891bdf35fbe7b8390ea6cc663a870fb9ea7820c`
- `release_ready=false`
- `suite_bytes_preserved=false`
- 27 official evaluation suites across nine task IDs and three policy families;
- 450 resolved evaluation input identities;
- seeds 1050–1099;
- `historical_full_model_rollouts_rerun=false`.

The declared source revision `b891bdf...` differs from the inspected source revision `2c7d498...`. The release must reconcile and freeze one exact source tree before native evaluation. A candidate manifest naming a different source commit is not silently treated as evidence for the inspected source.

Evaluation input lock:
- `configs/benchmark/evaluation_input_lock.json` blob: `ea23daa72b86c7e34bfd9af199b456c5a6693d2e`
- status `candidate_inputs_verified`;
- `release_ready=false`;
- 9 tasks, 27 suites, 450 input identities.

Suite source manifest:
- `configs/benchmark/evaluation_suite_sources.json` blob: `2ee8c16a9deb01661d10071f2e9575126970cb5d`
- 27 policy/task configuration entries; each task/family entry uses 50 seeds (1050–1099).

### Asset evidence has two different assertions

The release-candidate manifest contains a nested receipt:
- status `verified_for_all_450_official_inputs_in_isolated_installation`;
- archive SHA-256 `140176b06cf220f6d11440fe3cede54cca7577edf13473d62fb210bd03ca68e3`;
- 3,035 files;
- receipt SHA-256 `9a3ad2e984c6a6c72938cdf9b65919725936c87dd90f7d4a0dd0f40c2ed878d2`;
- audit SHA-256 `6fab308ef817abd26ca042f26a57b814c87ba40f844d7a3f3b1631f969f63929`;
- nested receipt says nine-task coverage verified;
- nested receipt says clean candidate installation is **not** verified.

But the referenced distributed `configs/benchmark/simulator_assets.json` is a separate artifact:
- current blob: `9c1f7643fc19fe9e944580bf05053da2f05bf723`;
- its own scope says `nine_task_coverage_verified=false`;
- its note says the archive is an existing inventory, not a minimal nine-task dependency closure;
- its `clean_candidate_installation_verified=false`.

These two facts are deliberately preserved separately. The release gate requires both the asset-manifest scope and an independent coverage receipt, then also requires a clean installation of the exact candidate. It does not choose the more optimistic assertion.

### Installed-wheel evidence is historical

The release-candidate metadata describes an earlier acceptance:
- accepted wheel SHA-256 `f19e89795ccb904de20e221595daf88f160b95fbeca7300a31ec50b52c600c8c`;
- 450 official task/seed input cases, three hold-action steps each;
- `runtime_files_match_accepted_wheel=false`;
- status `historical_passed_runtime_changed`;
- no model inference in that installed-wheel acceptance.

The README also describes earlier source-installation and model smoke checks, but those are scoped to their recorded environment and runtime. They do not qualify the changed release candidate. The current wheel/runtime must be rebuilt, pinned and accepted again.

### Rights and training overlap

The package code declares MIT, while RoboCasa/robosuite dependencies, simulator assets and checkpoints have separate terms. A code license is not an asset/checkpoint/data license. The benchmark README says the asset/checkpoint terms must be checked at their source. A native comparative run must carry explicit dataset, asset and checkpoint rights evidence. Training-data overlap with evaluation tasks must also be disclosed. Unknown rights or unknown overlap block a qualified comparison; known overlap remains visible and is not relabeled as independence.

## 3. Rust qualification contract

Implementation:
`experiments/robocasa-mem-bce-contract/src/release_gate.rs`

Export:
`pub mod release_gate;`

The typed evidence record includes:
- release ID/version and exact candidate/manifest source commits;
- release, source-file, suite, evaluation-input, simulator-source, dataset, dataset-rights, asset-rights, asset-verification and policy-family manifest digests;
- explicit source-file-manifest verification status;
- expected/observed task, suite and input counts;
- suite-byte preservation and release-ready attestation;
- separate asset-manifest and independent-receipt coverage flags;
- asset archive digest/availability and clean candidate install status;
- dataset/asset/checkpoint rights status and training-overlap disclosure;
- candidate wheel and runtime digests versus the exact wheel/runtime exercised by acceptance;
- expected checkpoint identity for every `(policy family, task)` suite;
- one model inference smoke record for every expected pair.

The 27 expected policy-run pairs correspond to nine tasks × three families: ACT, Diffusion Policy and Pi0.5. A smoke of ACT on one task does not unlock the other ACT task suites or any other family. Each expected pair has a frozen checkpoint digest and its smoke must match that digest and the candidate runtime.

Model inference smoke is intentionally separate from task success. For each pair it checks that the checkpoint loaded, inference requests occurred, no runtime errors were recorded and episode artifacts are complete. The benchmark evaluator—not the release gate—later determines task success and stage completion.

### Qualification stages

The report can independently mark:
1. source identity audited;
2. evaluation inputs frozen;
3. dataset provenance qualified;
4. simulator asset closure qualified;
5. current wheel accepted;
6. model inference smoke qualified;
7. native evaluation ready.

A stage is included only when its declared evidence checks pass. All blockers are retained as a sorted set. `native_evaluation_ready` is true only when no blocker remains.

The current upstream metadata should not pass this gate. It has source identity/suite-byte/release-ready issues, asset-manifest and clean-install issues, historical wheel/runtime mismatch, and missing current per-suite inference evidence. Depending on rights/overlap manifests, additional blockers remain.

## 4. Statistical and benchmark implications

A technically reproducible release is necessary but does not establish SOTA. After the gate passes, each policy/family/task benchmark must still:
- use the official evaluator and pinned seed/input manifest;
- run full model inference, not just input resolution or hold-action initialization;
- preserve every attempted episode and stage receipt;
- report task success (SR) separately from stage completion (SCR);
- disclose checkpoint provenance, training overlap, model calls, assistance, resources and runtime errors;
- compare against matched official baselines on identical task/seed/evaluator configurations.

For Luminous's own experiments, preserve paired episode IDs and independent scene/seed units, use held-out calibration splits, and report intervention/abstention and operational faults alongside SR/SCR. Never present a successful release-preflight report as a benchmark score.

## 5. Test boundary

The synthetic release-gate tests exercise blocker logic:
- historical wheel/runtime mismatch;
- source revision mismatch and unpreserved suite bytes;
- different asset-manifest and receipt assertions;
- missing clean candidate installation;
- wrong official case count or failed cases;
- unknown/restricted rights and unknown training overlap;
- missing source verification receipt;
- all 27 task/family checkpoint tuples, duplicate/missing tuples and checkpoint digest mismatch;
- checkpoint loaded/inference request/runtime-error/artifact completeness checks;
- deterministic blocker ordering and release-ready attestation.

They do not hash/authenticate input artifacts, verify licensing, establish truth of upstream verification receipts, build a wheel, install a simulator, run a policy, or reproduce the paper. The adapter supplying `RoboCasaMemReleaseEvidence` remains responsible for hashing the named files and sourcing independent verification records.

## 6. Next execution sequence

1. Reconcile the release manifest's declared source commit with the exact checked-out source.
2. Preserve/verify the 27 suite bytes and freeze the 450 input identities.
3. Reconcile the asset manifest's own coverage status with the independent asset receipt and build a clean, candidate-specific installation.
4. Obtain explicit dataset, asset and checkpoint rights manifests; disclose training overlap.
5. Pin the expected checkpoint SHA for every one of the 27 policy/task suites.
6. Build the current wheel and runtime, then repeat all 450 installed-wheel acceptance cases against those exact digests.
7. Run one checkpoint-loaded inference smoke for every policy/task tuple using that exact runtime.
8. Only then run full policy rollouts and compare SR/SCR.

No runner wait is required to improve the evidence contract. The actual upstream install and policy runs remain external qualification work and cannot be fabricated by synthetic Rust tests.
