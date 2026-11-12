# Textile Scheduler Benchmark v0.1

Status: proposed benchmark contract; targets are not measured results.

## Purpose and scope

This benchmark defines the first numerical scheduling slice for the
symtropy-textile-process crate. It targets one-stage parallel-machine scheduling
of garment lots or specialist garment-service jobs. It is designed to create a
reproducible bridge from evidence-aware garment profiles to production planning,
before introducing multi-stage factory simulation.

The scheduler includes:
- machine capability eligibility and capability evidence;
- release times, soft due times, and optional hard latest-finish windows;
- machine-specific sequence-dependent setup time, energy, and water;
- declared process energy/water units and hard planning-horizon budgets;
- mandatory evidence gates with Pass, Fail, or Unknown states;
- deterministic Pareto comparison across lateness, tardiness, makespan, energy,
  water, and setup duration;
- four deterministic dispatch rules and a bounded exhaustive solver.

It is not a factory digital twin, lifecycle assessment, textile chemistry model,
staffing optimizer, conservation diagnosis, or carbon calculator. Numbers and
evidence references are caller-supplied assertions. The code does not
authenticate the underlying measurements.

## Semantic safeguards

1. **Hard gates are non-compensatory.** A failed job gate is blocked; an unknown
   gate is unverified. Neither job can be scheduled. More throughput or lower
   resource use cannot compensate.
2. **Missing setup evidence is not free setup.** For every eligible machine,
   transitions among setup families used by that machine must be declared.
   Same-family changeovers take zero ticks by definition; all other missing
   edges fail closed.
3. **Machine capability is explicit.** A machine is eligible only if its
   declared capabilities cover all capabilities required by the job.
4. **Delivery is represented twice on purpose.** Due ticks contribute to soft
   tardiness objectives. An optional hard finish tick is an absolute feasibility
   gate; no objective trade-off can override it.
5. **Resource budgets are hard.** The total process-plus-setup energy and water
   for one planning horizon must not exceed the caller-declared budgets.
6. **Units do not silently convert.** Every problem declares time, energy, and water
   unit IDs. Profile and functional-unit IDs stay attached to each job. The
   scheduler does not compare cultural, aesthetic, ceremonial, conservation,
   or service value by an invented common scalar.
7. **Readiness is not the same as output acceptance.** Job gates are pre-dispatch
   preconditions (for example material compatibility, process qualification,
   safety authorization, or conservation approval). Finished-product condition,
   appearance, fit, functionality, and repair success are post-run observations.
   A scheduled finish does not prove acceptance or a successful second use.
8. **Low-frequency use is not a production penalty.** A couture, beaded,
   ceremonial, performance, rental, archive, or rare-use garment can be
   represented by its own profile, capabilities, hard gates, care process and
   declared functional unit. A garment's event count is not used as a proxy
   for its worth.

A companion robustness contract is now defined in TEXTILE_SCHEDULE_ROBUSTNESS_BENCHMARK_V0_1.md; it compares duration, job-release and machine-availability shocks with the exact reference and dispatch baselines. Synthetic scenario summaries are not real-world probabilities. The current code models aggregate utility budgets across a planning horizon;
it does not yet enforce time-indexed utility peaks. The time unit is explicit,
and all time fields in one problem use that shared unit. Setups are
non-anticipatory in v0.1: setup starts no earlier than both machine availability
and job release. The model represents machine capabilities but not individual
operator calendars or skill-level constraints. Those are future extensions,
not implied capabilities.

## v0.1 algorithms

### Exact reference

The exhaustive solver enumerates per-machine job sequences for tiny instances:
at most 7 jobs, 4 machines, and 2,000,000 visited search nodes. It returns:

- every nondominated objective vector (the Pareto frontier);
- an actual schedule representative for each unique frontier vector;
- the independently best observed value for each objective over all feasible
  schedules;
- feasible complete-schedule and search-node counts.

Independent single-objective optima are lower bounds, not a claim that one
schedule achieves every best value at once. A size or node cap returns an
explicit error; it never returns a partial search as exact.

### Dispatch baselines

- **EarliestDue:** prioritizes earliest due tick, with deterministic tie-breaks.
- **LowestSetup:** prioritizes shorter next setup.
- **LowestEnergy:** prioritizes job-plus-next-setup energy, then water.
- **LeastSlack:** prioritizes smallest remaining hard-finish slack; where a job
  has no hard finish tick, it uses the soft due tick as a fallback.

These rules are deliberately simple. They are comparison points, not claims
that any rule is universally superior. Input vectors are canonicalized by
stable IDs to make outputs independent of caller ordering.

### Objective vector

All dimensions are minimized without a weighted sum:

1. number of late jobs;
2. total tardiness ticks;
3. makespan tick;
4. total energy units, including setup;
5. total water units, including setup;
6. total setup-duration ticks.

The Pareto frontier keeps legitimate trade-offs visible. A separate decision
policy may later select among feasible candidates, but it must explain its
priority order and retain the original vector.

## Corpus design

The locked scenario corpus should stratify by garment profile and process
complexity, not only by job count. Minimum categories:

- everyday basics and workwear;
- rare-use formal, occasion and ceremonial pieces;
- couture with dense embroidery, beads, sequins or metallic thread;
- detachable or mixed-material decoration requiring inspection or specialist
  finishing;
- performance costumes with event deadlines and repair/reuse options;
- archive/display textiles with conservation or condition-sensitive handling;
- deadstock, runway samples and small batches where preventing surplus matters.

A job record should reference a profile and functional-unit contract. Do not
rank two garments as better or worse by dividing by raw wear counts when one
serves as protective workwear and another as a conserved ceremonial object.
The scheduler itself does not measure appearance retention, cultural value,
repair success, or conservation condition: these must be supplied as required
acceptance gates with provenance and evaluated in domain-specific checks.

Demand strategy comparisons (made-to-order, preorder, short batch, forecast
batch, shared/rental stock, reuse of an existing sample, commission, repair
first) should use the companion textile-process benchmark. A schedule alone
must not be credited with reduced overproduction unless accepted output and
unsold inventory are measured at the declared boundary.

## Benchmark protocol

For every fixture, lock and report:
- benchmark and scenario IDs, revision, input SHA-256, and scheduler version;
- job and machine counts, eligible machine edges, setup matrix coverage;
- all time/resource unit IDs and budget values;
- the number of blocked or unknown gates (expected to prevent the run);
- hard-deadline and resource-budget compliance;
- every objective value, exact frontier or heuristic rule, node count, runtime,
  and exit status;
- software/toolchain versions and the exact Git commit tested.

Use paired instances for comparisons. Do not compare runs with different
demand, machine capability, accepted-quality definitions, utility units, or
hard gates. Generated scenarios are synthetic evidence for algorithm behavior,
never factory-savings evidence. Preserve infeasible cases as negative controls.

## Proposed target ladder

These are engineering goals, not results already attained.

| Tier | Measure | Initial acceptance target |
|---|---|---|
| A — safety | Failed or unknown mandatory job gates scheduled | **0** |
| A — feasibility | Duplicate/missing jobs, incompatible machine assignment, broken release/hard-finish windows, setup overlap, or resource-budget violation | **0** |
| A — units | Silent unit conversion or missing setup edge interpreted as free | **0** |
| A — determinism | Same input manifest and version yields a different normalized schedule/report | **0** |
| B — exact tiny oracle | Agreement with a separately implemented brute-force checker on the locked tiny corpus | **100%** |
| B — bounded search | Exact solve completes within declared node cap for every in-bound fixture | **100%**, otherwise surface explicit cap error |
| C — heuristic quality | Median normalized gap to each independently best objective on feasible tiny cases | **<= 15%**; report each objective separately |
| C — tail quality | 95th-percentile normalized gap, split by profile and problem family | **<= 30%**; zero-optimum cases reported as absolute deltas |
| D — pilot operations | Metered energy per accepted good unit versus a paired incumbent schedule | Target **>= 5% relative reduction** with no hard-gate regression |
| D — service | On-time rate and total tardiness versus incumbent | No more than **1 percentage-point** on-time-rate regression; target lower tardiness |
| D — surplus | Unsold garments/samples per accepted unit under a declared demand strategy | Target **>= 10% relative reduction**, without delivery or acceptance regression |

For a positive exact optimum, normalized gap is
(heuristic value minus exact optimum) divided by exact optimum. If the optimum
is zero, report the absolute delta rather than divide by zero. Because optima
are calculated independently, a single multi-objective schedule is not
required to achieve every objective's gap target simultaneously; disclose its
full vector and which frontier trade-off it represents.

For real-factory trials, energy and water must be metered at a declared boundary.
Carbon requires a disclosed, time- and location-appropriate emissions factor;
this v0.1 scheduler does not turn energy units into emissions. Quality acceptance
must be counted at the receiving boundary, not inferred from a scheduled finish.
The 5% and 10% values are proposed starting targets and should be recalibrated
after baseline variance and safety margins are measured.

## Next extensions, gated by evidence

1. Add a separately implemented exhaustive checker and locked scenario manifest.
2. Add operator/skill calendars and setup-resource constraints; compare against
   published flexible job-shop instances before claiming parity.
3. Add time-indexed energy/water capacity windows and measured carbon factors.
4. Add multi-stage flow, batching and lot splitting only with explicit
   conservation-of-flow and no-double-counting invariants.
5. Couple schedule outputs to the unsold-stock/demand benchmark; keep prediction,
   accepted output, repairs, resale, donations, and actual second use distinct.

## Research references

- European Commission, [Sustainable and Circular Textiles Strategy](https://environment.ec.europa.eu/strategy/textiles-strategy_en).
- European Commission, [Ban on destruction of unsold clothes and shoes enters into application](https://environment.ec.europa.eu/news/ban-destruction-unsold-clothes-and-shoes-enters-application-2026-07-17_en), 17 July 2026.
- Eroglu, D. Y., [Green and on-time scheduling in real-world textile production: a genetic algorithm approach validated by MIP and benchmarked with ACO](https://doi.org/10.1080/00207543.2025.2549138), *International Journal of Production Research*, 2025. Published results are specific to that study's datasets and are not assumed as expected gains here.
- [The Flexible Job Shop Scheduling Problem with Setups and Operator Skills: an application in the textile industry](https://doi.org/10.1016/j.ifacol.2024.09.125), IFAC-PapersOnLine, 2024.
- [Scheduling a dual-resource flexible job-shop with sequence-dependent setup times: an industrial case study in garment manufacturing](https://doi.org/10.1080/00207543.2026.2711058), *International Journal of Production Research*, 2026.
