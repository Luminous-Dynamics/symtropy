# Accuracy-constrained efficiency frontier v0.1

Status: implementation contract for the natural-hazard benchmark programme; no physical simulation result is claimed here.

## Paradigm

Compare model variants on a quality/resource Pareto frontier, not with a weighted realism score or raw frames-per-second. A run is useful only when its physics-specific error envelope is acceptable for the intended claim. Runtime, peak memory, energy, water or quantity-ledger residual and separate physical observables remain distinct.

For flood solver/adaptivity experiments, candidate quality metrics should be selected before running the comparison. They can include water-depth RMSE and signed bias; velocity error where the solver claims velocity; arrival-time or hydrograph phase error; inundation extent overlap and boundary-distance error; absolute and relative water-ledger residual; and lake-at-rest drift, positivity/dry-bed behavior or other required invariant gates.

The Rust helper in efficiency.rs validates measurement identity and performs arithmetic over declared metrics. It does not compute these metrics from fields, choose scientifically appropriate thresholds, or establish that the reference is physically valid.

## Fairness rules

1. Freeze the exact case/configuration identity and compare only matching case digests.
2. Match hardware/backend identity for runtime and memory comparisons. Cross-hardware comparisons require a separate normalized protocol and must not use these paired ratios.
3. Require identical quality metric dimensions for every compared run.
4. Missing energy is not measured, never zero. An energy-aware comparison must meter both runs at the same declared boundary.
5. Publish raw metrics and per-metric quality gate outcomes; do not collapse critical failures into an average.
6. A faster result outside any predeclared quality tolerance fails that quality gate even when its runtime is impressive.
7. Pareto-frontier membership is descriptive, not qualification. The frontier can contain inaccurate runs; each claim must also pass the appropriate physical/reference/observation gate.

## Proposed experiment design

For each compatible flood case, compare uniform fine-grid, static refinement, dynamic error-controlled refinement, and a no-adaptivity ablation under identical forcing, topography, boundary conditions, time horizon and observable support. Repeat across at least three spatial/time-step levels where the scheme supports convergence testing. Report wall time, peak memory and energy only when measured, plus all preregistered quality metrics and run identities.

The stretch goal is lower compute cost at a predeclared error tolerance, not a universal speedup percentage. LISFLOOD-FP 8.2 provides an external comparison target for compatible published cases; its reported speedups must not be transferred to Symtropy.

Related work: https://github.com/Luminous-Dynamics/symtropy/issues/2317, https://github.com/Luminous-Dynamics/symtropy/issues/2306, and https://github.com/Luminous-Dynamics/symtropy/issues/483.
