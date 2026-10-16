# Textile Schedule Robustness Benchmark v0.1

Status: proposed benchmark contract and initial synthetic fixture plan. Results are
not yet measured; stress-case frequencies are not estimates of real-world risk.

## Why nominal optimality is insufficient

A schedule that is optimal for one set of exact durations can still be fragile
when specialist work takes longer than estimated, a required input is delayed,
or a machine is unavailable. Research on distributionally robust parallel-machine
scheduling explicitly considers uncertain processing time and tardiness. Work
on flexible job-shop scheduling also models stochastic breakdowns and the
stability of rescheduling after a disruption. The textile implementation should
use those as benchmark dimensions, not claim a novel optimizer before it has
earned comparisons.

Circularity introduces a second uncertainty boundary: estimating demand or
recommending reuse is not the same as an accepted garment, an actual second use,
or verified material recovery. A review of 45 textile LCA studies found that
reuse outcomes depend materially on service-life extension, substitution
assumptions, recycling yield, care/use patterns and transport. These quantities
need evidence and sensitivity analysis rather than one default circularity
multiplier.

## v0.1 stress harness

The public Rust module symtropy-textile-process::stress evaluates a locked
baseline problem against deterministic perturbation cases. Supported shocks:

- additional processing time for a named job;
- delayed release of a named job, representing late material arrival, specialist
  preparation or another readiness precondition;
- machine unavailable until an explicit tick in the shared time unit;
- a hard utility-budget curtailment that may preserve or tighten but never loosen
  the baseline energy/water limits.

The harness runs the bounded exact solver and all four deterministic dispatch
rules for every case. This establishes four different outcomes:
1. the exact reference finds a feasible schedule;
2. the exact reference finds no feasible schedule under the hard constraints;
3. exact evaluation fails due to an invalid fixture or declared bound;
4. an exact-feasible case is missed by a heuristic.

Those outcomes must not be collapsed. In particular, a greedy dispatch rule
failing to find a plan is not evidence that the underlying factory problem is
infeasible. Each scenario retains a fixture reference, the exact result,
dispatch outcomes, full scheduled operations where a plan was found, original
profile and functional-unit identifiers, and gaps from the independently best
exact objective values.

The v0.1 harness is bounded to 32 scenarios and reuses the scheduler's tiny
exact caps (7 jobs, 4 machines, 2,000,000 search nodes per case). It is for
reproducible tiny-instance validation, not production-scale Monte Carlo or
probability estimation. The machine availability shock is a pre-horizon
availability shift; breakdowns occurring in the middle of active operations
and re-entrant rescheduling remain future work.

## Locked test design

The first fixture should include the following matched cases over the same
profile/capability/problem contract:
- nominal baseline;
- longer processing time for specialist repair/embellishment;
- delayed material/specialist-input readiness;
- utility budget reduced below the minimum process requirement;
- specialist machine unavailable until a time that makes a hard finish window
  impossible.

Keep non-feasible scenarios as explicit negative controls. Add future corpus
slices for utility budget reduction, rush-order insertion, supplier quantity
shortfall, rework/rejection, and multiple concurrent machine outages only after
each perturbation has deterministic semantics and unit checks.

All scenario input files must have stable IDs, versions, source/fixture refs and
a canonical input SHA-256. Record the exact commit, toolchain, node count,
runtime, each objective vector, the scenario's exact feasibility classification,
whether each heuristic found a plan, and any per-objective gaps. Never report
a synthetic fixture reference as if it authenticated an industrial measurement.

## Proposed acceptance targets

| Measure | Tiny locked corpus target | Notes |
|---|---:|---|
| Invalid/duplicate/unknown shock references accepted | **0** | Fail closed |
| Utility-budget shock raises a baseline limit or declares no affected resource | **0** | Curtailment cannot silently relax a constraint |
| Arithmetic overflow accepted | **0** | Checked addition |
| Dispatch claims a plan where exact oracle says no feasible schedule | **0** | Oracle contradiction is a harness failure |
| Oracle feasibility classification or heuristic-miss attribution is incorrect | **0** | Exact feasibility and planner quality are separate findings |
| Missed exact-feasible cases are hidden or counted as infeasible | **0** | Every miss must be reported per rule |
| Future improved selector misses a locked tiny feasible case | **0** target | The v0.1 dispatch rules are baselines; this corpus deliberately tests that they can miss feasible schedules |
| Repeat-run normalized reports differ for identical manifest/version | **0** | Deterministic ordering |
| Hidden infeasibility or partial exact search presented as exact | **0** | Bound failures must remain explicit |
| Factory risk / outage probabilities inferred from unweighted stress corpus | **0** | Requires calibrated external frequency evidence |

For each dispatch rule, report:
- exact-feasible scenarios and scenarios where the rule found a feasible plan;
- missed feasible cases;
- nearest-rank P50 and P95 of late-job count, total tardiness, makespan,
  energy, water, and setup duration across plans produced for exact-feasible
  stress cases;
- per-objective gaps to the independently best value over the exact feasible
  set for that scenario. This is a vector of separate lower bounds; one schedule
  need not attain every objective minimum simultaneously.

The P50/P95 values summarize the enumerated corpus only. Until representative
field distributions and failure rates are calibrated, they do not estimate the
probability of a real-world disruption or its tail risk.

## Circularity and demand outcomes: separate causal evidence

Add a downstream campaign that compares made-to-order, preorder, short batch,
forecast batch, shared/rental inventory, repair-first, reuse of existing samples,
and commission strategies under paired demand cases. For each strategy record:
- forecast demand, realized demand, accepted good units and unsold units;
- actual garment/service events, return and repair outcomes, condition at re-entry,
  verified second use, and material recovery at a declared boundary;
- energy/water/material input and output balances with an explicit functional unit;
- transportation and cleaning/maintenance for shared-use scenarios;
- unverified claims and missing data as unknown, not as zero impact or a successful
  circular outcome.

Do not credit the schedule alone for reduced overproduction. A forecast can be
wrong; a donation can remain unused; a repair offer can be rejected; a collected
garment may not be technically recyclable. Count only the observed outcomes
supported by the applicable evidence contract. Keep circular routes as separate
options when substitution, durability, transport, care and recycling yields are
uncertain; don't force them into a universal scalar.

## Paradigm-shifting research sequence

1. **Exactness before sophistication:** complete independent tiny-case oracles
   and negative controls before adding stochastic optimizers.
2. **Robustness before average throughput:** report missed-feasible cases and
   worst-case hard-constraint breaches before optimizing mean values.
3. **Demand before volume:** pair scheduling with observed surplus/acceptance
   ledgers so fewer produced items, not just faster schedules, become measurable.
4. **Functional units before rankings:** compare a garment by declared service
   and context; don't penalize rare-use ceremony, performance, archive or couture
   by ordinary wear-count assumptions.
5. **Evidence before circularity credit:** model repair/reuse/recycling as an
   observed lifecycle chain with custody and displacement assumptions, not as
   a prediction score.
6. **Machine-readable provenance:** preserve stable profile, material,
   functional-unit and lifecycle-event identifiers for downstream Digital
   Product Passport integration without asserting unannounced legal field
   requirements.

## Research references

- [A distributionally robust approach for the parallel machine scheduling
  problem with optional machines and job tardiness](https://doi.org/10.1016/j.cor.2024.106776),
  *Computers & Operations Research* 170, 106776 (2024).
- Fan, Wang & Tian (2024), [Flexible job shop scheduling with stochastic machine
  breakdowns by an improved tuna swarm optimization algorithm](https://doi.org/10.1016/j.jmsy.2024.03.002),
  *Journal of Manufacturing Systems* 74, 180–197.
- Abagnato, Rigamonti & Grosso (2024), [Life cycle assessment applications to
  reuse, recycling and circular practices for textiles: A review](https://doi.org/10.1016/j.wasman.2024.04.016),
  *Waste Management* 182, 74–90.
- European Commission, [Digital Product Passport](https://single-market-economy.ec.europa.eu/single-market/digital-product-passport_en).
- European Commission, [Sustainable and Circular Textiles Strategy](https://environment.ec.europa.eu/strategy/textiles-strategy_en).
