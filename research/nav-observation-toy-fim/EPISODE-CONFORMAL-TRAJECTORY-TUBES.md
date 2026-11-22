# Episode-Level Conformal Trajectory Tubes

## Purpose

This module calibrates a conservative, constant-radius 2D tube around a predicted trajectory using one calibration score per independent trajectory episode. Its purpose is to measure whole-episode localization uncertainty without treating temporally correlated packets or adjacent positions as independent calibration trials.

This is an uncertainty-calibration primitive. It is not a protection-level theorem, a rare-event probability-of-hazardously-misleading-information (PHMI) certificate, a safety case, or a substitute for deterministic fault analysis.

## Score and finite-sample rule

For calibration episode i, define

`S_i = max_{t in episode i} ||x_hat_(i,t) - x_true_(i,t)||_2`.

The maximum is taken across every evaluated position in that complete episode. The calibration set therefore contains n episode scores, not the sum of the packet counts or position counts.

For requested marginal trajectory coverage p, select the one-based order-statistic rank

`k = ceil((n + 1) p)`.

Sort the n scores in ascending order and set the tube radius to the k-th score. If k > n, no finite conformal radius is emitted. For example:

| Target marginal coverage | Calibration episodes | Rank | Result |
|---:|---:|---:|---|
| 95% | 19 | 19 | finite radius: calibration maximum |
| 99% | 98 | 99 | no finite radius |
| 99% | 99 | 99 | finite radius: calibration maximum |
| 99.9% | 998 | 999 | no finite radius |
| 99.9% | 999 | 999 | finite radius: calibration maximum |

The finite radius is an order statistic, not a claim that 19, 99 or 1,000 episodes establish an operationally acceptable hazard rate with a separate confidence level.

The implementation records evaluated-position count for audit context, but that count never increases n. It rejects duplicate episode IDs and repeated independence-group IDs. Those structural checks help prevent obvious pseudo-replication; they cannot prove that the episodes are statistically independent or exchangeable.

## Guarantee and assumptions

Under exchangeability of the independent episode-level scores, the split-conformal order statistic provides marginal coverage for a future complete episode drawn from the same exchangeable population. The covered event is joint across the episode: every evaluated true position lies within its corresponding radius disk around the predicted position.

The predictor being calibrated must be fixed independently of the calibration episodes. Any estimator training, hyperparameter selection, feature selection, threshold selection or calibration-stratum design that uses the calibration outcomes changes the procedure and can invalidate the stated split-conformal guarantee. Use a separate development cohort for those choices, then freeze the full predictor and score definition before computing calibration scores.

The guarantee does **not** imply:

- conditional 99% coverage in every room, anchor geometry, device, channel, obstacle material or motion regime;
- coverage under arbitrary distribution shift or adversarial manipulation;
- a time-uniform PHMI or per-hour integrity guarantee;
- a calibrated covariance matrix or Gaussian confidence ellipse;
- statistical independence merely because episode identifiers differ;
- authenticity of caller-supplied SHA-256 commitments.

The module accepts source-manifest, cohort, score-definition and solution commitments as inputs. It checks that they are nonzero and preserves them. It does not calculate or authenticate the bytes behind those digests, determine whether ground truth is independent, or prove the cohort's exchangeability.

## Required evaluation protocol

Use four disjoint cohorts:

1. **Development/training cohort**: fit the estimator and choose every model, feature, threshold, hyperparameter, window, optimizer and preprocessing choice. This cohort must not be reused for conformal calibration or final evaluation.
2. **Calibration cohort**: run the frozen prediction pipeline once and compute only the episode maximum-error scores used to select the conformal order statistic. Do not tune the estimator, select features, adjust thresholds or choose a subgroup policy using these scores. Freeze the cohort manifest, predictor version and score definition before calibration.
3. **In-distribution evaluation cohort**: evaluate whole-episode containment on independent episodes not used for development or calibration, and with independence-group IDs disjoint from both.
4. **Shift/challenge cohort**: hold out entire environments, days, anchor configurations, device or firmware revisions, obstacle materials, and motion regimes. Report this separately; do not extend the in-distribution guarantee to unsupported strata.

Where multiple trajectories come from the same physical run, site visit, device session or common environmental episode, they must not silently count as independent calibration units. Define the independence group at the level justified by the data-generation process. If independence cannot be justified, group the observations conservatively or mark the result indeterminate.

Do not randomly split packets from the same trajectory across calibration and evaluation. Do not let truth labels, future observations, evaluator-only invalidity flags or NLOS labels influence the controller's input selection.

For each evaluation episode, compute one Boolean whole-trajectory result: all valid truth positions inside the predicted tube, or a failure. Then report:

- number of independent evaluation episodes;
- number of whole-trajectory tube failures;
- empirical coverage;
- a one-sided exact binomial confidence bound on failure probability;
- mean, median, P90, P95 and maximum tube radius/width;
- results by environment and fault/shift stratum;
- availability and proportion of episodes for which no finite calibration support exists.

Do not convert the conformal marginal guarantee into a confidence interval on PHMI. The held-out binomial confidence bound is a separate empirical statement about the evaluation cohort.

### Sample-size interpretation

With zero observed failures, the one-sided 95% exact binomial upper bound on the failure probability is

`1 - 0.05^(1/N)`,

where N is the number of independent evaluation episodes. Approximately 299 zero-failure episodes are needed to put this upper bound below 1%; approximately 2,995 are needed to put it below 0.1%; approximately 29,956 are needed to put it below 0.01%.

These counts are for independent **evaluation** episodes, not calibration episodes, and assume the episode outcomes are independent Bernoulli trials from the population being described. Correlated episodes do not count separately. The codebase's existing integrity-evidence module owns exact binomial confidence bounds; this module intentionally does not implement a second interval engine.

## Integration with navigation integrity

A useful combined design keeps two sources of uncertainty distinct:

- **Statistically calibrated residual tube**: episode-level conformal radius for the empirically sampled residual process.
- **Deterministic hard-fault envelope**: explicit bounded terms for clock, anchor, antenna delay, calibration, geometry, synchronization and other admissible common-cause hypotheses.

Do not root-sum-square correlated systematic terms without a justified dependence model. A later integration may form a conservative set sum or outer union of the statistical tube and deterministic hard-fault sets, but the combination rule must be proven conservative for the stated geometry and decision profile. Until then, the episode tube is diagnostic and cannot restore NORMAL after a fault-hypothesis monitor has declared an incomplete or unbounded hypothesis set.

A practical decision policy should return a distinct unsupported/unknown state when the current environment or configuration is outside calibration support, when the score contract changed, when calibration is stale, or when the hard-fault envelope is unbounded.

## Benchmark tournament

Run all methods with identical source packets, anchor surveys, evaluation truth, hardware privileges and trajectory-level train/calibration/test partitions:

1. WLS and conventional EKF/ESKF;
2. robust or NLOS-aware methods available on the same inputs, including MCC-VC-TOA, SR-MCC, AR-PNN and published HBR-PGD;
3. learning-assisted and correction methods such as LARC or CR-WLS only when their data requirements and calibration privileges can be matched;
4. solution-separation / RAIM-style integrity comparators;
5. HBR-PGD with independent range-set posterior audit;
6. HBR-PGD with dependency-aware multi-hypothesis protection envelope;
7. HBR-PGD plus episode-level conformal tube;
8. the combined statistical-plus-deterministic method only after its set-combination proof and independent evaluation.

Report separate Pareto frontiers rather than one weighted leaderboard score:

- point error: RMSE, MAE, median, P90, P95, P99 and maximum;
- complete-episode coverage at 90%, 95% and 99%;
- tube radius/diameter and width-versus-coverage;
- false-safe, missed-fault, false exclusion and false-alert rates by fault family;
- unsupported-NORMAL exposure and time to alert/recovery;
- qualified availability and continuity at matched alert limits;
- performance under environment, device, anchor, channel and common-cause shift;
- CPU time, p50/p95/p99 latency, peak RAM and measured energy on named hardware.

The principal success criterion is not a lower RMSE alone. It is a better integrity/availability/width/resource Pareto frontier at matched sensor privileges and source protocol, with no leakage and no hidden outlier or unavailable episodes.

## Falsification suite

At minimum include:

- a nominal low-error trajectory with a low-calibration-support environment;
- rare large errors concentrated in one independent episode;
- thousands of correlated packets in one episode versus the same error represented once;
- repeated episode IDs and repeated independence-group IDs;
- calibration/evaluation reuse by ID or group;
- 99% calibration at n=98 and n=99;
- one test episode with a single position outside the tube;
- a trajectory with a missing truth interval;
- environment, anchor geometry, device, channel and obstacle-material shifts;
- common-mode clock/antenna/calibration faults;
- stale or changed score-definition and cohort commitments;
- test truth mutations that must not alter controller event inclusion or order;
- invalid truth, missing source lineage and unknown strata.

Metamorphic requirements:

- increasing any calibration episode's maximum score cannot reduce the selected radius;
- input permutation does not change the result;
- increasing packet count alone does not increase independent n;
- adding a calibration episode from an already represented independence group is rejected;
- calibration/evaluation overlap is rejected;
- a non-finite score, missing source commitment or unsupported order-statistic rank never produces a finite tube.

## Relationship to the HBR-PGD reproduction

HBR-PGD's local curvature-trace fusion is an estimator aggregation heuristic. Curvature is not measurement-error covariance. The episode-level tube is a separate empirical calibration layer: it calibrates complete trajectory residuals after the estimator has produced the predicted trajectory. The two must not be conflated.

The HBR-PGD paper's reported point-error metrics must first be reproduced on the exact source data version, preprocessing and calibration/test split. Only then should the episode tube be evaluated as a separate extension. A different Bregar release, different trajectory partition or different preprocessing must be reported as a separate experiment, not as a reproduction of the paper.

## Current implementation status

Implemented in `episode_conformal_tube.rs`:

- typed episode maximum-error input;
- explicit independence-group identity;
- finite-sample order-statistic calibration;
- fail-closed rank and input validation;
- source/cohort/score/solution commitments retained in the report;
- calibration/evaluation episode and group disjointness checks;
- constant-radius complete-trajectory tube;
- tests for rank boundaries, packet-count non-inflation, permutation invariance, monotonicity, duplicate groups, whole-trajectory containment and invalid data.

Not implemented or demonstrated:

- source data extraction, truth qualification or SHA-256 calculation;
- statistical proof of independence/exchangeability;
- environment-conditional or weighted conformal calibration;
- a distribution-shift detector or a justified backoff policy;
- deterministic hard-fault set combination;
- real Bregar/IDLab/outdoor/MILUV calibration or held-out results;
- measured latency, memory or energy;
- physical navigation or safety qualification.

This module should remain marked **research-only** until independent source-complete episodes, frozen cohort manifests, and held-out evaluation results exist.
