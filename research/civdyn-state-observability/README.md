# CIVDYN state-observability reference audit v0.1

This is a **pinned artifact audit and reproduction adapter**, not a reimplementation of the IJCAI paper's algorithms.

It targets Wendland et al., *Missingness-MDPs: Bridging the Theory of Missing Data and POMDPs*, IJCAI 2026, pp. 6217–6226:
https://ijcai.org/proceedings/2026/692

The upstream implementation is MIT-licensed:
https://github.com/ai-fm/missingness-mdps

## What is pinned

- Upstream commit: `186c454c8e1581adbbf3cbb49ed9e0df510356f4`
- License file git blob: `859c41b3529fe6408802113bf8931575339e3262`
- Experiment, learning, metrics, ICU and Predator source-file git blobs: see `reference-lock.json`.
- Five primary result bundles with exact Git blob IDs are pinned in the same lock.

The five audited primary experiment bundles are:
1. `icu/smar`
2. `icu/mnar-ident`
3. `icu/mnar-unident`
4. `predator-factored/smar`
5. `predator-factored/mnar-unident`

Each bundle declares 20 repetitions, ten dataset sizes `[10,50,100,500,1000,5000,10000,100000,1000000,10000000]`, and three learned missingness assumptions (MCAR, MAR, MNAR). That yields 600 learning rows per bundle.

## Direct artifact audit finding

The pinned upstream result JSON files contain 3,000 learning records in total. Their status is:

| Experiment | Finished | Recorded exceptions | Failure rate |
|---|---:|---:|---:|
| ICU simple MAR | 567 / 600 | 33 | 5.50% |
| ICU identifiable MNAR | 564 / 600 | 36 | 6.00% |
| ICU unidentifiable MNAR | 585 / 600 | 15 | 2.50% |
| Predator simple MAR | 600 / 600 | 0 | 0.00% |
| Predator unidentifiable MNAR | 561 / 600 | 39 | 6.50% |
| **Total** | **2,877 / 3,000** | **123** | **4.10%** |

The 123 exception rows are not silently filtered. All 123 are reported as `RemoteException` with the same failure fingerprint: `Failed discrete belief update: new probabilities sum to zero.` Exactly 120 occur at dataset size 10 and 3 at size 50; none occur at larger sizes. The normalizer and Rust auditor retain the exception type and check that every failed row has this fingerprint while every finished row has no exception metadata. The status is sourced from the upstream `finished` and `exception` fields, not inferred from absence of summary metrics.

This is an **artifact audit**, not a conclusion that the paper's aggregate plots are wrong. It establishes that the public pinned result JSON contains explicit failed learning tasks and that these should remain part of the reproducibility denominator. The reported mean/TV summaries for successfully completed rows must be read alongside failure rates and the original paper's aggregation procedure.

## Matrix coverage limits

The pinned tree contains three primary ICU result bundles and two primary Predator bundles. There is no primary ICU MCAR `results.json` in `experiments/logs/icu` at this commit. Predator MCAR search outputs use a separate experiment contract and must be audited independently rather than coerced into these five bundles.

The upstream tree also contains 242 separate PPO / stacked-PPO result JSON files under `logs_PPO` and `logs_PPO_stacked`. They are not counted as complete baseline coverage by this v0.1 adapter. A separate adapter must map their experiment identifiers, seed schedules, task completion and hyperparameters before the PPO baseline matrix can be called reproduced.

The five primary bundles' `results` arrays contain two baseline rows (`upper` and `random`), one M-constant row and eight POMCP rows. The reference audit checks these identities and completion flags, but does not assert that these arrays alone reproduce every model-free baseline in the paper.

## Adapter contract

The shell normalizer:
- checks the upstream repository checkout is at the exact pinned commit;
- checks Git object hashes for the license, relevant source files and all five results JSON blobs;
- emits one contract record and each raw learning task row in TSV;
- preserves failed rows, missing means, exception flags and both average/worst TV.

The dependency-free Rust auditor then checks:
- exactly five expected experiment identities and state counts;
- 20 seeds × 10 dataset sizes × 3 assumptions = 600 unique run keys per bundle;
- no duplicate or missing run keys and no unknown assumptions/algorithms;
- exact baseline identities and counts, plus M-constant and POMCP counts;
- finished rows have means and valid TV values;
- unfinished rows carry recorded exceptions and are included in the denominator;
- pinned per-experiment and per-dataset-size failure counts;
- deterministic aggregate CSV summary.

The auditor does not recompute TV from state-level distributions: the upstream result JSON stores only aggregate ATV/WTV, not the complete per-state distributions. It can therefore verify the finite aggregate values and compare them with policy value, but cannot independently regenerate the statewise TV calculation. An independent re-computation of TV requires additional state-level output from the upstream runner. This is recorded as an evidence limitation, not approximated.

## Run locally

Requirements: Git, jq and Rust toolchain.

```bash
git clone https://github.com/ai-fm/missingness-mdps reference/missingness-mdps
git -C reference/missingness-mdps checkout 186c454c8e1581adbbf3cbb49ed9e0df510356f4
bash scripts/audit_miss_mdp_reference.sh reference/missingness-mdps > /tmp/miss-mdp.tsv
cargo run --locked --manifest-path research/civdyn-state-observability/Cargo.toml -- /tmp/miss-mdp.tsv
```

The script intentionally refuses an unpinned upstream commit or blob mismatch.


## PPO / stacked-PPO artifact matrix (separate baseline audit)

The pinned upstream tree also contains **242 individual PPO result manifests** under `experiments/logs_PPO` and `experiments/logs_PPO_stacked`. These are audited separately from the five primary learning-result bundles above. They form 12 groups with a non-uniform count:

| Family | Group | Files |
|---|---|---:|
| PPO | ICU / simple MAR | 20 |
| PPO | ICU / identifiable MNAR | 20 |
| PPO | ICU / unidentifiable MNAR | 20 |
| PPO | Predator / MCAR search | 20 |
| PPO | Predator / simple MAR | 20 |
| PPO | Predator / unidentifiable MNAR | 20 |
| Stacked PPO | ICU / simple MAR | 23 |
| Stacked PPO | ICU / identifiable MNAR | 20 |
| Stacked PPO | ICU / unidentifiable MNAR | 19 |
| Stacked PPO | Predator / MCAR search | 20 |
| Stacked PPO | Predator / simple MAR | 20 |
| Stacked PPO | Predator / unidentifiable MNAR | 20 |
| **Total** | | **242** |

Each individual manifest includes its own seed, model/pattern, training budget, evaluation episode count, architecture and PPO hyperparameters. The audit normalizer validates those per-run records and the Rust adapter checks exact group membership/counts, unique run IDs, non-zero seed coverage, budget integrity, finite return summaries and observed model labels. The upstream stacked-Predator path uses both `pred` and `predator` model labels; the audit permits only those two scoped aliases and emits the actual observed set rather than normalizing the source value away. It reports per-group mean/median/min/max returns, mean reported return standard deviation, seed coverage, architecture/hyperparameter diversity, the complete observed run-ID sequence, missing run IDs and duplicated seed values. Non-contiguous IDs are preserved; they are not renumbered to the file count.


### Pinned PPO artifact irregularities retained in the audit

The 242 manifests include two groups with non-contiguous run IDs:
- Stacked ICU / unidentifiable MNAR: 19 files, IDs 1–6 and 8–20; **PPO_007 is absent**. There are 17 unique seeds; seeds 421 and 422 appear twice each, with matching recorded hyperparameters and returns in the repeated pairs.
- Stacked ICU / simple MAR: 23 files, IDs 1–10 and 12–24; **PPO_011 is absent**.

These are reported as artifact coverage/provenance findings, not silently repaired. The audit requires each present ID to be unique and consistent with its path, but no longer assumes that the numeric ID is at most the number of files. The duplicated seeds remain in the run denominator and are emitted as a separate field. The exact missing IDs and the first group's duplicate seed values are pinned in CI.

**Important comparison boundary:** the audit does not pool ICU and Predator returns into a single headline mean because they are different tasks with different reward scales. Stacked and unstacked models are also kept as separate groups; their architecture and tuned hyperparameters differ, so their raw returns are not by themselves a controlled causal comparison. The separate output is a provenance/coverage audit of the available reference artifacts, not a claim that all methods were matched in compute or evaluation.

### Producer provenance finding (important limitation)

The exact producer code was inspected and pinned. `experiments/PPO_baseline.jl` declares `RUNS = 0`, `N_FRAMES = 1` and `SEED = 1500` at this commit. Its unmodified execution therefore generates no PPO training runs and cannot generate the stacked records with `n_frames = 149`. The training implementation exists in `src/PPO.jl` (`train_PPO_agent` and `sample_ppo_hparams`), and the shared frame-stacking module exists, but the pinned experiment tree contains no dedicated stacked-PPO experiment entrypoint. The artifact normalizer now verifies exact Git blob IDs for the producer implementation, frame-stacking module, logger, missingness model and Julia project, and asserts the disabled producer state.

The result JSON files themselves record the seed, hyperparameters, training steps, evaluation episode count and aggregate return mean/std, but they do not record the generating source commit, environment-lock digest, raw episode returns or an immutable run receipt. Thus the 242 files are **pinned result artifacts**, not a fully reproducible baseline run matrix. The checked-in baseline entrypoint is not sufficient to regenerate them. A faithful reproduction requires a versioned, executable runner with non-zero run count, explicit stacked configuration, source/environment digests, seed schedule, raw evaluation episode outputs and an independently checked run manifest.

## Policy-value / observation-fit analysis (per-cell, not pooled)

The adapter also emits a separate 150-row per-cell summary (five experiment bundles × ten dataset sizes × three assumed missingness classes). It reports completed/exception counts, the distribution of successful per-seed policy returns, average TV, average worst-state TV, and a within-bundle normalized policy value:

`(candidate mean return - random policy mean) / (SARSOP true-M mean - random policy mean)`

The normalization anchors are taken from the corresponding result bundle. It is descriptive and conditional on completed runs; it is not a confidence interval, and the summary retains the number of exception rows in every cell. ICU and Predator scores are not pooled.

A few source-derived examples illustrate why both axes are needed:

| Bundle / sample size / assumed mechanism | Finished | Normalized policy value | Mean average TV | Mean worst-state TV |
|---|---:|---:|---:|---:|
| ICU simple MAR / 10 / MAR | 9 of 20 | 0.9306 | 0.4383 | 0.7683 |
| ICU simple MAR / 10,000,000 / MAR | 20 of 20 | 0.9986 | 0.00067 | 0.00160 |
| ICU identifiable MNAR / 10,000,000 / MNAR (AIMI) | 20 of 20 | 0.9977 | 0.00252 | 0.01333 |
| Predator unidentifiable MNAR / 10,000,000 / MNAR (AIMI) | 20 of 20 | 0.9093 | 0.8273 | 0.9525 |

The Predator unidentifiable-MNAR example is particularly instructive: the normalized policy value remains around 0.91, while the learned observation function is still very far from the true function (average/worst-state TV around 0.83/0.95). A policy-value-only leaderboard would hide this failure. Conversely, the ICU simple-MAR low-sample cell's value is calculated over only 9 successful runs; it must not be read as a 20-run estimate.

These statistics are recomputed from the pinned run rows and baseline anchors. They are not independently reproduced upstream plots, and they do not include confidence intervals or independent coverage/power analysis. The audit output and tests preserve all failed rows.

To generate the per-cell table after normalizing the upstream artifacts:

```bash
cargo run --locked --manifest-path research/civdyn-state-observability/Cargo.toml -- --policy-summary /tmp/miss-mdp.tsv
```

## Next step beyond the paper

After paper-faithful reproduction and separate PPO artifact auditing:
1. Add policy-dependent observation availability (e.g. sensing actions, reduced inspections, telemetry loss during stressed service).
2. Add correlated feature outages and infrastructure common-cause failures.
3. Add unknown transition dynamics as an independent axis from unknown observation functions.
4. Introduce active sensing actions with explicit cost, latency and service trade-offs.
5. Evaluate worst-subgroup belief error, service-floor violations and decision regret alongside ATV/WTV.
6. Test shift between training and evaluation missingness mechanisms, including identifiable-to-unidentifiable transitions.

Do not combine these challenge results with the paper's original score. The paper assumes known underlying transition dynamics and an observation function whose missingness function is action-independent without loss of generality in its formulation; policy-dependent sensing is an extension that needs its own explicit causal/structural contract.

## Claim ceiling

This v0.1 deliverable audits public pinned artifacts and their run-denominator integrity. It does not reproduce the paper from source, recompute the statewise observation TV, rerun SARSOP/POMCP/PPO, validate external data, or claim SOTA superiority. A future reproduction must use the upstream experiment contracts and matched compute/seed budgets.
