# CIVDYN HumanStudy reproduction adapter v0.2

This work implements a source-pinned **receipt and conformance boundary** for issue [#3786](https://github.com/Luminous-Dynamics/symtropy/issues/3786). It is not yet an importer for either upstream runner and does not claim a completed empirical reproduction.

## Two source protocols — never merge their leaderboards

| Profile | Repository and exact source | Study count | Native result contract |
|---|---|---:|---|
| `HSB-PAPER-12-CF05D330` | `AISmithLab/HumanStudy-Bench`, commit `cf05d330fc315168c8c8e23b4acd882f91bb7408`, root tree `9b56c62406fb6b77d87d5032c3c6906583aca24e` | 12 | Original paper pipeline |
| `HSB-COMMUNITY-16-2026-11-07` | `HumanStudy-Hub/HumanStudy-Bench`, commit `b34d214b980be13ec5944a4d570a38bc4047b0f9`, root tree `26851c1d6564c78b25600afc7de8ec435e4aa3b8` | 16 | Community playground pipeline |

The paper profile pins `data/studies/study_001`–`study_012`, each study's data tree, and its original config/evaluator blob SHAs. It also pins the global runner, evaluator, scorer, stats library and metric documentation in [foundational_12_profile.json](foundational_12_profile.json).

The Community Edition is a separate, evolving repository. Its current snapshot contains `study_001`–`study_016`. A result from one profile must not be labelled as a result from the other.

## Native artifact contracts

### Original paper pipeline

The pinned paper repository documents result directories under:

```text
results/
  runs/<run_name>/<study_id>/<model>_<prompt>/
    full_benchmark.json
    detailed_stats.csv
    evaluation_results.json
```

The receipt requires these exact native files, relative to a single study/config run directory:

- `full_benchmark.json` — run payload, individual simulation records and usage metadata.
- `detailed_stats.csv` — detailed per-test statistical output.
- `evaluation_results.json` — official evaluator and paper metric output.

`raw_responses.json`, where present, is an additional source artifact and should be recorded separately. The aggregate files `simple_summary.md`, `simple_studies.csv`, and `simple_findings.csv` are derived reporting views, not substitutes for the per-run outputs. Hash original file bytes before parsing; never hash a re-serialized JSON object and call it the source digest.

### Community playground

The pinned community runner writes these files:

```text
<run-dir>/
  run.json
  progress.json
  output/
    profiles.json
    responses.json
    evaluation.json
    analysis.json
    transcript_sample.json
    charts.json
    charts.default.json
```

The receipt requires:
- `run.json` — run configuration and provenance.
- `output/responses.json` — participant-response payloads passed to the study evaluator.
- `output/evaluation.json` — study-native evaluator output.
- `output/analysis.json` — community-normalized test/finding analysis.

Profiles, transcripts, progress and chart artifacts may be recorded as additional artifacts. Charts are presentation outputs, not score authority. The normalized `analysis.json` contains community fields such as replication rate, direction-match rate and effect gaps. It is **not** automatically the paper benchmark's PAS/ECS result.

## Metric contract and an upstream specification discrepancy

For the paper profile, PAS/ECS fields are accepted only from `evaluation_results.json` (or another official paper-run result artifact) and are never recomputed by this adapter. The metric contract ID binds the receipt to the exact source profile.

The audit found a discrepancy that must remain visible:
- `docs/PAS_ECS_math.md` describes effective-sample-size weighting in raw PAS pooling.
- In the pinned `src/evaluation/stats_lib.py`, `aggregate_finding_pas_raw()` actually uses an unweighted mean of Fisher-z-transformed test PAS values; `aggregate_study_pas()` averages finding-level z values before inverse-transforming.
- The pinned implementation's main ECS is weighted Lin's concordance correlation coefficient (CCC) over standardized human/agent effect profiles.
- `docs/ECS_vs_PAS_explanation.md` describes an older Z-difference/RMS ECS and conflicts with the main metric documentation and current `compute_ecs_corr()` implementation.

For a faithful reproduction, the pinned executable implementation is the authority; the documentation/code disagreements are reported as specification debt. A future corrected metric is a new metric version, not a silent change to the reproduction path. See [metric_spec_audit.md](metric_spec_audit.md).

The Community Edition receipt allows PAS and ECS to be absent. No PAS/ECS value is inferred from `replicationRate`, `studyScore`, significance counts or effect-correlation fields.

## What the Rust gate enforces

- Exact source repository, commit, root tree and named protocol/metric contract.
- Complete study inventory and each study's pinned data tree.
- Run-level and study-level model/config/prompt/analysis digests.
- Native artifact roles, relative paths and SHA-256 of the exact bytes.
- Required paper-vs-community artifact sets; extra native outputs can be recorded without being elevated to score authority.
- Paper PAS/ECS must be present and within the declared scales. Community PAS/ECS may be null/absent.
- Effect estimates preserve finding/test identifiers, units and optional intervals.
- Artifact paths must be relative, non-traversing and unique per study receipt.
- Participant-level human data cannot be published without explicit redistribution authorization.
- The original analysis protocol must remain unchanged for an exact reproduction claim.

## Historical baseline snapshot (not current SOTA)

The original paper repository also contains a stored results summary generated on **30 January 2026**, covering 61 model/prompt configurations. Its highest reported PAS in that snapshot is **0.4971** (Gemini 3 Flash Preview, `v3-human-plus-demo`); its highest reported ECS (CCC) is **0.4589** (`mixed_models_v3`). The source summary and CSV artifact SHAs are recorded in `foundational_12_profile.json`.

These are historical repository results, not a present-day leaderboard, not independently reproduced in this phase, and not accompanied here by a confidence interval. PAS and ECS are separate metrics; do not combine them or compare their scales with SimBench/SSDataBench scores.

## Implemented paper-native importer v0.1

The Rust adapter now imports one original paper-run study directory containing:
- `full_benchmark.json`
- `detailed_stats.csv`
- `evaluation_results.json`
- `civdyn_paper_run_attestation.json` (a CIVDYN sidecar, not an upstream-generated file)

It hashes exact bytes before parsing; checks the native study, model and prompt preset against the sidecar; validates the paper commit, study-specific config/evaluator Git blob SHAs, baseline runner, generation runner/pipeline, evaluator runner and shared stats-library blob SHAs against the frozen source profile; checks the detailed CSV header and row count against the official evaluator’s `test_results`; derives an agent-configuration digest from settings recorded in the full benchmark; and carries the exact frozen source-pin manifest digest into each study receipt.

The importer copies `score` as official PAS and preserves `ecs_corr_study` / `ecs_corr` only when upstream supplies a finite value. **Per-study ECS is optional** because the pinned upstream metric can be undefined when fewer than three valid effect-size pairs are present. No score is reconstructed or reimplemented. Effect rows are emitted only when both `human_effect_d` and `agent_effect_d` are present.

The sidecar is an operator attestation of which upstream source revision, study config/evaluator, global runner/evaluator/metric implementation, prompt and independent unit were used. The importer checks each declared source blob against the frozen profile and requires `analysis_protocol_preserved=true`; this is still not cryptographic execution proof and cannot independently establish which external Python process produced a pre-existing output directory or prove model-weight identity. Preserve environment logs and an immutable container digest for a full reproduction claim.

The importer is a per-study adapter plus a 12-study receipt assembler. The community playground importer remains unimplemented. No model execution, HumanStudy reproduction, PAS/ECS improvement, or benchmark SOTA claim is made by these synthetic conformance tests.

No raw participant data or study instruments are copied into this repository. The adapter marks run artifacts as local, not automatically publishable; source-code licensing does not settle the rights to each paper, instrument, prompt stimulus or human dataset.

## Run

```

Assemble one matched paper-era 12-study run from the 12 native study directories:

```sh
cargo run --locked --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --bin paper-import-run -- receipt.json ./study_001_run ./study_002_run ./study_003_run ./study_004_run ./study_005_run ./study_006_run ./study_007_run ./study_008_run ./study_009_run ./study_010_run ./study_011_run ./study_012_run
```

Use `-` instead of `receipt.json` to write the normalized receipt to stdout. Every directory must contain the three pinned upstream outputs plus the CIVDYN run-attestation sidecar. All 12 imports must share the same run/model/prompt/config identity; the command fails closed otherwise.
sh
cargo fmt --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-humanstudy-repro/Cargo.toml --all-targets
```

CI also validates both JSON Schema fixtures. Green CI establishes the receipt code's software checks only; it does not establish scientific validity or benchmark reproduction.

## Rights and privacy

Source-code MIT licensing does not grant rights to redistribute each study's data, questionnaires, stimuli, papers or participant records. Keep record-level human traces out of version control unless the exact study's consent, licence and redistribution rights authorize it. Preserve source/rights references and artifact hashes even when payloads must remain access-controlled.
