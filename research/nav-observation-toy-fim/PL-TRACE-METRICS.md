# Protection-level trace metrics v0.1

**Issue:** [NAV-PL-TRACE-METRICS-001 #6744](https://github.com/Luminous-Dynamics/symtropy/issues/6744)  
**Purpose:** reproduce paper pointwise PL exceedance while separating physical breach events and exposure.

## Record contract

Each record provides:
- unique record ID and predeclared route/session/event cluster;
- start timestamp and explicit support duration;
- truth-valid flag and along/cross-track errors if truth is valid;
- along/cross-track protection levels.

The evaluator does not infer sample duration from row spacing. Invalid truth requires both error fields be absent; it is counted as reference-invalid and is not silently treated as safe.

Support intervals may have gaps but may not overlap. The trace is sorted deterministically by timestamp and record ID.

## Metrics

- pointwise along-track, cross-track and union exceedance count/fraction (Stanford-style);
- valid/invalid reference sample counts and distinct cluster count;
- valid exposure and time above PL by axis/union;
- contiguous breach episode counts, total duration, maximum and mean episode duration;
- maximum error excess beyond the bound, returned in micrometers (integer; divide by 1,000,000 for metres);
- descriptive either-axis breach episodes per valid exposure hour, scaled by 1,000,000 (not a probability).
- descriptive breach-event rate per valid exposure hour.

A breach episode continues only through adjacent support intervals whose samples exceed the same bound **within the same predeclared cluster**. A cluster boundary, safe interval, invalid-truth interval or time gap terminates the episode. The trace treats each sample's exceedance state as constant over its explicit support interval; this is not continuous-time error interpolation.

## Important statistical limit

Pointwise exceedance fraction is the paper's sample-level Stanford-diagram metric. It is not automatically a calibrated per-hour or per-mission integrity probability. Event counts and exposure durations are descriptive. Uncertainty estimation must cluster by the predeclared independent route/session/event unit; do not use individual high-rate epochs as independent trials.

The event-rate metric is not a probability and should not be labelled an integrity risk guarantee. This evaluator computes metrics from supplied truth/PL pairs; it does not calibrate the estimator, validate truth independence or certify service integrity.
