# HumanStudy-Bench native output contract (pinned paths)

This is a source-derived path map for the two protocols used by the CIVDYN receipt gate. All paths are relative to the native run or config directory. Hash exact bytes before parsing.

## Paper repository: `AISmithLab/HumanStudy-Bench`

The pinned `docs/RESULT_STORAGE.md` documents:

```text
results/runs/<run_name>/<study_id>/<model>_<prompt>/
  full_benchmark.json
  detailed_stats.csv
  evaluation_results.json
  [raw_responses.json]
```

Required receipt roles:

| Role | Path | Authority |
|---|---|---|
| `paper_full_benchmark` | `full_benchmark.json` | Original simulation/run payload and usage |
| `paper_detailed_stats` | `detailed_stats.csv` | Original detailed statistical table |
| `paper_evaluation_results` | `evaluation_results.json` | Official PAS/ECS and per-test evaluator outputs |

Optional: `paper_raw_responses` at `raw_responses.json` where present. Aggregate summary tables in `results/benchmark/` are derived outputs; record them as separately identified baseline artifacts, never as substitutes for the native run files.

## Community repository: `HumanStudy-Hub/HumanStudy-Bench`

The pinned Community Edition `playground/README.md` and `playground/run_playground.py` document:

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

Required receipt roles:

| Role | Path | Authority |
|---|---|---|
| `community_run_manifest` | `run.json` | Run settings/model/prompt provenance |
| `community_responses` | `output/responses.json` | Participant-response payload passed to evaluator |
| `community_evaluation` | `output/evaluation.json` | Study-native evaluator output |
| `community_analysis` | `output/analysis.json` | Normalized one-row-per-test analysis |

Optional: `community_profiles`, `community_transcript_sample`, `community_charts_non_authoritative`. Charts are presentation artifacts. `output/analysis.json` does not automatically contain the original paper's benchmark-level PAS/ECS.

## Importer invariants

- Hash each file as raw bytes before UTF-8/JSON/CSV parsing.
- Store path, role, byte count, record count if known, SHA-256, rights basis and privacy classification.
- Do not normalize unknown fields away. Keep the original artifact as the source of truth.
- If an expected native artifact is missing, fail the exact-reproduction receipt. Do not synthesize a substitute from a summary.
- Do not infer the independent statistical unit from row count; map it from the study's original design and evaluator.
- Keep charts, LLM-generated summaries and aggregate dashboards explicitly non-authoritative.
