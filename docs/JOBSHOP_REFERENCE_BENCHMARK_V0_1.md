# JOBSHOP-REF-001 — pinned classic job-shop reference

## Research motivation

The 2026 open benchmark platform by Reijnen et al. covers classic job shop,
flow shop, flexible job shop, assembly job shop, sequence-dependent setup
times, and online arrivals. This small first step intentionally qualifies only
classic fixed-route JSP semantics before any solver is compared. ISO 23247-6:2026
also formalizes integrated, unified, and federated manufacturing digital-twin
composition; a composed system needs explicit interfaces and evidence semantics,
not an unsupported claim that one local optimizer solves the whole factory.

External references:

- Reijnen et al., “Job shop scheduling benchmark: environments and instances
  for learning and non-learning methods,” Annals of Mathematics and Artificial
  Intelligence (published 30 May 2026), https://doi.org/10.1007/s10472-026-10007-3
- ISO 23247-6:2026, Digital twin composition,
  https://committee.iso.org/standard/87426.html
- NIST Smart Manufacturing Systems Test Bed,
  https://www.nist.gov/laboratories/tools-instruments/smart-manufacturing-systems-sms-test-bed

## Included artifacts

- A strict parser for the pinned upstream machine/duration-pair format.
- Two immutable, provenance-pinned benchmark files: Taillard ta01 (15×15)
  and Adams abz5 (10×10).
- Workload lower bounds: maximum total processing time of any job or any
  machine. These are lower bounds only—not feasible schedules.
- A bounded exhaustive serial-schedule search for at most 12 operations and
  two million expanded nodes. It emits operation-level timing records.
- An independent schedule verifier checks completeness, identity, assignment,
  processing duration, job precedence, machine exclusivity, and reported makespan.
- Twenty deterministic tiny instances cross-checked against a separate machine-order
  permutation oracle that rejects precedence cycles and evaluates longest paths.
- Three deterministic dispatch rules (shortest processing time, most work remaining,
  most operations remaining) produce independently audited feasible schedules on both
  pinned larger instances without claiming optimality.
- Negative controls for malformed token counts, non-integer/negative values,
  zero durations, invalid machine IDs, overflow, and attempts to run the exact
  oracle above its limit.

## Benchmark contract

For every admitted instance:

1. Every operation has one zero-based fixed machine and a positive integer duration.
2. Operations within one job obey their declared order.
3. A machine processes at most one operation at a time.
4. Durations are non-preemptive and all jobs are available at time zero.
5. The only objective in v0.1 is makespan; no setup, transport, energy, due-date,
   maintenance, or staffing effects are modeled.
6. “Optimality proved” is emitted only when the full bounded search finishes.
   Search-budget exhaustion is an explicit error and must never be scored as a
   solution.

## Paradigm shifts and staged benchmarks

| Stage | Evidence to earn before making the next claim | Candidate target |
|---|---|---|
| Parser integrity | pinned fixture dimensions, exact source blob IDs, malformed input rejection | 100% fixture parse agreement; zero silent truncation |
| Tiny-instance oracle | independent exhaustive reference and hand-checkable cases | 20 tiny cases; 100% exact objective agreement |
| Algorithm comparison | pinned instances, identical time/node budgets, published solver settings | at least 50 instances across multiple families before broad claims |
| Robust scheduling | held-out machine outages, durations, job releases, and forecast error | zero hard-constraint violations; report median and worst-decile regret |
| Cross-system optimization | connect schedule decisions to validated machine capability, material, energy, transport, and demand contracts | every capacity claim has source, unit, timestamp, uncertainty and validity window |
| Field outcome | instrumented pilot with a preregistered baseline | report throughput, lead time, energy per good unit, on-time completion, scrap, and worker safety separately |

A future aspiration such as ≥5% median objective improvement may be worth testing,
but it is not an acceptance result until a fair matched-baseline experiment shows
it. Report median, p90/p95, worst-decile, runtime, memory, seed, instance digest,
and all constraint violations; never use only the best case.

## Next extensions (not in this PR)

- Add a separate FJSP parser only after machine-alternative semantics are tested.
- Add a canonical result receipt binding source commit/path and input digest to
  parser/model version, method/config, resource budget, schedule, verifier result,
  and the distinction between proved-optimal and heuristic outcomes.
- Introduce setup times, releases, deadlines, disruptions, and rolling-horizon
  replanning as distinct benchmark tracks.
- Compose schedulers with industrial-coupling and telemetry evidence through
  typed interfaces; observed events, inferred capability, available capacity,
  and execution authorization must remain different claims.
- Keep public input data and code provenance, result digests, exact solver
  limits, and negative controls in the evidence bundle.

## Boundaries

The included instances are synthetic benchmark artifacts, not current plant
data. Passing a parser test is not evidence of improved scheduling. Passing the
tiny exact oracle establishes only the admitted instance/model result—not
production readiness, factory availability, an energy saving, or a real-world
economic benefit.
