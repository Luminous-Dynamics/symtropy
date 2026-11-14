# Shared-Heat Kiln Scheduling and Conforming Dry-Lumber Flow Benchmark v0.1

**Owner:** Symtropy  
**Consumers:** Symthaea, Mycelix, Sol-Atlas  
**Evidence class:** synthetic fixtures only

## Research grounding

- Vanzetti, Corsano and Montagna, “Integrated scheduling of the drying process in a sawmill,” *Computers & Chemical Engineering* (2021): https://doi.org/10.1016/j.compchemeng.2021.107407. This work couples kiln filling, drying program choice and boiler/by-product operation through a discrete-time MILP. It motivates joint resource constraints; it does not establish performance for this benchmark.
- Berardinucci et al., physics-based digital-twin framework for lumber handling in the wood industry (2026): https://doi.org/10.1016/j.rcim.2025.103144. This is a later research direction for machine handling; v0.1 does not model its physics or claim replication.
- NIST digital-twin workshop summary: https://www.nist.gov/publications/digital-twins-workshops-summary-report. Interoperability and VVUQ remain relevant; a synthetic schedule is not model validation.
- Reuse the existing WOOD-LIFE-001 #1741, sawlog recovery contract #1745, lifecycle benchmark PR #1742, and MFG-VALUE #7608. Do not duplicate their lifecycle, mass-flow, provenance or production-value ownership.

## Paradigm shift

Optimize **conforming dry volume delivered**, not green-board throughput in isolation. A local sawline policy may create a green inventory peak that the kiln/boiler envelope cannot process. The drying profile, release time, batch duration, target/achieved moisture, grade evidence and inspection state all constrain what counts as useful product.

The benchmark has two planners over identical frozen input:
1. deterministic earliest-start / first-fit baseline;
2. exhaustive small-instance oracle that maximizes conforming dry volume, then minimizes makespan, then modeled energy, then wait, then uses a stable lexical tie-break.

The exhaustive solver is a reference oracle for tiny fixtures only. It is not a scalable production optimizer.

## v0.1 model

- Time is discrete in integer hours; batches are non-preemptive.
- A kiln has stable ID, finite volume capacity, supported profile IDs and maximum power.
- A batch has release hour, duration, profile ID, dry-volume units and heat/power demand.
- Concurrent active batches across all kilns may not exceed the shared heat/power ceiling in any hour.
- A batch is blocked before scheduling if no kiln supports its profile, capacity/power is insufficient, duration exceeds horizon, achieved moisture misses target, required grade evidence is not qualified, or required inspection failed.
- Each schedule is independently replayed to reject duplicate batch IDs, invalid kiln assignments, overlap, incompatible profile/capacity/power, release-time violations, horizon overrun and shared-envelope breach.
- The receipt binds exact corpus bytes, validator version and 40-hex source commit. Unknown JSON fields and duplicate identifiers fail closed.

Moisture, duration, power draw and grade/inspection states are declared synthetic inputs. This tool does not predict moisture, dry-kiln physics, board defects or real energy use.

## Frozen known-answer cases

1. Two conforming batches run sequentially through one kiln.
2. An unsupported drying profile is blocked while a valid batch proceeds.
3. Two individually acceptable kilns cannot run two high-power jobs concurrently under the shared power limit; they must be serialized.
4. A moisture miss, missing structural-grade evidence and failed inspection are excluded from conforming dry output.
5. The greedy baseline commits to a small early batch and delivers 10 volume units, while the exhaustive oracle chooses a longer batch and delivers 100. The 90-unit gap is a constructed counterexample, not an estimate of real mill savings.

## Benchmark scorecard

### Contract / feasibility gate
- 100% known-answer outcomes and full gate-reason sets match.
- Zero accepted overlap, capacity, profile, readiness, horizon or shared-power violations.
- 100% deterministic replay for identical corpus bytes and source commit.
- The independent schedule validator reconstructs all reservations from the frozen inputs and returned placement receipt.

### Optimization gate
- For tiny instances within the declared search-node bound: exhaustively enumerated optimal schedule, explicit node count and baseline-to-oracle conforming-volume gap.
- On larger or bounded-out instances: report indeterminate search-limit disposition; never say “optimal” when the search was cut off.
- Compare conforming dry-volume delivery, makespan, wait hours, modeled energy-unit-hours, kiln busy hours and available kiln hours independently; no opaque combined score.
- Later benchmark expansions should measure rolling green-buffer peaks/overflow and compare baseline/candidate on identical held-out instances before making a planner-quality claim.

### Physical-science gate (future only)
A physical pilot requires lawful, documented mill data, calibrated moisture/energy measurements, declared kiln profile and equipment configuration, independent quality/grade evidence, and uncertainty-aware model validation. Synthetic receipts are not physical measurement or evidence of savings.

## Claim ceiling

A passing v0.1 establishes only that the small scheduling model, gate logic and independent checker reproduce the frozen synthetic cases. It does not qualify real wood, predict drying schedules, demonstrate kiln throughput/energy savings, grade lumber, validate kiln physics or authorize production operation.
