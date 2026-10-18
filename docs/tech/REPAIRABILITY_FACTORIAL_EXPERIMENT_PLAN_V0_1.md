# Interaction-Aware Repairability Factorial Experiment Plan v0.1

**Status:** design proposal only; protocol not frozen, no physical trials started, no observed outcomes and no winner.  
**Tracker:** [APPLIANCE-DOE-001 #1910](https://github.com/Luminous-Dynamics/symtropy/issues/1910). Related: [washer architecture](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/docs/tech/WASHER_REFERENCE_ARCHITECTURE_V0_1.md), [fault-to-repair curriculum](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/docs/tech/WASHER_FAULT_TO_REPAIR_BENCHMARK_CURRICULUM_V0_1.md), [cross-appliance morphology](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/docs/tech/CROSS_APPLIANCE_ARCHITECTURE_MORPHOLOGY_AND_HYBRIDS_V0_1.md), and [restoration outcomes #1908](https://github.com/Luminous-Dynamics/symtropy/issues/1908).

## 1. Paradigm shift: test interactions, not isolated features

The morphology list has several promising patterns—service spine, small justified repair unit, and evidence-guided diagnosis—but their value may be interactive. Access can be easy while the repair unit is uneconomic; a small part can be cheap but hard to diagnose; better diagnostics may prevent unnecessary large-assembly replacement. Selecting each pattern independently risks missing that design-space structure.

The access factor must use a time-based protocol, not just fastener/step counts. Dangal et al.'s 2026 DaRT study reports validation against independent disassembly data for vacuum cleaners, washing machines and smart TVs, with better performance than simpler step-count methods ([doi:10.1007/s44498-026-00150-9](https://doi.org/10.1007/s44498-026-00150-9)). Record disassembly and reassembly separately, including tool changes, retries, handling, and errors; this method still does not prove restoration or recurrence. A 2025 observational study with 26 participants found mismatches between calculated indices and perceived repairability for two small appliances, highlighting access and fault-identification friction ([doi:10.1016/j.spc.2025.05.024](https://doi.org/10.1016/j.spc.2025.05.024)). Thus, the trial must separate timed technician service tasks from safe user-level task completion/referral and must not project a small user study's numerical differences to other products.

This proposal uses a **2×2×2 full factorial** on a common *unpowered service mock-up* to estimate the main effect of each factor and their interactions:

| Factor | Level 0 | Level 1 | Primary question |
|---|---|---|---|
| A — access architecture | Conventional access | Service spine | Does a deliberate service path reduce access and reassembly burden? |
| B — repair granularity | Assembly-level replacement | Smallest justified service unit | Does the repair unit reduce material/cost without unacceptable complexity or rework? |
| C — diagnostic support | Baseline documentation | Evidence-guided test-before-replace | Do discriminating tests reduce wrong-part replacement and rework? |

Eight cells cover every combination. The factor cell `DOE-000` is the common mock-up baseline for estimating interactions. The commercial `W0-RETAIL-CONTROL` remains a separate contextual baseline; it does not define the factorial coefficients unless a later protocol justifies a matched equivalence.

## 2. Feasibility pilot, not a powered superiority claim

The proposed planning floor is **3 independent technicians × 4 standardized task families × 8 cells = 96 task attempts**, or 12 planned attempts per cell. This is a variance/feasibility pilot, not an assumption that 96 trials are sufficient to prove superiority. Use the pilot to estimate between-technician/task variation and measurement noise, then freeze a defensible sample size and acceptance margin before the confirmatory trial.

Initial task families:
- Filter / foreign-object access.
- Door, lock and seal service.
- Drain and pump diagnosis.
- Bearing / tub / drum access planning.

The order of cell trials is randomized within technician-by-task blocks, and session order is recorded to detect learning effects. Use privacy-preserving technician tokens. An independent reviewer should assess outcome receipts without seeing the favored architecture label where practical.

## 3. Endpoints and analysis plan

**Primary endpoint 1: verified restoration probability per started task.** The denominator includes all task attempts that actually start, including failed, abandoned and safety-stopped attempts. Correct diagnosis-only referral before work starts is reported separately, not recoded as repair success.

**Primary endpoint 2: time to verified restoration.** Record active work, diagnosis, disassembly, reassembly, waiting and verification separately. Failed repair, abandonment and safety-stop are competing outcomes; only a task still unresolved when observation ends is right-censored. Do not compare only the successful jobs without also reporting task success probability.

Pre-register the main effects A/B/C and all interactions A×B, A×C, B×C and A×B×C. Report per-cell denominators, median/P90 times, uncertainty intervals, rework, wrong-part replacement, part/assembly mass, parts/labour/downtime cost, reassembly errors and diagnostic referrals. Use a blocked model with technician, fault-task and session order; do not treat repeated attempts by one technician or on one mock-up as independent people/products. The pilot is primarily for feasibility and variance estimation, not p-value hunting.

## 4. Hard gates and claim boundaries

- All positive task-restoration claims require explicit post-task function and safety verification receipts.
- A safety stop is a legitimate safe outcome but not a completed repair.
- Lower time, cost or removed-part mass cannot compensate for a failed safety or core-function gate.
- Do not report an energy/water improvement, wash-quality result, product-life result, or product-safety result from an unpowered service mock-up.
- Keep missing and unknown outcomes separate from zero. Preserve failed attempts and the reason each task did not end in verified restoration.
- No cell is a winner because its hypothesis sounds best. A candidate must pass the predeclared gate and have evidence with adequate precision.

## 5. Sequencing and freeze gates

1. Freeze task definitions, safe role boundaries, measurement tools, randomization list, response definitions and exclusions.
2. Run control/mock-up-only practice trials to expose ambiguity, inter-rater disagreement and instrument error; freeze those changes before candidate comparison.
3. Run the 96-attempt feasibility pilot, report its full outcome census and estimate variance.
4. Recalculate the confirmatory sample size and freeze the model, multiplicity handling and practical margins before confirmatory observations.
5. Only after a service-task candidate survives may a physical washer be evaluated under the existing matched-control protocol for successful cleaning/rinsing/spin, energy/water per useful load, leaks, noise, durability and safety. These are separate qualification stages.

## 6. Machine-readable audit

`schemas/appliances/washer_repairability_factorial_plan_v0_1.schema.json` and `fixtures/appliances/washer_repairability_factorial_plan_v0_1.json` declare the eight cells, factors, blocking/randomization policy, endpoints, interactions and proposed benchmark rules. The Rust audit at `tools/household-appliance-bench-audit/src/experiment.rs` rejects missing or duplicate cells, factor-ID mismatches, nonzero outcome observations in a not-yet-run plan, changed denominators, altered interaction lists and any premature winner claim.

A successful schema/Rust audit means only that the *plan contract* is internally consistent. It does not mean the study has been preregistered, approved, run, or powered, and it does not establish a repairability improvement.
