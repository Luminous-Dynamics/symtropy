# Shipyard exact tiny polygon packing oracle (experimental V0)

This crate adds a bounded exact packing solver on top of the isolated polygon geometry and deterministic first-fit baseline crates.

## Why this is a paradigm shift

A failed heuristic should be reported as “this heuristic did not find a layout,” not “the yard is infeasible.” For the tiny declared model, this solver separates three outcomes:

- **Feasible:** an explicit layout was found and each footprint passes the exact fixed-point contact/overlap predicate.
- **Proven infeasible:** every relevant placement branch was exhausted in the finite model.
- **Budget exceeded:** the search stopped before a layout or complete proof. This outcome remains unknown.

It uses dynamic minimum-remaining-values (MRV) branching: at every search depth it chooses the not-yet-placed block with the fewest currently compatible positions, with stable input-order tie breaks. Positions are scanned y-major then x-major. This is a reproducible exact reference and small-instance fallback, not a production-scale global optimizer.

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
