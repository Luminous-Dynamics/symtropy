# Bounded Exact Oracle for Seismic Repair Scheduling — v0.3

Status: experimental scheduling benchmark. The oracle evaluates small, synthetic, fixed-duration repair scenarios; it is not a tunnel recovery model, construction plan, or re-entry authority.

## Why add an exact reference?

The v0.2 scheduler is a deterministic parallel greedy heuristic. Its makespan lower bound is useful, but excess over a lower bound is not an optimality gap unless that bound is tight. To compare scheduling policies honestly, small instances need a reference optimum that is exact under the same declared task graph, duration, precedence, and renewable crew-pool assumptions.

This increment adds the repair-optimality-benchmark executable, a dependency-free Rust binary. It enumerates all precedence-feasible task orders, constructs an earliest-start serial schedule for every order, and takes the minimum makespan. In the classical fixed-duration resource-constrained project scheduling model, the serial schedule-generation scheme over precedence-feasible orders spans the active schedules; exhaustive enumeration therefore supplies an exact makespan for each bounded fixture. The broader scheduling literature discusses this relationship and the difficulty of scaling exact methods (see Kolisch and later RCPSP surveys).

Reference: *Fifty years of research on resource-constrained project scheduling explored from different perspectives*, European Journal of Operational Research (2025), https://doi.org/10.1016/j.ejor.2025.03.024.

## Contract and limits

- Exact search is limited to at most nine tasks. Larger inputs fail closed rather than silently falling back to a heuristic or presenting an unproven result as exact.
- Tasks are non-preemptive, durations are fixed, precedence edges are deterministic, and crew pools are renewable capacity constraints keyed by declared skill.
- The objective is total work makespan. This is not, by itself, a stochastic service-recovery distribution or a probability of meeting a recovery target.
- The oracle consumes user-supplied data. It does not infer damage, fragility, repair durations, mobilization delays, travel times, shift calendars, spatially correlated failures, utility interdependencies, hazard likelihoods, or crew availability from real incidents.
- Every fixture in this increment is synthetic. A bounded exact result verifies the scheduling implementation for that fixture; it is not empirical validation, a tunnel safety finding, or evidence of industry-wide superiority.

The separate service-ready gate remains unchanged: a completed inspection task does not substitute for an explicit external re-entry authorization. The benchmark’s hand and generated fixtures set that authorization only to exercise software scheduling behavior.

## Benchmark contents

1. A hand-checkable case is designed to expose an ID-ordered greedy loss: the greedy schedule has a 21-hour total-work makespan, while exhaustive enumeration has a 20-hour minimum. The one-hour / 5% difference is a synthetic regression target; the unit test asserts it rather than treating it as a field result.
2. Fifty deterministic, seeded fixtures contain three to six repair tasks plus a post-event inspection task. Every fixture is solved by exhaustive order enumeration and compared with the current greedy scheduler and the existing makespan lower bound.
3. Negative control: the exact oracle rejects inputs above the nine-task bound.
4. The focused GitHub Actions workflow runs the existing locked tests and the exact-oracle demonstration without adding dependencies.

The exact reference must never return a makespan below the proven lower bound, and the feasible greedy schedule must never beat the exact optimum. These are software invariants, not hazard-model acceptance thresholds. Compare average and worst relative gaps only on matched instances:

\[
\text{relative gap} = \frac{T_{\mathrm{greedy}} - T_{\mathrm{exact}}}{T_{\mathrm{exact}}}
\]

The existing excess-over-lower-bound metric remains separately named and reported; it is not relabeled as an exact gap.

## Paradigm shift and comparison plan

The key change is from “the scheduler produced a plausible time” to “the same instance has a verified reference optimum and a quantified heuristic gap.” That enables defensible comparisons among ID-order greedy, alternative priority rules, and exact methods on small instances before evaluating scalable methods on harder cases.

NIST’s functional-recovery work emphasizes time-dependent restoration of lifeline services rather than component-only recovery. The 2025 NIST workshop also highlights differing community needs and interdependencies; NIST’s September 2026 paper calls out the need for consistent system-level resilience guidance across interdependent lifelines:

- NIST SP 1341, *Functional Recovery Performance Targets Workshop Report* (2025): https://doi.org/10.6028/NIST.SP.1341
- NIST, *Advancing Resilience Across Lifeline Infrastructure Systems: The Need for Consistent Guidance* (2026): https://www.nist.gov/publications/advancing-resilience-across-lifeline-infrastructure-systems-need-consistent-guidance

Those are recovery-objective comparators, not numerical scheduling baselines. This increment does **not** claim to beat RCPSP state of the art. Fair SOTA comparison requires a standard public instance suite, matched objectives and resource semantics, transparent runtime/hardware limits, known-optimum or best-known values, independent replication, and comparisons beyond the tiny exact-search ceiling.

## Next benchmark gates

- Reproduce a standard, license-compatible RCPSP instance subset and preserve the original instance digests.
- Compare the current greedy policy, at least two documented priority rules, and the exact oracle on every small instance.
- Report per-instance makespan, proven optimum status, relative gap, lower bound, explored order count, and runtime; do not aggregate away failed or censored cases.
- Add duration uncertainty only after its representation and provenance are declared. Report quantiles and recovery-goal probabilities only for explicit scenario weights or defensible distributions; use CVaR only when scenario probabilities are supportable.
- Keep synthetic, sourced, measured, and independently reviewed evidence separate. Hold out any calibration records used to fit task-duration or hazard models.

A meaningful near-term target is: zero invariant violations in the supplied test suite, exact agreement with the exhaustive oracle on all bounded test instances, deterministic outputs for fixed inputs, and a published per-case comparison against public scheduling baselines. None of these alone establishes physical-model accuracy or real-world safety.
