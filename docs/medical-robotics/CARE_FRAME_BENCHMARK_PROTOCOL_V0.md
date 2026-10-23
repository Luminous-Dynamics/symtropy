# Careframe Benchmark and Industry-Comparison Protocol v0.1

**Status:** pre-registered design proposal. No robot is built or physically tested by this document.

## Objective
Determine whether Symthaea/Symtropy improves hospital support tasks compared with real operational baselines, without exchanging human safety, privacy, hygiene, or control for average task throughput.

## Benchmark tasks (simulated first)
- Move sealed supply container from a mapped depot to an identified handoff zone.
- Dock and transfer empty tray to a human operator.
- Locate an explicitly named supply package and verify label/ID with a human-in-the-loop confirmation.
- Navigate doorways/corridors while handling dynamic obstacle appearance and emergency stop signals.
- Establish telepresence only after explicit session authorization; verify privacy shutter and capture indicator state.
- Demonstrate a mannequin-only probe holder as a training task.
- Change tool or calibration revision during an episode and prove that prior plans are invalidated.

The suite excludes diagnosis, medication dispensing, patient transfer, invasive tasks, and autonomous patient-contact activities.

## Baseline design
Compare the current human workflow, a standard cart/workstation, a conventional wheeled mobile manipulator, a fixed-base setup for static work, and the proposed wheeled humanoid upper body. Compare a biped only where a documented task need justifies it. Use the same task, payload, route, map, tool, environmental difficulty, time budget, operator interventions, and endpoint rubric. Report each task category separately.

## Metrics
- Completed tasks / attempted tasks, time-to-completion median and p95, travel distance and energy per completed task.
- Manual interventions, safe aborts, recovery success, blocked false positives and unsafe false-eligible actions.
- Tool identification / latch verification errors; payload retention and drops in simulation and later bench tests.
- Collision/contact count, near misses, stopping latency/distance, limit violations, stale-plan detection, privacy-state errors and authorization violations.
- For simulated privacy tasks, measure prohibited captures attempted and prevented; in human studies, use approved protocols and report participant understanding.
- For hygienic tool/contact surfaces, use only approved surrogate-based protocols at future bench stage; no hygiene claim is accepted from material labels alone.
- Maintenance time, tool-change time, calibration frequency, failure recovery, availability and cost per completed task.

## Benchmark validity audit

Before using a task-suite score as evidence of general manipulation ability, run the separate [Careframe Benchmark Validity Audit](CARE_FRAME_BENCHMARK_VALIDITY_AUDIT_V0.md). It covers shortcut resistance, statistical power/uncertainty, benchmark overfitting, and training/test data-source dependence, drawing on the June 2026 manipulation-benchmark audit and NIST's mobile-robot metrology work.

The benchmark should include trivial policy baselines, randomized layouts and nuisance conditions, a public development corpus, a frozen regression corpus and a separately maintained holdout. Preserve the exact task, environment, tool, model and simulator digests for each result. A high score that can be reproduced by a shortcut baseline narrows the claim; a score from a tuned public set is not a held-out generalization result.

## Statistical and reporting protocol
Freeze candidate revision, baselines, tasks, scorer, scenario digests, resource budgets and acceptance rules before running. Split public development fixtures from a holdout maintained by an independent custodian. Preserve every failure, timeout, refusal and missing result. Report paired effect sizes, confidence intervals, per-scenario distributions and subgroup/task strata. Avoid post-hoc scenario removal and aggregate-score compensation across hard safety gates.

Preliminary software targets are zero false-eligible outcomes in the frozen synthetic suite, complete detection of registered stale-evidence mutations, and 100% fail-closed handling of malformed or out-of-scope tool manifests. These are test-suite acceptance targets only, not physical safety rates.

## Novel contribution hypothesis
A potential contribution is not merely the humanoid body: humanoids already exist in assistive-care research. The hypothesis is a shared, revision-bound capability/tool contract with independent qualification receipts and strict separation between planning, safety authority, and clinical authorization. Test whether this reduces qualification time, integration errors and unexpected recovery failures across multiple embodiments and tools, compared with one-off platform implementations.

## Evidence ladder
1. **Schema/contract** — synthetic manifest/episode conformance only.
2. **Simulation** — modeled task performance with explicit model assumptions.
3. **Bench/HIL** — measured prototype behavior with sensors and fault injection.
4. **Mock clinical workflow** — usability / workflow findings in a non-patient setting with appropriate oversight.
5. **Clinical research / deployment** — only when an independently reviewed risk-management, quality, ethics, legal and regulatory pathway permits it.

Never promote evidence from one stage to the next by inference.
