# Dynamic DRSS benchmark provenance and score contract v0.1

**Related Symtropy work:** static FJSP-SDST exact oracle [#2599](https://github.com/Luminous-Dynamics/symtropy/pull/2599); static greedy schedule baselines [#2700](https://github.com/Luminous-Dynamics/symtropy/pull/2700); tracking issue [#2702](https://github.com/Luminous-Dynamics/symtropy/issues/2702).

**Status:** upstream identity, scenario/reference-table completeness and published score contract only. No upstream DRSS code is executed by this stage.

## Why this is the next benchmark, not a merged leaderboard

The static Fattahi examples validate makespan scheduling with operation precedence and sequence-dependent machine setup. The DRSS paper benchmarks dynamic 20-machine job shops on tardiness under changing utilization and due-date allowance. These objectives and simulators are not interchangeable.

The paper reports that its tested periodic and real-time dispatching-rule selection systems beat the best individual rules by over 10% on its evaluated dynamic cases. The pinned README's aggregate test scores include S9 at 1.213, DQL at 1.034 and SVM regression at 0.995; lower is better. The published test table therefore provides a concrete reproduction target, but **not a Symtropy result**. The 0.995 score is an aggregate normalized score, not a claim of 0.995 units of tardiness or a direct raw-tardiness percentage reduction.

Research source: Marques, Figueira and Guimarães, *Dynamic dispatching rule selection for the job shop scheduling problem*, *Computers & Industrial Engineering* 210 (December 2025), DOI [10.1016/j.cie.2025.111471](https://doi.org/10.1016/j.cie.2025.111471). The paper reports its own method against its own simulator/protocol. It discusses a “flusher” which overrides a dispatching rule for an operation that has remained in the shop too long; that concept motivates fairness guards in our own models but does not establish equivalent behavior.

## Pinned upstream assets and licensing boundary

Repository: [nuno-a-marques/drss](https://github.com/nuno-a-marques/drss) at commit `b3db3fb98ab5215fd8612a5ea3638702a1d39728`.

| Pinned asset | Git blob SHA-1 | Bytes |
|---|---|---:|
| `README.md` | `c3bbb6d68b864a511cc9bce03d26d795a9b5a8ee` | 16,903 |
| `drss-prs.zip` | `a64ea1ae19054d7da894320c7c7b0b9c8dda18e2` | 24,023,748 |
| `drss-rtrs.zip` | `713c46453481e5afb0e15c58bfa94d9db6e1a435` | 19,575,462 |

At this pinned revision the repository's root tree has no `LICENSE` or `COPYING` file, and GitHub returns no SPDX license metadata. The source archives are therefore downloaded into a temporary CI checkout **for content identity checks only**. They are not committed, uploaded as artifacts, or executed. This is a provenance check, not a legal conclusion; a later code-execution adapter requires separate licensing/safety review.

## Published simulation protocol captured by the manifest

- 20 machines; 2,500 jobs per episode; the first 500 jobs are warm-up and excluded from tardiness scoring; 100 replications per scenario.
- Each job has 2–14 operations; processing times are integers from 1 to the case's `ptMax`.
- Validation grid: 4 utilization levels × 4 allowance factors × 2 processing-time maxima = **32 scenarios**.
- Test grid: 5 utilization levels × 5 allowance factors × 3 processing-time maxima = **75 scenarios**.
- Full protocol budget if all 107 scenarios receive 100 replications: **10,700 episodes, 26,750,000 generated jobs, 21,400,000 jobs outside warm-up**. This is a derived budget, not a claim that this run was executed.
- The source defines periodic selection intervals of 50, 100, 200, 400 or 800 jobs, state sets SS1–SS3 using pending-operation processing times with tardiness and/or earliness features (natural-log transformed), and action IDs RR=18, H2=214 and S9=508.

The manifest preserves every per-scenario reference-tardiness value (32 validation rows, 75 test rows) and all 11 published aggregate algorithm score rows. CI parses those tables directly from the README at the exact pinned commit and requires byte-level Git blob identity plus semantic table equality; changing a single reference value or published score fails closed.

## Score semantics and next gates

For scenario `s`, the published protocol computes the model's mean tardiness divided by the scenario-specific reference value; it then takes the geometric mean of these normalized ratios across scenarios. Lower is better. Do not replace this with a pooled mean of raw tardiness, because high-tardiness cases would dominate and the resulting number would be a different metric.

The verifier report status is `source_and_score_contract_validated_not_benchmark_reproduced`. Follow-on implementation must separately:
1. review the upstream Java simulator and both PRS/RTRS paths in an isolated, temporary workspace, with explicit licensing and execution boundaries;
2. reproduce one reference policy/score under a small documented scenario subset before scaling;
3. align due-date construction, arrival process, warm-up exclusion, random seeds, policy update cadence, and exact tardiness aggregation;
4. match paper validation/test scenario membership without tuning on test results;
5. scale to the full 10,700 episode grid and compare confidence intervals or nonparametric tests per scenario and aggregate score, recording wall-clock/runtime decision latency and rule switches.

No paper reproduction, policy comparison, Symtropy score, state-of-the-art claim, factory-output estimate, safety certification or production readiness follows from this contract stage.
