# Maritime Decision-Time Replay (MDTR) v0

**Status:** research specification; synthetic-only; not operationally qualified.  
**Owner:** Symtropy research.  
**Related:** #7504, #7488, #3683, #7400, #6997, #7231, #6662.

## Thesis

Evaluate the quality of a *justified decision*, not merely the quality of a schedule.

A policy may use only observations available, rights-cleared, and decision-usable at cutoff \(t_d\). It must return either a candidate action with typed evidence references or an explicit abstention. The evaluator alone holds realized scenario truth \(\omega\).

The policy objective is to minimize service regret subject to hard physical, authority, participant-floor, evidence and cargo-conservation constraints. A low objective value never compensates for a hard-gate violation.

## Episode boundary

An episode is identified by immutable:
- scenario/topology revision and operating regime;
- cargo/vessel cohort and quantity units;
- policy and candidate-grid revisions;
- decision cutoff and timezone/clock uncertainty;
- source-event manifest and as-of information-set digest;
- physical, authority and participant-floor contract revisions;
- objective/loss contract and evaluator split.

Never join records by display names or timestamps alone. Use stable subject, episode, event, artifact revision and handoff identities.

## Evidence visibility

An event can affect a decision only when all conditions hold:
1. source/artifact rights permit the intended research use;
2. event identity and payload revision are bound;
3. publication and retrieval visibility are at or before the cutoff;
4. event is within its valid-time and decision-use interval;
5. source/dependency cluster is represented in the frozen manifest.

Event time is not knowledge time. A later correction may affect a later decision, but cannot rewrite the earlier replay. A digest establishes byte identity, not truth or source independence.

## Policy interface (conceptual)

Input: frozen episode descriptor, admissible observations, query budget/deadline, candidate actions, and typed oracle interfaces.

Output:
- `action_id` or `ABSTAIN`;
- evidence/receipt references;
- policy version and deterministic trace;
- declared objective vector and bounded uncertainty;
- stable reason codes for every gate or abstention.

The policy must not receive evaluator-only realized truth, hidden future arrivals, post-cutoff corrections, or evaluation labels.

## Gate semantics

The evaluator applies independent, non-compensating gates:
- evidence visibility, identity, rights and claim scope;
- physical feasibility;
- authority and revocation;
- participant service floors;
- cargo conservation and custody/receiver requirements;
- statistical validity for any probabilistic claim.

Known violation => reject. Missing/unknown/not-evaluable => abstain. A passing result means only eligible for the declared decision review; it does not grant execution authority.

## Regret

For episode \(\omega\), let \(I_{t_d}\) be the admissible information set and \(A_f(\omega)\) the evaluator's physically feasible and authorized action set. Report:

\[
R_{k}(\pi,\omega)=J_k(\pi(I_{t_d}),\omega)-
\min_{a\in A_f(\omega)}J_k(a,\omega)
\]

for each objective axis \(k\), not only one weighted scalar. If the policy abstains, score both the operational fallback and the abstention cost under a preregistered rule. The clairvoyant optimum is an information-relaxation bound, not a deployable baseline.

Report regret decomposed into receiver-service delay, missed connections, cost, energy, disruption recovery, participant displacement and infeasibility. Never compare policies using different information cutoffs.

## Baseline tournament

Use matched frozen instances, observation cutoffs, hardware/compute budgets and stopping criteria:
- FCFS, greedy, current dispatch (when available);
- rolling-horizon scheduling and local repair;
- exact MILP/CP-SAT on enumerable instances;
- two-stage robust optimization / C&CG;
- ALNS and RL-enhanced ALNS;
- integrated GNN/MAPPO berth-QC-AGV methods;
- branch-and-price where applicable;
- constrained RL, active sensing and bounded contingent recovery;
- strongest feasible same-information baseline.

Published results without matched instances are contextual literature comparisons only.

## Metrics

Report distributions, uncertainty and failed/timeout cases:
- exact-oracle agreement on tiny exhaustive instances;
- false-safe hard-constraint rate; unauthorized-action rate;
- post-cutoff leakage and historical replay errors;
- objective-wise decision regret and regret tails;
- receiver-accepted on-time service and p50/p90/p95 chain time;
- cargo duplication/drop, unknown/censored outcome and custody-closure rates;
- value of information net of query cost/latency;
- abstention coverage vs selective risk;
- runtime, optimality gap/bound, timeout and incumbent quality;
- mutation score and surviving mutations.

## Falsification suite

Freeze seeds, candidate grid, objective, splits and exclusions before evaluation. Split by topology, disruption family, temporal regime and source-dependency cluster. Include:
- late corrections, missing/duplicate/conflicting events and timezone ambiguity;
- source outage and shared failure dependencies disguised as separate feeds;
- stale query past decision-use deadline and query cost omitted;
- permission revoked between proposal and action;
- physical and authority receipts bound to different revisions;
- participant-floor violation masked by aggregate improvement;
- duplicated cargo, terminal aliases, transfer legs, wrong receiver/handoff;
- censored/missing cargo outcomes treated as success;
- candidate-grid reorder or calibration/evaluation cluster leakage.

Negative controls must detect or explicitly bound each mutation. Publish all survivors.

## Industry scorecards

Keep separate:
1. **CPPI-compatible port-level vessel time:** correct port-limit/anchorage arrival and departure-from-berth boundaries, port aggregation and workload stratification.
2. **Terminal/cargo-service chain:** unique cargo denominator, quantity-safe units, custody, receiver acceptance, inland leg and tail service.

Use DCSA event semantics at interoperability boundaries. Treat terminal operating systems such as N4 as execution/optimization systems and integration partners, not straw-man baselines. Commercial datasets require rights and license review.

## Initial targets (not results)

- 100% exact agreement on exhaustive tiny instances.
- Zero false-safe and unauthorized admissions in the frozen synthetic suite.
- Zero post-cutoff leakage and modeled cargo duplication/drop.
- Median <=5% optimality gap on applicable mid-scale instances under a fixed published budget.
- Aspirational 5–10% improvement in verified receiver service or end-to-end p90 delay vs strongest matched baseline, with every participant floor preserved.

Finite-suite zero failures do not establish zero real-world risk. Targets must be reported with confidence intervals and the complete test denominator.

## Claim ceiling and field transition

This v0 is a benchmark contract, not a live-port model. Passing it does not prove source authenticity, oracle independence, operational safety, or real service improvement.

Field evaluation begins read-only, rights-cleared and independently adjudicated. Any later operational mutation requires separate authority, reconciliation, and a distinct safety case. No berth booking, dispatch, TOS mutation, custody transfer or payment is authorized here.

## Next executable increment

1. Define a dependency-light frozen episode JSON schema.
2. Add tiny hand-authored exhaustive episodes and expected outcomes.
3. Implement deterministic as-of visibility and cargo conservation checks.
4. Emit a stable replay receipt.
5. Add a separately implemented evaluator/negative-control suite.
6. Only then add simulator scale and algorithm adapters.
