# External job-shop benchmark parity contract v0.1

**Related dispatch-policy evaluation:** [EV-FACTORY-DESIGN-003 #2445](https://github.com/Luminous-Dynamics/symtropy/issues/2445)  
**Current EV-line experiments:** [PR #2443](https://github.com/Luminous-Dynamics/symtropy/pull/2443), [age fairness override #2457](https://github.com/Luminous-Dynamics/symtropy/pull/2457), [threshold sweep #2467](https://github.com/Luminous-Dynamics/symtropy/pull/2467), [multiplicity correction #2545](https://github.com/Luminous-Dynamics/symtropy/pull/2545)  
**Status:** pinned input parity plus exhaustive exact makespan oracles and four deterministic greedy serial-schedule baselines on three tiny instances. This remains a small static makespan screen, not external-paper reproduction or a SOTA claim.

## Why a second benchmark stratum is required

The EV-line simulator and external job-shop suites answer different questions. The EV-line model reports good units/hour, changeovers, scrap/rework, queue waits, finite-buffer blocking and release-order inversions. A static flexible job shop with sequence-dependent setup times (FJSP-SDST) evaluates makespan under operation precedence, machine alternatives and machine-specific setup transitions. A dynamic due-date benchmark evaluates tardiness under a declared arrival/due-date process. Do not collapse them into one leaderboard or reuse a similarly named policy without matching its semantics.

Two public sources provide complementary references:

1. **Unified static/dynamic job-shop benchmark (2026).** Reijnen et al., *Job shop scheduling benchmark: Environments and instances for learning and non-learning methods*, *Annals of Mathematics and Artificial Intelligence* (2026), DOI [10.1007/s10472-026-10007-3](https://doi.org/10.1007/s10472-026-10007-3). The [public benchmark repository](https://github.com/ai-for-decision-making-tue/Job_Shop_Scheduling_Benchmark_Environments_and_Instances) includes JSP, FSP, FJSP, FAJSP, FJSP-SDST and online-arrival variants, plus exact, heuristic and learning-based methods.
   - Pinned repository revision: `d088a26582f5cb79c6b4f2ad94f68ac7c50878cb` (2026-09-24).
   - Control instance: `data/fjsp_sdst/fattahi/Fattahi_setup_01.fjs`.
   - Upstream Git blob SHA-1: `6bac8cfaf9f326f6405d5396b9cedd12941aba94`, 131 bytes.
   - `Fattahi_setup_01.fjs` has 2 jobs, 2 machines, 4 operations, alternative counts `[2,2,2,2]`, declared average machine flexibility 2.0 and a 2×4×4 setup tensor. The tiny oracle finds makespan 70 under the documented setup semantics. It does **not** supply due dates.
   - `Fattahi_setup_02.fjs` has the same job/machine/operation dimensions, alternative counts `[1,2,2,1]`, and average machine flexibility 1.5. Its setup matrix includes 1,000,000 sentinel values on some transitions; preserve the source values rather than silently recoding them. The exhaustive Rust oracle finds makespan 112.
   - `Fattahi_setup_03.fjs` expands the tiny oracle to 3 jobs, 2 machines and 6 operations, with alternative counts `[1,2,2,1,2,2]`, declared average flexibility 1.7, a 2×6×6 setup tensor and setup range 6–1,000,000. The exhaustive Rust oracle considers 840 assignment/sequence combinations, keeps 298 acyclic feasible schedules and finds makespan 233.
   - All three fixtures and the MIT license are checked out at the pinned revision, and CI verifies their Git blob IDs and byte-for-byte equality before accepting parser/oracle results.
   - The upstream repository is MIT licensed; see [UPSTREAM_NOTICE.md](../../tools/ev-external-jobshop-parity/UPSTREAM_NOTICE.md) and the copied [license text](../../tools/ev-external-jobshop-parity/UPSTREAM_LICENSE_MIT.txt).

2. **Dynamic dispatching-rule selection (2025).** Marques, Figueira and Guimarães, *Dynamic dispatching rule selection for the job shop scheduling problem*, *Computers & Industrial Engineering* 210 (2025), DOI [10.1016/j.cie.2025.111471](https://doi.org/10.1016/j.cie.2025.111471). They report their tested rule-selection systems outperforming the best individual dispatching rule by over 10% on tardiness in their evaluation, with periodic selection more robust across their tests. This is a source-paper result—not an EV-line result or a gain we assume Symtropy will achieve. Their [public DRSS repository](https://github.com/nuno-a-marques/drss) is a separate reference environment.

## What the Rust parity tool currently guarantees

- Pins the exact upstream repository revision, instance path, repository license and upstream instance blob.
- Strictly parses the tiny FJSP-SDST instance: header, per-job operation count, eligible-machine/processing-time pairs, then a complete square setup matrix per machine.
- Rejects missing/extra setup rows, wrong matrix widths, duplicate machine alternatives, out-of-range machine IDs, zero processing times, negative setup times, undeclared dimensions and malformed integer values.
- Parses the third header token as **average machine flexibility** rather than an operation-count limit. The pinned cases verify 2.0 for case01, 1.5 for case02, and 1.7 for case03 against the observed alternative counts rounded to one decimal; mismatches fail closed.
- Verifies all frozen cases independently: case01 alternative counts `[2,2,2,2]`, setup tensor `[2,4,4]`, setup values 3–8; case02 counts `[1,2,2,1]`, tensor `[2,4,4]`, setup values 3–1,000,000; case03 counts `[1,2,2,1,2,2]`, tensor `[2,6,6]`, setup values 6–1,000,000.
- Emits deterministic structural/provenance reports and exhaustively enumerates every eligible-machine assignment and machine sequence for each supported tiny case. It rejects cyclic machine/job precedence combinations and uses the upstream CP-SAT model's anticipatory setup semantics: processing starts at max(job readiness, machine-predecessor completion plus transition setup); setup may overlap waiting for the job predecessor, and the first operation on a machine has zero setup.
- Known-answer results on the three frozen cases: case01 has 120 assignment/sequence combinations, 52 acyclic feasible schedules and exact makespan 70; case02 has 20 combinations, 11 feasible schedules and makespan 112; case03 has 840 combinations, 298 feasible schedules and makespan 233. Best assignments, machine sequences and per-operation setup/start/completion traces are emitted. Exactness applies only to these finite tiny cases and the documented setup semantics. CI also runs the pinned upstream CP-SAT model / OR-Tools 9.11.4210 for each case and requires optimal status and matching makespan; the result artifact records solver/input/model identity. Do not call the CP-SAT comparison passed until exact-head CI completes.
- No dispatch-rule policy is executed. This does not reproduce a dynamic dispatch-rule result, compare with SOTA, model due dates, or establish industrial outcomes.

## Next benchmark sequence

1. **Input parity and tiny exact oracle:** the current experiment pins two 2-job/2-machine/4-operation cases and one 3-job/2-machine/6-operation case. A header misinterpretation treated decimal average-flexibility metadata as an operation-count limit and blocked the 1.5-flexibility case. The corrected parser checks that metadata against observed machine alternatives; all three Rust exact-oracle results are independently checked against pinned upstream CP-SAT before this benchmark stage counts as reproduced.
2. **Reproduce reference baselines:** execute upstream dispatching-rule and exact/CP-SAT/MILP methods on the same selected instances. Preserve version, configuration, objective, solver limits, seeds and raw outputs. Require independent agreement with the tiny exact oracle before expanding the corpus; do not compare wall-clock values across unlike hardware as quality scores.
3. **Add a native Symtropy action adapter:** map the upstream ready-operation and machine-eligibility state into a versioned JSON contract, run the candidate policy against the *same* upstream simulator and constraint checker, and log decision latency plus chosen action. The current EV-line `minimize_changeover` rule is not presumed equivalent to machine/operation-dependent setup: implement that mapping explicitly.
4. **Evaluate dynamic tardiness separately:** use a source whose instances define arrivals, due-date construction and tardiness scoring. The EV-line fixture lacks those semantics, so EDD, minimum slack, critical ratio and tardiness results are unsupported there. Preserve the upstream score definition exactly.
5. **Predeclare statistics:** use paired replications only when common exogenous streams are genuinely shareable; use at least 100 predeclared replications per dynamic regime as an initial target, forward-time validation/holdout, multiplicity correction for the primary hypothesis family, and published adverse cases. Keep fairness, setup time, constraint violations and rule-switch/runtime overhead disaggregated.

## Benchmark goals—not claims

- **Correctness gate:** reproduce upstream instance parsing, precedence, eligibility, setup matrix and exact-oracle scores before policy comparison.
- **Baseline gate:** reproduce selected upstream baseline scores within documented tolerances before introducing a new controller.
- **Generalization gate:** show gains on held-out cases and regimes without tuning thresholds on the holdout; report corrected tests, confidence intervals and practical effect sizes.
- **Ambitious target:** on a genuinely matched dynamic-tardiness suite, aim to meet or exceed the published study's reported greater-than-10% improvement over the best static rule. This is an aspirational research target only. It counts only after reproducing the study's case construction, split, score and reference baselines; otherwise, the comparison is not equivalent.
- **Industrial gate:** no production claim before measured site traces, calibration/validation, uncertainty coverage, operator constraints, cybersecurity and a prospective shadow-mode/pilot evaluation.

## Claim ceiling

This v0.2 contribution implements finite exhaustive makespan oracles for three tiny public instances and configures independent upstream CP-SAT cross-checks under the explicitly stated setup semantics. No new exact-head CI pass is claimed until the branch workflows complete. It does not run a dispatching policy, reproduce an external paper, compare Symtropy with SOTA, estimate factory output or due-date performance, or authorize plant decisions.


## First executable schedule-construction comparison (PR #2700)

The same pinned instances now have an optional `--dispatch-baselines` mode, separate from the structural parity report. It schedules all operations with deterministic serial schedule generation and compares four simple rules with the exhaustive optimum:

- `job_index_order`: lower job ID, then operation position; projected completion and IDs break ties.
- `shortest_processing_time`: shortest eligible operation-machine processing time, then projected completion and IDs.
- `minimum_setup_plus_processing`: smallest machine-specific transition setup plus processing duration, then projected completion and IDs.
- `earliest_completion_time`: smallest projected processing completion, then setup-plus-processing and stable identifiers.

Machine assignment is integrated into candidate selection; only operations whose job predecessors are already placed can be chosen. Every schedule is independently checked for operation completeness/uniqueness, eligible machine and processing time, job precedence, setup-transition identity, first-operation zero setup, and machine non-overlap under the pinned anticipatory setup semantics. Reports include exact makespan, absolute and relative optimality gaps, and the full schedule trace for each policy.

CI checks against exact makespans 70, 112 and 233, rejects a greedy result below the exhaustive optimum, and repeats every report byte-for-byte. This measures four explicit baseline heuristics on only three tiny static cases; it does not reproduce the dynamic due-date/tardiness DRSS study and does not support a SOTA or factory-performance claim. Next valid work is scaling the matched protocol to a larger independently pinned benchmark slice with reference-rule parity and held-out instances.
