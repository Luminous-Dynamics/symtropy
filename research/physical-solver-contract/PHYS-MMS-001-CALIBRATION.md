# PHYS-MMS-001 exploratory calibration (not qualification)

**Date:** 2026-10-22  
**Environment:** Python/SciPy 1.17.0 local sparse prototype, IEEE-754 binary64.  
**Purpose:** verify the manufactured equations, the consistent body-force integration, and the expected order bands before consuming a real CalculiX runner.  
**Claim ceiling:** this is not a CalculiX execution, not a run of the new Rust generator/auditor, and not a physical-validation result.

## Setup

The prototype assembled linear CPS3 plane-stress stiffness matrices, integrated the analytic body force with a 12-by-12 Gauss-Legendre tensor rule mapped to each triangle, applied exact zero-displacement boundary conditions, solved the sparse linear system, and integrated field errors against the exact displacement/stress field. The mesh is the same uniform split-square topology and parameter set as the Rust fixture.

## Results

| Cells per axis | Nodes | Triangles | Displacement L2 norm (m²) | H1 seminorm error | Stress tensor L2 error (Pa·m) |
|---:|---:|---:|---:|---:|---:|
| 4 | 25 | 32 | 1.4650992e-7 | 1.5796499e-6 | 257138.43 |
| 8 | 81 | 128 | 4.3297110e-8 | 8.2742004e-7 | 136824.13 |
| 16 | 289 | 512 | 1.1463415e-8 | 4.1796133e-7 | 69756.55 |
| 32 | 1089 | 2048 | 2.9133602e-9 | 2.0945837e-7 | 35065.05 |
| 64 | 4225 | 8192 | 7.3148068e-10 | 1.0478625e-7 | 17556.64 |

Observed order from each pair of consecutive uniform refinements:

| Refinement | Displacement L2 order | H1 seminorm order | Stress L2 order |
|---|---:|---:|---:|
| 4 → 8 | 1.7587 | 0.9329 | 0.9102 |
| 8 → 16 | 1.9172 | 0.9853 | 0.9719 |
| 16 → 32 | 1.9763 | 0.9967 | 0.9923 |
| 32 → 64 | 1.9938 | 0.9992 | 0.9980 |

The final two intervals satisfy the proposed broad ranges: displacement L2 order [1.4, 2.6], H1 order [0.4, 1.6], stress L2 order [0.4, 1.6]. All three errors decrease monotonically across these levels.

## Interpretation and caveats

1. The model's exact formula and sign of the body force were exercised by solving the corresponding weak-form linear elasticity problem, and the rates approached the expected P1-element asymptotic values (2 for displacement L2 norm; 1 for H1 and stress L2 norms).
2. The prototype is an independent local numerical check of the mathematics. It did not invoke CalculiX, did not execute the Rust code, and did not audit any solver-build/process manifest.
3. No conclusion is drawn about the installed or pinned CalculiX solver until the actual generated deck is executed and its raw output is audited against the exact input/build/process manifests.
4. The per-mesh audit labels results MEASURED_UNQUALIFIED; only the entire frozen five-level convergence sequence may pass the defined code-verification gate.
