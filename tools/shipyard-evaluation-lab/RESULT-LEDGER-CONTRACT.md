# Shipyard Result Ledger Contract v0

Status: research protocol; this document asserts no benchmark outcomes.

## Why a ledger, not a leaderboard

A leaderboard tends to retain successful scores and hide failures. A decision-quality benchmark must retain every planned episode, terminal status, hard-gate result, and exact input identity. Candidate policies may not write their own evaluator metrics. The ledger is append-only at the run-record level; corrections create a superseding record with a reason rather than rewriting history.

## Required record hierarchy

1. **Benchmark release** — corpus revision, contract versions, expected episode IDs, partitions, scenario family tags, provenance and corpus digest.
2. **Method registration** — policy identity, source/binary digest, configuration digest, baseline class, tuning partition, declared parameters and resource budget.
3. **Episode assignment** — one method × one immutable episode × one paired replicate; initial-state and tape digests, seed/RNG definition, hardware profile, evaluator version and budget.
4. **Raw run record** — terminal status, action-trace digest, policy-visible input digest per decision, resource usage, artifact digests, failure reason.
5. **Independent evaluation** — geometry, swept path when modeled, precedence, resource reservations, required-work coverage, non-anticipativity and receipt-integrity results, each with checker revision and diagnostics.
6. **Metrics record** — metric identifier, value, unit, aggregation boundary, recomputation source, uncertainty/quality qualifier and validity status.
7. **Analysis release** — frozen inclusion rules, paired comparisons, confidence intervals, effect sizes, multiplicity policy, Pareto summaries, and all exclusions with reasons.

## Terminal status taxonomy

Every assigned episode must end in exactly one primary status:
- **feasible**: all required independent gates passed and mandatory work is accounted for.
- **infeasible_proven**: the declared exact proof procedure established infeasibility within its supported model and completed proof budget.
- **budget_exceeded**: declared compute/evaluation budget was consumed without an accepted outcome.
- **timed_out**: external wall-clock limit ended the run; this does not imply proof of infeasibility.
- **invalid_input**: schema, digest, model, or contract validation failed before valid evaluation.
- **incomplete**: required trace, scenario, metric, or checker output is missing.
- **invalid_candidate**: action/plan violates an action-interface or model constraint.
- **evaluator_error**: independent evaluation failed or contradicted its own declared protocol.
- **not_run**: the assignment exists but execution did not start; the reason is mandatory.

A feasible result can still have poor performance. An infeasible_proven result is not the same as a heuristic failing to find a solution. A run that never completed is not a zero-cost solution and may not be dropped.

## Hard-gate admission rule

The performance analysis must report assigned episode count, complete evaluation count, feasible count and rate, every terminal-status count, independent hard-violation counts by type, and missing receipt/metric counts.

A method with any accepted schedule containing a hard violation is ineligible for the feasible-schedule performance frontier for that release. Still retain and report the failure. Never turn a failed run into an infinite or zero objective without separately reporting that transformation and its rationale.

## Paired comparison design

For every matched comparison, the pairing key is the tuple of benchmark release, episode ID, replicate ID, initial-state digest, tape digest, and budget profile.

Paired methods must share these values and the same information-access contract. Method binary/config may differ. If one method times out, retain that outcome and its paired partner.

Report:
- paired per-episode differences and the full difference distribution;
- median and interquartile range;
- mean difference only when meaningful for the metric;
- confidence interval and method/assumptions used;
- fraction of pairs where each method wins, ties, or loses;
- results stratified by disturbance family and size tier;
- worst case and tail behavior, not only overall averages;
- multiplicity and exploratory-analysis decisions.

For stochastic methods, replicate seeds are predeclared and matched by episode. A shared seed does not guarantee common random numbers if algorithms consume RNG differently; the event tape must therefore be externalized and fixed.

## Metrics and directionality

Each metric declares unit, aggregation, direction (minimize, maximize, or target range), and whether it is a hard gate or descriptive outcome.

Suggested metrics:
- makespan (minimize);
- total and empty vehicle distance/time (minimize);
- actual polygon utilization (maximize subject to all hard constraints);
- rehandles and schedule changes (minimize);
- time to first independently verified feasible action (minimize);
- recovery completion rate (maximize);
- mandatory-work omission (hard gate);
- geometric/resource/precedence/causality violations (hard gates, target zero);
- p50/p95/p99 decision latency and peak memory (minimize under a fixed measurement method);
- bounded-oracle objective gap on supported tiny cases (minimize, with solver status/bound).

Do not aggregate unlike metrics into a single score for primary claims. If a stakeholder needs a scalar utility, publish the weight vector and complete Pareto table; label it a decision-specific utility, not universal superiority.

## Pareto reporting

- Freeze metric directions and any reference point before held-out evaluation.
- Report nondominated feasible methods and the metric vector for each.
- Hypervolume is optional and only comparable with identical normalized metric definitions and a fixed reference point.
- Keep infeasible, incomplete and timed-out outcomes visible outside the feasible frontier.
- Do not normalize against the best observed candidate after seeing test results.

## Exactness and proof labels

Use separate labels:
- **heuristic_feasible**: independently verified feasible, no optimality claim.
- **bounded_gap**: solver supplied a valid bound; report incumbent, bound, gap, termination status and model.
- **optimal_proven**: exact proof completed for the declared formulation and instance.
- **infeasible_proven**: exact proof completed for the declared formulation and instance.
- **unknown**: proof not completed or unsupported.
- **not_applicable**: metric/model does not apply.

Never upgrade a timeout or unsupported model to a proof claim. Exactness is always relative to the modeled constraints and geometry/integer domain.

## Integrity controls

- Unique run ID and immutable benchmark/method/episode IDs.
- Exact-byte digests for source artifacts and separate digests for normalized semantic payloads.
- Independent evaluator output; candidate-produced metric fields are untrusted.
- Append-only correction via superseding record with parent record ID and reason.
- Strict schema and enum validation; unknown fields/version reject.
- Every expected assignment appears in the ledger, including runs not started, with explicit status and reason.
- Reports derive from the ledger via versioned analysis scripts; manual row deletion is forbidden.
- The final report lists every omitted episode and the predeclared exclusion rule that applied.

## Publication and industry-comparison gates

Before claiming superiority:
1. Freeze the test corpus and metric definitions before evaluating candidates.
2. Align constraints, units, objective boundaries, observation access, hardware, budgets and stopping rules with the comparator.
3. Distinguish a reproduction of a published case from a new synthetic benchmark.
4. Record source revisions, raw records, analysis code, and environment.
5. Report effect sizes and uncertainty, not just a point estimate.
6. State the tested envelope and exclusions; do not generalize synthetic wins to live yards.
7. If results fail to improve, publish the negative result and trade-offs.

## Synthetic example

The companion fixture illustrates record shape only. It is marked not_run and must never count as a benchmark outcome or evidence of policy quality.

## Claim boundary

A conforming ledger makes failures and comparisons auditable. It does not itself validate physics, geometry, simulator calibration, or operational relevance.
