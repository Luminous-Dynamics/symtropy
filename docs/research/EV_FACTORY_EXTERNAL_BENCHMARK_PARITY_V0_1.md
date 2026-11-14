# External job-shop benchmark parity contract v0.1

**Related dispatch-policy evaluation:** [EV-FACTORY-DESIGN-003 #2445](https://github.com/Luminous-Dynamics/symtropy/issues/2445)  
**Current EV-line experiments:** [PR #2443](https://github.com/Luminous-Dynamics/symtropy/pull/2443), [age fairness override #2457](https://github.com/Luminous-Dynamics/symtropy/pull/2457), [threshold sweep #2467](https://github.com/Luminous-Dynamics/symtropy/pull/2467), [multiplicity correction #2545](https://github.com/Luminous-Dynamics/symtropy/pull/2545)  
**Status:** input-parity control only; the external scheduler has not been run by this harness.

## Why a second benchmark stratum is required

The EV-line simulator and external job-shop suites answer different questions. The EV-line model reports good units/hour, changeovers, scrap/rework, queue waits, finite-buffer blocking and release-order inversions. A static flexible job shop with sequence-dependent setup times (FJSP-SDST) evaluates makespan under operation precedence, machine alternatives and machine-specific setup transitions. A dynamic due-date benchmark evaluates tardiness under a declared arrival/due-date process. Do not collapse them into one leaderboard or reuse a similarly named policy without matching its semantics.

Two public sources provide complementary references:

1. **Unified static/dynamic job-shop benchmark (2026).** Reijnen et al., *Job shop scheduling benchmark: Environments and instances for learning and non-learning methods*, *Annals of Mathematics and Artificial Intelligence* (2026), DOI [10.1007/s10472-026-10007-3](https://doi.org/10.1007/s10472-026-10007-3). The [public benchmark repository](https://github.com/ai-for-decision-making-tue/Job_Shop_Scheduling_Benchmark_Environments_and_Instances) includes JSP, FSP, FJSP, FAJSP, FJSP-SDST and online-arrival variants, plus exact, heuristic and learning-based methods.
   - Pinned repository revision: `d088a26582f5cb79c6b4f2ad94f68ac7c50878cb` (2026-09-24).
   - Control instance: `data/fjsp_sdst/fattahi/Fattahi_setup_01.fjs`.
   - Upstream Git blob SHA-1: `6bac8cfaf9f326f6405d5396b9cedd12941aba94`, 131 bytes.
   - The instance has 2 jobs, 2 machines, 4 operations, two eligible machines per operation, and a 2×4×4 setup-time tensor. Its primary objective for this parity control is makespan. It does **not** supply due dates. CI checks out only this upstream file and its license at the pinned revision, verifies the blob identity and byte-for-byte equality with the attributed local fixture, then validates the parsed structure.
   - The upstream repository is MIT licensed; see [UPSTREAM_NOTICE.md](../../tools/ev-external-jobshop-parity/UPSTREAM_NOTICE.md) and the copied [license text](../../tools/ev-external-jobshop-parity/UPSTREAM_LICENSE_MIT.txt).

2. **Dynamic dispatching-rule selection (2025).** Marques, Figueira and Guimarães, *Dynamic dispatching rule selection for the job shop scheduling problem*, *Computers & Industrial Engineering* 210 (2025), DOI [10.1016/j.cie.2025.111471](https://doi.org/10.1016/j.cie.2025.111471). They report their tested rule-selection systems outperforming the best individual dispatching rule by over 10% on tardiness in their evaluation, with periodic selection more robust across their tests. This is a source-paper result—not an EV-line result or a gain we assume Symtropy will achieve. Their [public DRSS repository](https://github.com/nuno-a-marques/drss) is a separate reference environment.

## What the Rust parity tool currently guarantees

- Pins the exact upstream repository revision, instance path, repository license and upstream instance blob.
- Strictly parses the tiny FJSP-SDST instance: header, per-job operation count, eligible-machine/processing-time pairs, then a complete square setup matrix per machine.
- Rejects missing/extra setup rows, wrong matrix widths, duplicate machine alternatives, out-of-range machine IDs, zero processing times, negative setup times, undeclared dimensions and malformed integer values.
- Verifies the frozen structure: 2 jobs, 2 machines, 4 operations, `[2,2,2,2]` alternatives, `[2,4,4]` setup tensor and setup values 3–8.
- Emits a deterministic structural report with source provenance and an explicit claim ceiling.
- Has no scheduler, machine simulator, due-date field or schedule-quality result. It cannot establish policy parity, improvement or SOTA.

## Next benchmark sequence

1. **Input parity first:** keep the pinned tiny instance as a parser/setup-tensor regression control; add more upstream cases only after their semantics are independently inspected.
2. **Reproduce reference baselines:** execute upstream dispatching-rule and exact/CP-SAT/MILP methods on the same selected instances. Preserve version, configuration, objective, solver limits, seeds and raw outputs. For tiny cases, use exact results as an oracle/lower bound; do not compare wall-clock values across unlike hardware as quality scores.
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

This v0.1 contribution verifies the input contract for one small public benchmark instance. It does not run a scheduler, reproduce an external paper, compare Symtropy with SOTA, estimate factory output or due-date performance, or authorize plant decisions.
