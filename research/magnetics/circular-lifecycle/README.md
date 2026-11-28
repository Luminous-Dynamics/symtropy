# Circular and Second-Life Rotor Benchmark

Program: MAG-AM-CIRCULAR-MOTOR-LIFECYCLE-001
Issue: #7870
Version: 0.1
Status: protocol and evaluator prototype; no physical superiority claim

## Thesis

Co-design the rotor, magnet carrier, joining system, material passport and service tooling for a second life. Evaluate lifetime serviceability and recovery against optimized conventional PM rotors while preserving first-life motor function.

## What counts

- A magnet is reuse-grade only after post-recovery functional acceptance.
- Material sent to a recycler is recycling feedstock, not recycled material or direct reuse.
- Remanufactured output counts only after the remanufactured magnet passes acceptance.
- Every mass stream is joined to one teardown, component, specimen, build and material lot.
- Missing mass, unbounded uncertainty, unlinked streams or mismatched genealogy produce Indeterminate.

## Comparators

At least one optimized conventional PM rotor family is mandatory. Bonded/adhesive retention and mechanically retained segmented PM designs are application-dependent comparators. Magnet-free EESM, induction and reluctance architectures belong in a separate function-matched architecture tournament; they do not belong in the PM recovery denominator.

## Decision rule

The candidate must pass absolute post-service function and safe-disassembly limits, be conservatively non-inferior to every required PM comparator, and show strict uncertainty-separated improvement in the same preregistered circularity benefit metric against every comparator. No weighted scalar score.

See MAG_AM_CIRCULAR_MOTOR_LIFECYCLE_BENCHMARK_2026-11-28.md, benchmark-plan.toml and evaluator/.
