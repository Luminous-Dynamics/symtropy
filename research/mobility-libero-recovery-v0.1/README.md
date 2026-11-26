# LIBERO-Recovery corpus adapter v0.1

**Scope:** import the Apache-2.0 ModelScope LIBERO_Recovery_FailureScenes release as an artifact-backed RRVR corpus. This is not a policy reproduction and does not import the separate raw-rollout or expert-demonstration datasets.

## Pinned source snapshot

- Code repository: https://github.com/liulin815/LIBERO-Recovery
- Upstream code revision inspected: 832d54dfc43f0c9e4240bb8d92f5fda7001bb093 (main, commit date 2026-10-22).
- Dataset: https://www.modelscope.cn/datasets/ataier/LIBERO_Recovery_FailureScenes
- Dataset-card license declaration: Apache License 2.0.
- The GitHub repository metadata has no declared license and no root LICENSE file. The import uses only the ModelScope failure-scene dataset. It does not copy upstream code.
- The separate LIBERO_Recovery_Rollouts (1,446 NPZ files / ~80 GB) and expert-demo datasets are outside v0.1. Their terms must be checked independently before import or redistribution.

The importer records the actual ModelScope dataset checkout commit and the exact upstream code revision in every release manifest. If the dataset repository cannot supply a revision identity, the import is marked incomplete; do not silently substitute the GitHub code commit for the dataset commit.

## Do not conflate entity counts

The upstream GitHub README currently reports:
- 4,803 annotated failure-scene snapshots across five suites;
- 4,338 VLM-annotated scenes plus 465 constructed L4 scenes; the L4 total is 474 including nine natural L4 scenes;
- 4,803 rows in selection.jsonl; 465 rows in selection_l4.jsonl;
- 26,710 inventory entries in scenes_index.jsonl;
- 1,446 raw rollout NPZ files (separate 80 GB release);
- 3,184 human recovery demonstrations across 413 failure scenarios and 625,731 frames at 20 Hz on the project page.

The project page advertises 2,178 recovery scenarios / 130 subtasks / 16 evaluation dimensions, while the arXiv abstract says 1,000+ scenarios. Those are not automatically inconsistent; they are different source surfaces and possibly different entity definitions. The import report keeps separate counts for annotation scene rows, unique replayable BDDL/state-file pairs, source rollout identities referenced by annotations, unique scenario/source-episode identities, inventory-only scene-index rows, natural and constructed L4 scenes, human demonstration references (not imported in v0.1), and raw NPZ availability (external/not imported in v0.1).

No 2,178 or 4,803 headline is carried as a verified imported denominator until the actual release files are read and reconciled.

## Why annotation JSON is not proof of replayability

The official evaluation script recursively discovers .bddl files and pairs each with a sibling _sim_state.npy. It does not use the annotation index to execute the evaluation. make_level_subset.py creates symlink trees for the BDDL/state pair. Therefore v0.1 verifies the actual bytes of both artifacts. An annotation row with no valid BDDL/state pair remains in the reconciliation report as non-replayable/invalid; it is not silently dropped or counted as an executable scene.

## Preserved provenance

The release manifest and import report retain original scene IDs and suite/source names; source model label, run ID, episode index, per-task trial index, task text, step and timestamp; stage (pre-fail / mid-fail / post-fail), original failure steps, level L1-L4, confidence, reason, VLM model; constructed L4 source scene, injection strategy, object pair, placement parameter, seed, settle steps, contact/leaning validity measurements and original injected annotation bytes; exact raw selection, L4 selection, scenes-index, injection-provenance, annotation, BDDL and simulation-state artifact digests; external raw NPZ reference and explicit availability status.

Unknown upstream JSON fields are preserved byte-for-byte in the source artifacts and losslessly in the raw JSON value. The normalized fields are a view, not a replacement for the original annotation.

## Run

After downloading only LIBERO_Recovery_FailureScenes to an isolated directory and retaining its ModelScope Git revision:

cargo fmt --manifest-path research/mobility-libero-recovery-v0.1/Cargo.toml -- --check

cargo test --manifest-path research/mobility-libero-recovery-v0.1/Cargo.toml --locked

cargo clippy --manifest-path research/mobility-libero-recovery-v0.1/Cargo.toml --locked --all-targets -- -D warnings

cargo run --manifest-path research/mobility-libero-recovery-v0.1/Cargo.toml --locked --bin libero_recovery_import -- DATASET_ROOT DATASET_GIT_REVISION UPSTREAM_CODE_REVISION OUTPUT_RELEASE_MANIFEST.json

The CLI performs read-only validation and emits a release manifest/report; it does not modify source files, create symlinks, run LIBERO, start a policy, or execute robot actions.

## RRVR overlay

Host metrics remain native and separate: recovery success, recovery level, number/strength of intervention and elapsed recovery time. RRVR adds independent provenance, decision cut, observation quality, possible-history support, false-finality, effect reconciliation, safe-retry admissibility, probe cost and uncertainty coverage. No weighted scalar trades hard safety against task utility.

If the raw source NPZ is not imported, the scene can still be replayable from BDDL + state snapshot, but action-prefix fidelity is not established. The RoboRecover three-attempt/five-frame SSIM gate remains a separate qualification.

## Evidence tiers

- T0: schema, synthetic fixture, raw-byte digest and path-safety tests.
- T1: actual downloaded failure-scene corpus with independent count/digest reconciliation.
- T2: exact host LIBERO load-path replay with pinned environment/checkpoint and preserved rollout outputs.
- T3: preregistered simulator recovery campaign and RRVR overlay.
- HIL/physical work remains separate.

No reproduction, physical safety, SOTA superiority or statistical significance claim follows from an import validator pass.
