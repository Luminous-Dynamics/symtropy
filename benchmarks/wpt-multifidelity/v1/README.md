# WPT multi-fidelity literature benchmark v1

This directory stores literature-derived comparison cases separately from synthetic known-truth physics fixtures and physical measurement records.

## Lee & Lee (2026), heterogeneous 1T–3R

Source: Lee, J. & Lee, S. B., *Integrated electromagnetic–circuit digital twin modelling for constraint-consistent optimal operation in heterogeneous multi-receiver wireless power transfer systems*, Scientific Reports 16, 19957 (2026), DOI 10.1038/s41598-026-50781-y. Open access CC BY 4.0.

File: `lee-lee-2026-1t3r-literature-baseline.json`.

### What is transcribed

The record contains the source-reported 400 kHz / 10 mm-gap 1T–3R case; coil dimensions, turns, self inductances, resistances and the full 4×4 mutual-inductance matrix; the mixed compensation topology; the MMFD design settings; receiver output powers; reported steady-state efficiency; convergence/evaluation count; and workstation/runtime details.

The matrix is stored in the paper's displayed units of microhenries. Its independent numerical eigenvalues, calculated from the rounded table entries, are recorded as a derived check. The 4×4 matrix is symmetric and positive definite for the rounded published values. This is **not** an independent FEM mesh-convergence check, measured matrix or full physical passivity certificate: temperature, DC bias, dot-terminal map, FEM mesh/solver receipts and raw waveforms are not supplied by the table.

### Important source inconsistency

The paper states a ±3% receiver-power tolerance and reports converged powers:
- R1: 12.05 W against 12.5 W rated (-3.6%);
- R2: 4.98 W against 5 W rated (-0.4%);
- R3: 5.05 W against 5 W rated (+1.0%).

The stated 3% lower bound for R1 is 12.125 W. The table's 12.05 W is 0.075 W below that boundary. The paper's text says all receiver constraints are satisfied, but the displayed R1 output contradicts the stated ±3% constraint. This is marked `REPRODUCTION_BLOCKED_PENDING_SOURCE_CLARIFICATION`; do not silently change the reported value or declare the paper's operating point feasible under that rule.

### Fair comparison

This paper is a design-stage, fixed-geometry, fixed-coupling, steady-state 1T–3R model. It is a relevant benchmark for heterogeneous receiver constraints and electromagnetic-to-circuit integration. It is not a dynamic roadway case, a grid-to-battery efficiency record, a source-to-load energy ledger or a hardware road trial.

The reported 73.01% metric is the paper's steady-state circuit transfer metric (sum receiver output power divided by transmitter input power). It must remain separate from grid-to-battery DC–DC, inverter-bus-to-vehicle-bus and route-integrated Wh/pass scores.

Reported compute baseline: 33 optimizer iterations, 543 circuit evaluations, average 0.5 minutes/evaluation, 4.52 hours total, one initial electromagnetic extraction, on an Intel Core i5-10400-class 2.90 GHz / 32 GB Windows 10 workstation with ANSYS Maxwell and Simplorer. These figures are source-reported and hardware/software-specific; they are not a direct runtime contest without a matched implementation and machine.

### Extension programme

1. Reproduce the fixed 1T–3R matrix/circuit operating point, after resolving the R1 tolerance discrepancy.
2. Compare MMFD with deterministic constrained SQP, sequential quadratic programming, augmented Lagrangian and robust/lexicographic feasibility-first baselines under the same model and initial conditions.
3. Add full-matrix position dependence using the spatial atlas (PR #3563), with mode/regime boundaries explicit.
4. Add CC/CV load profiles, time-varying loads, temperature and geometry holdouts.
5. Extend from 3 receivers to 2–8 receivers and test service fairness as well as total efficiency.
6. Propagate matrix/circuit uncertainty into receiver Wh and route service, then compare model-reduction and learned-surrogate approaches.

The JSON record intentionally preserves unreported metadata as null or explicit gaps rather than inserting invented temperature, bias or polarity assumptions.