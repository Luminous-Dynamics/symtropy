# Temporal-alignment integrity validation receipt

**Related PR:** [#6785](https://github.com/Luminous-Dynamics/symtropy/pull/6785)
**Exact head:** `4aad5d3577e1576ffe46263d65d005ecab6f71e1`
**Workflow:** [run #177](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45018437180)
**Status:** passed — software and synthetic sensitivity only

## Exact-head validation

- Formatting and whitespace checks: passed.
- Clippy: passed.
- Tests: 102 passed, 0 failed: `nav-transition-validator` 31, `nav-observation-toy-fim` 59, `miluv-ingest` 12.
- Student-t, adaptive PID, PL trace, zero-failure, mission risk and temporal-alignment demos: passed.
- Generalization split schema/CLI and dataset-admission CLIs: passed.
- No payload is admitted by the base #6760 registry.

## Synthetic temporal demo

The declared synthetic bounds use 2 ms initial offset, 25 ppm relative clock scale error, 30 ms observation age, 3 m/s speed, 1 m/s² acceleration, 0.5 rad/s angular rate, 1 rad/s² angular acceleration and 0.2 m sensor lever arm.

| Time since synchronization | Clock offset+drift bound | Total time bound | Position-bias envelope | Inflated base HPL (5 m) |
|---:|---:|---:|---:|---:|
| 1 s | 2.025 ms | 32.025 ms | 0.099893 m | 5.099893 m |
| 10 s | 2.250 ms | 32.250 ms | 0.100599 m | 5.100599 m |
| 100 s | 4.500 ms | 34.500 ms | 0.107664 m | 5.107664 m |

These values are deterministic calculations from synthetic declared maxima, not observations from Hilti, UrbanNav, MILUV or a physical vehicle.

## Composition

The implementation adds the deterministic temporal position-bias bound to overall, longitudinal and cross-track Student-t protection levels. This is based on the triangle inequality and does not assume the temporal error is statistically independent of the estimator error.

The composition is valid only if the timing bound is itself a justified worst-case bound and has not already been included in the base covariance/PL. Otherwise double counting is possible. Root-sum-square is not used.

## Claim ceiling

The code does not estimate or qualify clock offset/drift, observation age, motion maxima or lever-arm calibration. It is not a covariance, probability or protection-level certification by itself. Physical promotion requires an independent timing reference, documented timestamp semantics, resynchronization evidence, ODD-specific motion bounds and held-out route containment tests. The Hilti reference's gyro-correlation synchronization is not an independent proof of its residual timing error.