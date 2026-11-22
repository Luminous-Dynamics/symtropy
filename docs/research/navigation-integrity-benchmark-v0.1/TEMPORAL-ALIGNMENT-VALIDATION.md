# Temporal integrity and synchronization-holdover validation receipt

**Related PR:** [#6785](https://github.com/Luminous-Dynamics/symtropy/pull/6785)
**Exact head:** `e6e6d77ced462a92db1b6d6885f13bf016023a84`
**Workflow:** [run #180](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45018670378)
**Status:** passed — software and synthetic sensitivity only

## Exact-head validation

- Formatting and whitespace: passed.
- Clippy: passed.
- Tests: 106 passed, 0 failed across `nav-transition-validator` (31), `nav-observation-toy-fim` (63), and `miluv-ingest` (12).
- Student-t, adaptive PID, PL trace, zero-failure, mission-risk and temporal-alignment demos: passed.
- Generalization split and dataset-admission CLIs: passed.

## Synthetic clock/motion sensitivity

Declared inputs: initial clock offset 2 ms; relative scale drift 25 ppm; observation age 30 ms; maximum linear speed 3 m/s; linear acceleration 1 m/s²; angular rate 0.5 rad/s; angular acceleration 1 rad/s²; lever arm 0.2 m.

| Elapsed since sync | Offset + drift | Total temporal bound | Position-bias bound | Inflated Student-t HPL from 5 m |
|---:|---:|---:|---:|---:|
| 1 s | 2.025 ms | 32.025 ms | 0.099893 m | 5.099893 m |
| 10 s | 2.250 ms | 32.250 ms | 0.100599 m | 5.100599 m |
| 100 s | 4.500 ms | 34.500 ms | 0.107664 m | 5.107664 m |

## Synchronization holdover demo

Declared inputs: 1 ms offset; 100 ppm drift; 5 m/s speed; zero acceleration/rotation/lever arm; temporal position-bias budget 1 cm; maximum search horizon 100 s.

Result: maximum additional holdover `10.000000000` s. At that point the declared timing-derived position-bias bound reaches the 1 cm budget. The solver reports that the budget is satisfied now but not at the 100 s search limit.

## Composition

The implementation adds the deterministic temporal position-bias bound to overall, longitudinal and cross-track Student-t protection levels using the triangle inequality. It does not assume timing bias is independent of estimator error and does not RSS-combine them.

The composition is valid only if the timing bound is itself a justified worst-case bound and has not already been included in the base covariance/PL. Otherwise double counting is possible.

## Claim ceiling

These are deterministic calculations under synthetic caller-supplied maxima. The code does not estimate or qualify clock offset/drift, observation age, motion maxima, or lever-arm calibration. The temporal envelope is not a covariance, probability or protection-level certification by itself. Physical promotion requires independent timing instrumentation, documented timestamp semantics, resynchronization evidence, ODD-specific motion bounds and held-out route containment tests. The Hilti reference's gyro-correlation synchronization is not independent proof of its residual timing error. No physical timing data, Hilti payload or UrbanNav route was used.