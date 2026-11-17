# HoloFrac Sensor-Placement and Virtual-Sensing Protocol v0

**Status:** synthetic geometric baseline; no measured-building or HLB superiority claim  
**Implementation:** \`research/holofrac-retrieval/src/sensor_placement.rs\`  
**Executable:** \`research/holofrac-retrieval/src/bin/sensor_placement_sweep.rs\`  
**Workflow output:** \`sensor-placement.csv\`  
**Related issue:** [#5974 — sensor placement, virtual sensing and adaptive observation](https://github.com/Luminous-Dynamics/symtropy/issues/5974)

## 1. Research question

Does a spatial sensing layout improve reconstruction of multichannel fields at a fixed sensor count, and can HLB later improve the placement decision under held-out spatial/temporal conditions?

The first benchmark isolates placement geometry. It compares three deterministic policies:
- regular-grid placement;
- seeded random placement without replacement;
- greedy maximin placement, selecting the next candidate that maximizes its minimum distance to an already-selected sensor.

This is not an optimal sensor-placement result. It is a baseline comparison that creates a reproducible reference for future information-aware, observability-aware and decision-value-aware methods.

## 2. Synthetic benchmark contract

The candidate sensor pool is a regular 2D lattice in a unit-square, building-local coordinate frame. Each selected sensor observes all six analytic channels at each sampled time. Query points are a separate spatial grid. The virtual sensor uses the nearest selected sensor's value for the same channel and time.

The first slice is noise-free to isolate the geometric effect of placement. It uses the same deterministic synthetic field functions as the HoloFrac temporal-observation fixture:
- temperature;
- illuminance;
- relative humidity;
- moisture;
- airflow;
- occupancy density.

It does not model real sensor error, calibration drift, correlated failures, boundary conditions, sensor height, room occlusion or physical process constraints. Those are follow-on benchmark dimensions.

## 3. Reported metrics

For each policy and sensor count:
- candidate and selected sensor counts;
- query grid and time sample counts;
- channel and query counts;
- mean and maximum distance to nearest sensor;
- mean absolute error (MAE);
- root-mean-square error (RMSE).

The executable sweeps 4 / 8 / 16 / 24 sensors from a fixed 20×20 candidate grid, using a fixed query grid, 24 time samples and a fixed seed. Raw CSV records the exact source SHA and runtime target architecture/OS.

## 4. Interpretation limits

- Maximin is a geometric coverage heuristic, not a guarantee of best prediction or best engineering decision.
- Regular-grid is a simple structured baseline, not necessarily the industry's strongest placement method.
- Random placement is reported with a fixed seed for deterministic replay; repeated-seed distributions should be added before inferential claims.
- Nearest-neighbour virtual sensing is a deliberately simple baseline. Future comparisons should include bilinear, RBF, Gaussian process, multi-target regression and temporal models where applicable.
- The initial noise-free fixture cannot establish robustness to real sensor uncertainty or sensor degradation.

Do not claim HLB superiority from beating only random placement. The next comparison set should include maximin/space-filling, sequential A-optimal, uncertainty-based and expected-decision-value selection under identical candidate pools, sensor counts, calibration assumptions and budgets.

## 5. Next benchmark extensions

1. Add seeded multi-run random placement and confidence intervals.
2. Add noise, missingness, delayed readings, calibration drift and sensor failure, consuming the bitemporal fixture.
3. Add multi-target virtual sensing and leave-one-sensor-out evaluation.
4. Hold out spatial regions, complete time episodes and full sensor deployments.
5. Add sensor cost, power, bandwidth, maintenance access and communication failure domains.
6. Compare sensor selection by expected decision value against fixed sensor count and cost.
7. Only after synthetic controls pass, reproduce a rights-cleared measured-building benchmark with a cross-year or cross-deployment split.

## 6. Claim ceiling

Passing the Rust tests establishes deterministic policy generation and metric computation for this synthetic setup. It does not establish sensor placement optimality, virtual-sensing validity, physical observability, independence of inferred observations, building energy improvement, occupant outcomes or engineering qualification.
