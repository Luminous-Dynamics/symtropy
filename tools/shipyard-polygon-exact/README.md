# Shipyard exact tiny polygon packing oracle (experimental V0)

This crate adds a bounded exact packing solver on top of the isolated polygon geometry and deterministic first-fit baseline crates.

## Why this is a paradigm shift

A failed heuristic should be reported as “this heuristic did not find a layout,” not “the yard is infeasible.” For the tiny declared model, this solver separates three outcomes:

- **Feasible:** an explicit layout was found and each footprint passes the exact fixed-point contact/overlap predicate.
- **Proven infeasible:** every relevant placement branch was exhausted in the finite model.
- **Budget exceeded:** the search stopped before a layout or complete proof. This outcome remains unknown.

It uses dynamic minimum-remaining-values (MRV) branching: at every search depth it chooses the not-yet-placed block with the fewest currently compatible positions, with stable input-order tie breaks. Positions are scanned y-major then x-major. This is a reproducible exact reference and small-instance fallback, not a production-scale global optimizer.

## Frozen greedy-failure benchmark

A second known-answer scenario uses a 4×4 yard and three fixed-orientation blocks in the same request order: two 1×1 squares followed by a 1×3 vertical bar. The deterministic exact-polygon first-fit baseline places the squares at (0,0) and (2,0), then reports the bar as unplaced. The exact MRV solver can change the placement order internally and recover a complete layout, for example the bar at (0,0) and the squares at (2,0) and (2,2). This is a constructed counterexample to greedy request-order completeness, not an empirical speed or industrial-quality result.

The test oracle checks every pair in the exact layout using integer-cell occupancy and independent axis-aligned boundary-contact tests. Target: greedy covers 2/3 requests; exact search covers 3/3 and produces zero independent conflicts. The exact head must confirm the scenario and the test fixture before treating the target as verified.

## Explicit limits

- At most four requests.
- Yard dimensions no larger than 5 by 5 integer placement units.
- Fixed polygon orientation; integer translations only.
- Boundary contact is considered a conflict.
- No dynamic time reuse, rotation, aisles, resource constraints, transfer paths, cranes, stacking, or swept-volume modelling.
- Every candidate-placement state evaluation counts against the declared budget.
- Arithmetic or geometry errors fail closed and are not converted into empty space.

## Required validation

- The two-L-shape known-feasible case must yield a layout accepted by a separate unit-cell / axis-aligned-boundary test oracle.
- Two 2x2 squares in a 2x2 yard must return a true infeasibility proof for this exact model.
- A one-check budget must return BudgetExceeded, never ProvenInfeasible.
- Out-of-scope yard size, more than four requests, duplicate IDs and a zero budget must be rejected explicitly.
- Rust 1.96 format, offline tests and Clippy with warnings denied must pass on the exact head before the result is treated as verified.

This solver does not make a general real-yard infeasibility claim. A proof applies only to the declared polygon geometry, yard bounds, fixed-orientation integer translation domain, boundary policy and input ordering-independent finite model.
