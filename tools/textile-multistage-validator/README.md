# Textile Multi-Stage Schedule Validator v0.1

A dependency-free Rust reference validator for proposed multi-stage garment
production schedules. It is a bounded first executable step for Symtropy issue
[#4875](https://github.com/Luminous-Dynamics/symtropy/issues/4875).

## Why a separate verifier comes first

Schedulers must not be trusted to certify their own output. Before comparing
MILP, constraint-programming, dispatch, genetic, evolutionary or learned
policies, the benchmark needs one independent feasibility judge with frozen
semantics. This crate is intentionally separate from any optimizer.

## v0.1 scope

The validator checks:

- exact one-to-one coverage of required operations;
- positive, exact operation durations and half-open time intervals;
- release and hard due-time bounds;
- declared machine capabilities;
- operator skill and certification requirements;
- machine and operator availability windows;
- machine and operator non-overlap;
- explicit sequence-dependent machine setup transitions, including explicit
  zero-duration rules;
- separate setup reservations that bind a positive-duration machine transition
  to a qualified operator, exact time interval and machine availability;
- setup-operator skill/certification, availability and non-overlap with both
  production operations and other setup reservations;
- precedence and minimum-lag constraints;
- deterministic, sorted findings for reproducible comparison.

The independent `utilities` module checks time-indexed electricity,
steam, water and wastewater envelopes. It binds complete/partial demand
profiles to production operations and setup reservations, segments the
schedule at every operation/envelope boundary, aggregates concurrent demand,
and rejects missing profiles, unknown capacity, uncovered positive demand,
overlapping envelopes and capacity exceedance.

Every demand profile and known capacity envelope now carries a provenance
basis: source/evidence ID, exact SHA-256 digest, method/version, and a declared
basis class (calibrated meter observation, calibrated process-trial
measurement, machine nameplate, or engineering estimate). Measured meter/trial
claims require a calibration ID. The validation report counts each basis
class separately so an estimate cannot be mistaken for a meter observation.
This is structural binding only: source bytes and calibration authenticity
must be checked by an upstream evidence verifier. An engineering estimate is
still usable for synthetic feasibility exploration but must not be promoted to
a measured production feasibility claim.

A complete profile explicitly declares omitted utility metrics as zero; an
incomplete or missing profile is not treated as zero. All time values are
integer ticks in one caller-declared common unit. The reference does not infer
minutes, shifts, machine cycle times or setup rules.

## Run locally

~~~sh
cargo test --manifest-path tools/textile-multistage-validator/Cargo.toml
cargo fmt --manifest-path tools/textile-multistage-validator/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/textile-multistage-validator/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/textile-multistage-validator/Cargo.toml
~~~

The known-answer case contains two jobs routed through cutting, sewing,
finishing and inspection. Unit tests mutate the valid case to verify that
malformed or infeasible plans are rejected.

## Deliberate non-goals

This crate does not:

- find or optimize a schedule;
- implement MILP, CP-SAT, genetic algorithms, reinforcement learning or
  dispatch policies;
- model lot streaming, batch splitting, rework or parallel subassemblies;
- model process emissions or a whole-factory environmental inventory;
- establish ergonomic exposure or worker wellbeing;
- command machines or act as an ERP/MES/plant controller;
- prove that a schedule was executed or that output passed inspection.

The bounded exhaustive oracle in `src/oracle.rs` independently searches tiny
integer-time instances for the minimum makespan. It does not call the validator
while searching and returns no result when its node budget is exhausted. Its
v0.1 scope is deliberately limited to explicit zero-duration setup transitions;
nonzero setup models are rejected as unsupported. It is an oracle for small
instances, not a production scheduler.

## Disruption replay and replan qualification v0.1

The `resilience` module evaluates a caller-supplied replan against a frozen,
independently feasible baseline. It does not construct or optimize the replan.
Supported deterministic event types are:

- machine outage and operator absence over half-open availability intervals;
- delayed material availability for an exact operation;
- longer operation duration and tightened hard due dates;
- utility capacity derating or capacity becoming explicitly unknown.

The evaluator first verifies baseline schedule feasibility and, when a utility
model is supplied, baseline utility feasibility. It then applies the future
portion of the event to a cloned problem/model, preserving the historical
portion before the observation tick. A proposed replan is checked with the
same independent machine/operator/precedence and utility validators.

Work already started before the observation tick is immutable. An event that
interrupts an in-progress operation or setup is rejected into a separate
incident/abort-restart path; v0.1 has no silent abort/restart semantics.
Unknown capacity remains unknown and cannot be replaced by a guessed value.
A numeric utility derate needs an evidence/method basis. The evaluator reports
feasibility, makespan, post-event completion horizon, tardiness and plan churn
(changed/added/dropped operations, machine/operator reassignment, start/finish
movement, total absolute shift and setup-reservation changes) as separate
metrics.

This is a deterministic replay evaluator, not a stochastic failure model or
replanner. It does not compare policy quality until matched replan candidates
are supplied. The exact tiny-instance oracle still rejects nonzero setup
durations and does not search setup operators, route receipts or utility
profiles. The next benchmark tranche is a frozen disruption corpus and a
compute-matched tournament of independent replan policies. Observed production
receipts, calibrated utility sources and worker-approved constraints remain
separate evidence gates.


## Claim ceiling

A passing report means only that the supplied schedule satisfies this
version's declared synthetic constraint model. It is not evidence of factory
productivity, energy/water savings, safe working conditions, product quality,
industrial feasibility or physical execution.


## Route eligibility gate v0.2

The `route_gate` module binds every production job to exactly one upstream
`RouteRequirement` and a matching `RouteReceipt`. A scheduled textile job is
not eligible merely because a schedule can be made feasible.

The gate requires exact matching of:
- physical lot ID and incoming mass;
- route ID and versioned receiving specification;
- receipt ID and exact receipt SHA-256;
- evidence-manifest SHA-256;
- upstream contract version and distinct issuer/verifier IDs.

Only `Eligible` route receipts pass. `Conditional`, `InsufficientEvidence`
and `Ineligible` all block scheduling. `validate_with_route_gates` combines
the ordinary machine/operator/precedence report with route precondition findings.

This layer checks structural identity and digest-string equality. It does not
recompute receipt SHA-256 from artifact bytes, authenticate the issuer/verifier,
or verify physical evidence. The upstream verifier must verify the exact receipt
bytes before a route is supplied here. A mismatch against the pinned digest
makes a changed/stale receipt visible to the schedule gate.

The 17 route-gate unit tests cover eligible multi-lot binding, missing and
non-eligible receipts, digest/lot/route/spec/mass mismatch, duplicate
requirements/receipts, unknown jobs, malformed digests, independent verifier
identity and combined schedule validation.


## Multiobjective schedule tournament and Pareto qualification v0.1

The `pareto` module evaluates a tournament of caller-supplied candidate schedules. It is deliberately **not an optimizer**. It consumes:
- the independent schedule `ValidationReport`;
- the upstream `validate_route_gates` findings plus exact route-receipt IDs, receipt digests and evidence-manifest digests;
- a valid time-indexed `UtilityReport` with the same problem/tick unit;
- an exact disruption `ResilienceReport` when the tournament declares a disruption scenario;
- independent receiver acceptance for the exact output lot, process trial and receiving specification;
- source-bound objective observations with fixed units, provenance class and evidence digests.

A hard gate is never an objective. An infeasible schedule, failed route/utility check or rejected receiver is excluded before Pareto sorting. Missing utility or conditional/unknown receiver acceptance is insufficient evidence and is not treated as a zero-cost or accepted result.

The objective schema preserves 13 separate directions:

| Direction | Objective | Unit |
|---|---|---|
| Maximize | Receiver-accepted output mass | mg |
| Maximize | Conservative fibre-length-qualified output per input mass | ppm |
| Minimize | Residual trim mass | mg |
| Minimize | Independently inspected damage area | mm² |
| Minimize | Electricity per accepted kg | Wh/kg accepted |
| Minimize | Steam per accepted kg | g/kg accepted |
| Minimize | Water per accepted kg | mL/kg accepted |
| Minimize | Wastewater per accepted kg | mL/kg accepted |
| Minimize | Operator/setup labour per accepted kg | min/kg accepted |
| Minimize | Makespan | tick |
| Minimize | Total tardiness | tick |
| Minimize | Plan churn | operations |
| Minimize | Post-disruption recovery horizon | tick |

The evaluator requires every metric. It checks unit and provenance class; binds accepted mass to the independent receiver evidence; binds fibre length, trim and output metrics to the exact output lot/trial; binds damage to the incoming lot/trial; checks schedule makespan against the independent report; and checks disruption metrics against the exact replay. Engineering estimates cannot stand in for calibrated meter observations or worker time records.

Candidates in one tournament must share exact benchmark suite/version, objective schema, hard-gate policy, problem, scenario, incoming lot/mass, receiving specification and utility tick unit. A mismatch invalidates the comparator; it is not repaired by rescaling or filtering.

Pareto dominance is computed on the full objective vector. Incomparable trade-offs remain on the same front. Equal vectors tie; candidate IDs provide deterministic display order only. No weighted scalar, hypervolume or single “winner” is invented. A later hypervolume report would require a preregistered normalization and reference point and would remain supplementary.

Run the focused tests with:

~~~sh
cargo test --manifest-path tools/textile-multistage-validator/Cargo.toml pareto
~~~

The comparator checks structural evidence references and report identities, not source-byte authenticity, worker consent or the physical truth of measurements. Upstream source/receipt verification remains mandatory.
