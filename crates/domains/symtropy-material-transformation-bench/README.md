# Symtropy Material Transformation Bench

Standalone, read-only evaluator for **stage-resolved dry-solid mass flow and particle-size distribution changes** between physical material-stream boundaries.

Implements [issue #3925](https://github.com/Luminous-Dynamics/symtropy/issues/3925). It is downstream of #3864/#3876 and can use their exact PSD binning as its parent authority. It complements #3891/#3910 morphology, #3877/#3913 conveying-state evidence, #3888/#3890 spatial floor-residue geometry, #3814 full-cycle metrology and #3825 resource accounting.

## Run

    cargo test --manifest-path crates/domains/symtropy-material-transformation-bench/Cargo.toml

## v0.1 contract

- Exact parent case/interval/trial/machine/material cohort/transport configuration identity.
- Explicit physical measurement stations and unique material-stream records.
- Each stream has a synchronized observation window, time uncertainty, dry-solid mass-flow interval and independently measured size-bin mass-flow intervals.
- Size bins must exactly match the parent PSD grid, including the open-ended final bin.
- Conservative interval sum of per-bin flow must overlap the independently measured stream-total interval.
- Transitions connect one or more input streams to one or more output streams; a stream can be produced once and consumed once.
- Stream topology is acyclic.
- Residence-time intervals are checked against observed input/output window midpoint displacement, expanded only by explicit time uncertainty.
- Aggregate input and output flow intervals yield WITHIN_UNCERTAINTY, DEFINITE_DEFICIT or DEFINITE_SURPLUS. Invalid/unmeasured boundaries remain errors/unknown, never zero.
- Per-bin output-minus-input flow-change intervals are reported. They are not causal attribution.
- Every stream is explicitly labelled as a material route: generated, captured, floor residual, line deposit, separator coarse, filter fines, reject, accepted discharge or other declared stream.

## Important non-claims

A downstream increase in fines or decrease in coarse material does **not** prove breakage. It can also arise from selective capture, segregation, sampling bias, unmatched cohorts, time misalignment or unmeasured branches. The evaluator reports measured distribution changes and mass-flow closure; it does not infer a particle-level breakage matrix.

Number-based morphology counts from #3891 cannot be converted to mass fractions without a separately qualified sampling/weighting model. Friction, strength, density, moisture, attrition rate and pipe erosion remain separately measured physical quantities.

The bundled fixture is fabricated conformance data. Tests are not execution evidence until run on the exact commit.
