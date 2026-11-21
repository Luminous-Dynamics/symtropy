# STEP constrained multiobjective benchmark plan v0.1

**Status:** reference plan only. No STEP task is implemented or executed by Symtropy at this revision.

## Why this lane is necessary

The current analytic qualification lane runs uniform random, NSGA-II and GDE3 on ZDT1–ZDT4. It tests known-front objective search, disconnected Pareto fronts and multimodality, but not constrained feasibility discovery. Engineering optimization often has disconnected or narrow feasible regions, changing feasible-front dimension, many interacting constraints and multiple active constraint boundaries.

Cork, Tušar and Filipič introduced the Selected Test Problems (STEP) suite in *Knowledge-Based Systems* 348 (3 August 2026), article 116376, DOI 10.1016/j.knosys.2026.116376. It selects 30 constrained problems in ten bins (2/3 objectives × 2/3/5/10/30 variables), with three problems per bin. It was designed to improve diversity and distinguish algorithm performance. The paper notes that some two-variable problems are too easy to discriminate methods, so report bin and task results separately.

A second 2026 comparison benchmarks 12 representative constrained multiobjective evolutionary algorithms across 92 tasks and nine suites. Its aggregate IGD results put c-DPEA and CCMO at the top; it also points to deficiencies in existing artificial problems, particularly variable feasible-front dimensionality, large constraint counts and diverse unconstrained front geometries. This is a strong reason not to stop at ZDT or at the current three unconstrained algorithms.

## Immutable source and license boundary

The reference-only map is data/cmo_step_reference_manifest.v0.1.json.

- Paper: https://doi.org/10.1016/j.knosys.2026.116376
- External implementation: https://github.com/jsi-ci/cmo
- Pinned main commit observed 2026-11-01: 9ba3f0649c87005dc7675b18698c2518659dc88b
- Pinned tree: a0ccc5a94b4d6ffe0ffbffb2c1a5d2c559ef7252
- Repository reports GPL-3.0. No upstream code or Pareto-front .pf assets are vendored into this Apache-2.0/MIT crate. The Rust adapter must be clean-room, derived from published mathematical definitions and reviewed independently.
- The article points to algorithm-run data at https://doi.org/10.5281/zenodo.19734874. Dataset-specific terms have not been verified; no bytes are downloaded or retained.

The manifest stores upstream Git blob IDs and sizes as object references, not as source authentication or permission to redistribute.

## STEP 30-task matrix

| Task | Problem | Objectives | Variables |
|---|---|---:|---:|
| STEP1 | DC2-DTLZ3 | 2 | 2 |
| STEP2 | MW7 | 2 | 2 |
| STEP3 | MW11 | 2 | 2 |
| STEP4 | MW11 | 2 | 3 |
| STEP5 | DC2-DTLZ1 | 2 | 3 |
| STEP6 | CF4 | 2 | 3 |
| STEP7 | C2-DTLZ2 | 2 | 5 |
| STEP8 | RCM2 | 2 | 5 |
| STEP9 | DC2-DTLZ1 | 2 | 5 |
| STEP10 | C2-DTLZ2 | 2 | 10 |
| STEP11 | CF4 | 2 | 10 |
| STEP12 | MW8 | 2 | 10 |
| STEP13 | DC2-DTLZ1 | 2 | 30 |
| STEP14 | ZXHCF16 | 2 | 30 |
| STEP15 | MW11 | 2 | 30 |
| STEP16 | DC2-DTLZ3 | 3 | 2 |
| STEP17 | DAS-CMOP8 | 3 | 2 |
| STEP18 | DC1-DTLZ3 | 3 | 2 |
| STEP19 | DC2-DTLZ3 | 3 | 3 |
| STEP20 | C3-DTLZ4 | 3 | 3 |
| STEP21 | DAS-CMOP8 | 3 | 3 |
| STEP22 | DC2-DTLZ3 | 3 | 5 |
| STEP23 | C3-DTLZ4 | 3 | 5 |
| STEP24 | C1-DTLZ1 | 3 | 5 |
| STEP25 | DC2-DTLZ1 | 3 | 10 |
| STEP26 | MW8 | 3 | 10 |
| STEP27 | C3-DTLZ4 | 3 | 10 |
| STEP28 | DC2-DTLZ3 | 3 | 30 |
| STEP29 | DAS-CMOP9 | 3 | 30 |
| STEP30 | CF8 | 3 | 30 |

Repeated base problems at different dimensions are separate task instances, not automatically independent problem families.

## What the published comparison says

The STEP paper's Table 10 reports both average algorithm rank and average AUC-ERD. AGE has the best mean rank, while CCMO/MCCMO/C-TAEA have the highest reported AUC-ERD values. Task-level winners vary. These are literature results, not Symtropy reruns; rank and AUC-ERD are not interchangeable. The new adapter must reproduce AUC-ERD exactly before using it as a comparison, and add feasible-only HV/IGD/IGD+ and feasibility-time curves.

The 2026 12-algorithm comparison identifies c-DPEA and CCMO as leading aggregate IGD baselines and discusses co-evolutionary approaches that preserve exploration through infeasible regions while maintaining a feasible Pareto search. This is an existing research direction, not a novelty claim for our design.

References:
- Cork, Tušar and Filipič (2026), STEP: https://doi.org/10.1016/j.knosys.2026.116376
- Ming et al. (2026), comprehensive CMOEA comparison: https://doi.org/10.1007/s10462-026-11576-9
- External source repository (GPL-3.0): https://github.com/jsi-ci/cmo

## Research hypothesis: evidence-gated feasibility coevolution

Test a two-archive policy with controlled fidelity promotion.

**Feasibility discovery archive.** Search for feasible-region discovery, low per-constraint violation and diversity across active-constraint patterns. It may preserve infeasible candidates as search information, but it may not certify a candidate.

**Feasible Pareto archive.** Contains only candidates whose constraints were evaluated and independently admitted at the declared fidelity/evidence level. Its Pareto and diversity calculations use feasible results only.

**Controlled exchange.** Candidate vectors or boundary seeds may cross between archives; feasibility labels may not. Promotions require re-evaluation and independent admission. Retain provenance from the originating archive and every evaluation event.

**Multi-fidelity decision policy.** Use a validated low-fidelity model only for screening. Spend high-fidelity calls where feasibility uncertainty or Pareto decision impact is largest. EVSI-style selection is allowed only with calibrated likelihoods; otherwise use transparent deterministic priorities and abstain out of domain. Charge all screens, failed solves, high-fidelity conditions, model fits, solver/mesh setup and audits to total budget.

Two archives/co-evolution alone are not novel: CCMO, c-DPEA and the March 2026 DPNCMO method already use related dual-population/co-evolutionary exploration. DPNCMO combines adaptive constraint relaxation with novelty-guided selection, and reports tests on 43 CMOP instances and 12 engineering problems. It is a required baseline, not a novelty claim. Potential contribution to test is the combination with per-constraint diversity, calibrated feasibility uncertainty, independent admission/promotion, selective abstention and total-cost/decision-value accounting. Require ablations and literature review before describing it as novel.

DPNCMO reference: Shiwei Chen et al., DOI 10.1093/jcde/qwag019, https://doi.org/10.1093/jcde/qwag019 (published March 2026).

## Baselines

Reproduce or compare against, subject to implementation availability and licence: random search with feasibility filtering; constraint-dominance NSGA-II / C-NSGA-II; GDE3 with explicit constraint handling; C-TAEA; CCMO; c-DPEA; DPNCMO; AGE-MOEA/AGE-MOEA-II; SPEA2; NSGA-III; C-MOEA/D / MOEA/D-IEpsilon; NSDE and NSDER. Specialist STEP paper baselines should be used where parameters and implementations are independently reproducible.

Record source revision, implementation identity, parameters, stopping criteria, compute environment and all tuning cost. The benchmark must not be reduced to our existing NSGA-II/GDE3 implementations.

## Fair experiment protocol

- Freeze definitions, bounds, constraint signs/tolerances, reference fronts, normalisation, seeds, metrics and budgets before tuning.
- Minimum 30 independent seeds per deterministic task; 50–100 for noisy or costly variants.
- Paired common initial designs; record divergence in subsequent search streams.
- Matched objective/constraint evaluation budgets and matched wall-clock/compute/energy budgets.
- Charge all low/high-fidelity calls, failed/non-converged runs, model fitting, acquisition, setup, tuning, orchestration and audit.
- Feasibility metrics: probability of finding feasible candidates, evaluations/time to first feasible, per-constraint violation profiles, feasible-front component coverage and false-feasible rate.
- Pareto metrics: feasible-only HV, IGD, IGD+, epsilon and coverage; AUC-ERD only when paper semantics are faithfully reproduced.
- Cost metrics: feasible HV versus total cost, probability of reaching a target feasible HV, cost per admitted feasible candidate, failed-call rate and resource/energy consumption.
- Use paired seed-level deltas, task-level results, ten-bin summaries, cluster bootstrap across base-problem families, effect sizes and multiplicity-corrected tests. Do not pseudo-replicate repeated base functions at different dimensions.
- Keep a blind family/condition confirmation split; report all failed runs and deviations.

## Pre-registered targets (aspirational, not achieved)

1. Exact independent agreement on all 30 objective/constraint definitions and constraint signs.
2. Improved time-to-first-feasible and feasible-run probability on narrow/disconnected feasible regions versus C-TAEA, CCMO and c-DPEA, without increasing false-feasible rate.
3. Better feasible-only HV/AUC-ERD at equal total cost, measured per bin and across a blind family holdout.
4. Target ≥20% fewer high-fidelity condition calls to the same independently admitted feasible-Pareto target, without degrading blind-set feasibility or uncertainty calibration.
5. Nominal interval coverage and false-feasible rate reported on held-out shifts; abstention outside the qualified domain.
6. Replication on DPW8-derived engineering tasks before any real-world aerospace superiority claim.

A superiority claim requires identical task definitions, baselines, seed protocol, budget counters, indicators and blind data split, plus statistical and practical significance.

## Implementation order

1. Pin reference metadata and terms (this version).
2. Clean-room implement a two-objective and a three-objective constrained pilot with formula provenance, known-answer tests and constraint-sign negative controls.
3. Implement generic feasibility/Pareto archives with evidence-gated promotion.
4. Run one-archive vs two-archive, scalar vs per-constraint diversity, fixed vs multi-fidelity and random vs EVSI ablations.
5. Expand across all 30 instances and add independent objective/constraint replay.
6. Reproduce AUC-ERD only after metric semantics and any external data terms are verified.
7. Connect selected expensive cases to CFD artifact/admission receipts without collapsing synthetic CMO results into aerodynamic validation.

## Evidence policy

A manifest check is not a task implementation. A formula unit test is not a full-suite reproduction. A synthetic run is not STEP performance. A benchmark win is not aerodynamic design validation. A low-fidelity feasibility prediction is not an admitted design. A queued workflow is not a pass.
