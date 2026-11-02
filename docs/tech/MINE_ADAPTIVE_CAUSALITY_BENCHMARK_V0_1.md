# MINE-ADAPTIVE-CAUSALITY-001 — non-anticipative policy trace benchmark v0.1

## Scope

This standalone Rust contract checks causal admissibility of decision traces for sequential mine-planning policies. It does not solve a scheduling problem and does not score economic utility. It composes with MINE-SYSTEM-001 (#1933), ORE-VOI-001 (#8060), ORE-FEED-BLEND-001 (#2002), and MINE-REFERENCE-001 (#2015) instead of duplicating their owners.

## Paradigm shift

The unit of evaluation becomes a **policy trajectory under a known observation history**, not a static plan produced with future knowledge. A policy can only use results whose delivery time is at or before the decision time, whose provenance is present, whose QC passes, and whose visibility is permitted. The time a sample was collected is not the time its assay became available.

Counterfactual replay makes this stricter: if two worlds have identical observable histories before hour 2 but the assay outcome arrives at hour 2, their logged policy actions before hour 2 must be identical. If the action changes earlier, the run has leaked future state—even if its final score looks better.

## Research grounding

- Armstrong et al. (2021), *Adaptive open-pit mining planning under geological uncertainty*, describes rolling-horizon replanning as new grade information arrives. It reports an average advantage over a non-adaptive two-stage policy for studied cases but explicitly finds that rolling horizon does not win on every geological realization: https://doi.org/10.1016/j.resourpol.2021.102086
- Khalifi et al. (2026 preprint), *Adaptive mine planning under geological uncertainty: A POMDP framework for sequential decision-making*, proposes sequential extraction/routing decisions with belief updates instead of a plan calculated once before future observations: https://arxiv.org/abs/2605.13702. Its headline performance is an external research claim to reproduce—not a fixture answer to assume.
- Jeuken & Forbes (2024), *The Value of Drilling—A Chance-Constrained Optimization Approach*, motivates valuing tests by downstream constrained decisions, not information gain in isolation: https://doi.org/10.1007/s42461-024-01061-8
- Andrade & Dimitrakopoulos (2024), *Integrated Stochastic Underground Mine Planning with Long-Term Stockpiling*, motivates retaining stockpile inventory inside the planning state: https://doi.org/10.3390/min14020123

## Frozen synthetic trace controls

The 11 authored scenarios cover:
- a qualified result delivered before a decision;
- a result used before its delivery timestamp;
- a sample collected earlier but whose assay is not delivered until later;
- a QC-rejected result;
- a missing source record;
- a safe hold when no admissible result is ready;
- evaluator-only truth mistakenly used as a policy input;
- two counterfactual futures with the same history and same pre-fork action but different later outcomes;
- a planted pair whose policy actions diverge before the futures become observable.

The validator binds event ID, sample/lot ID, observation time, delivery time, qualification, policy/evaluator visibility, source record, QC state, and result token. Each trace receipt binds the exact corpus SHA-256 and source commit.

## Acceptance gates

- Exact dispositions and reason codes match all 11 frozen known-answer scenarios.
- Zero policy use of a future-delivered or evaluator-only event.
- Zero use of evidence without source/QC/qualification where the trace claims it was used.
- 100% same-history counterfactual action agreement up to the fork time.
- A deliberate pre-fork disagreement is rejected in both members of its pair.
- A counterfactual pair must use the same pre-fork delivered-event history, have distinct branch IDs, and actually diverge after the declared fork.
- Unknown fields, duplicate IDs, bad timestamps, duplicate event references, missing events, and out-of-order decisions fail closed.
- Replay receipts bind exact trace bytes and source commit; identical corpus bytes / build / commit yield deterministic receipt identity.

## Next benchmark ladder

1. v0.1 causal trace conformance only.
2. v0.2 one-step decision utility with evaluator-only outcomes and a separate policy input channel.
3. v0.3 rolling-horizon vs static / two-stage / decision-aware measurement baselines on paired held-out synthetic realizations.
4. v0.4 inject prior misspecification, delayed/failed measurements, plant outages, stockpile interactions, energy/water/contaminant limits, and test turnaround costs.
5. v1 external research reproduction only when model assumptions, uncertainty generation, objective basis and data rights are explicit.

## Later policy-quality metrics

Report separately: average utility/NPV-like value, paired regret to a clairvoyant upper bound, downside CVaR, probability and severity of hard-constraint breach (target zero in the contract layer), test cost, time-to-information, replan churn, unmet production demand, energy/water/contaminant breaches, stockpile inventory, runtime and optimality bounds. A perfect-knowledge plan is an upper bound—not an operationally deployable baseline. If a rolling-horizon policy wins on average but loses on some realizations, preserve that distribution rather than report just the mean.

## Claim ceiling

This public fixture is synthetic and not hidden from the evaluator. It verifies trace causality only; it says nothing about mine revenue, geology prediction, real reserve quality, worker safety, or industrial performance. Passing this gate is necessary before comparing policy quality, but it is not sufficient to prove policy quality.
