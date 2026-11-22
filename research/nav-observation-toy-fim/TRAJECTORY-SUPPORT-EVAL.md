# Trajectory availability versus evidence support evaluator v0.1

## Motivation

Modern SLAM benchmarks often reward trajectory completeness and position accuracy. The Hilti–Trimble–Oxford challenge, for example, requires at least 99% reference-pose coverage and applies an exponential per-pose accuracy score. This is a valuable compatibility metric, but trajectory output alone does not say whether each pose is supported by a fresh measurement, bounded dead reckoning, offline interpolation or extrapolation.

The evaluator therefore reports two distinct coverage measures:

- **Output/official-style coverage:** any emitted pose counts as available, including interpolated and extrapolated poses.
- **Evidence-backed coverage:** only a qualified measurement or explicitly bounded dead-reckoning interval counts as qualified support.

An output can satisfy a completeness threshold while being poorly supported. That difference must be visible to downstream service policy and independent evaluation.

## Input contract

Each record supplies a pre-matched timestamp/frame reference interval, explicit support duration, cluster/session identity, reference-validity state, output-support class, service disposition and position error when truth is valid and a pose is present.

Supported classes:

- `MeasuredQualified`: direct measurement/fusion path has qualified evidence support.
- `DeadReckonedBounded`: propagation remains within an explicitly bounded support interval.
- `MeasuredUnqualified`: estimate uses measurements but the evidence path is not qualified.
- `DeadReckonedUnbounded`: propagated estimate has no qualified bound.
- `Interpolated`: offline fill between output samples.
- `Extrapolated`: output beyond an observed/initialized interval.
- `Missing`: no trajectory output.

`NORMAL` with anything except `MeasuredQualified` or `DeadReckonedBounded` is counted as `normal_without_qualified_support`; it is not silently treated as valid service. The evaluator measures this policy violation rather than hiding it.

## Metrics

- official-style matched-pose coverage and the 99% gate;
- evidence-backed coverage and unqualified-output coverage;
- counts/exposure by measured, bounded dead-reckoned, unqualified, interpolated, extrapolated and missing support;
- NORMAL/DEGRADED/INTEGRITY_UNCERTAIN/UNAVAILABLE/RECOVERING/INDETERMINATE exposure;
- NORMAL exposure without qualified support;
- maximum contiguous missing and unqualified-output intervals, split by gaps and session boundaries;
- p50/p95/p99/max position-error quantiles for all, evidence-backed and unqualified output poses;
- challenge-compatible exponential per-pose score after the caller has matched timestamps/frames and applied reference-validity windows.

## Important limits

This is not the official Hilti evaluator. It does not parse ROS bags, synchronize camera/IMU clocks, transform poses, associate timestamps, apply the official first-five-second exclusion or communicate with the submission server. It consumes already-matched records. Its exponential score formula follows the published per-pose curve and 99% coverage gate for compatibility testing only.

Invalid reference intervals are reported separately and never counted as safe. Missing truth is not converted to zero error. High-rate samples and overlapping windows are not independent trials. Cluster IDs are used to keep different sessions and clock origins separate.

## Proposed benchmark interpretation

Always publish the official compatibility score **and** the evidence-support/service metrics. Do not game the 99% coverage threshold by silently filling gaps: interpolation/extrapolation must be labelled and included in the unqualified-output exposure. Where bounded dead reckoning is used, publish the propagation horizon, bound derivation and requalification rule.

## Future work

- connect per-record support classes to signed source/evidence receipts from the mission-risk contract;
- compute time-varying protection levels and reachable sets during bounded dead reckoning;
- add causal fault-injection labels to each support transition;
- include energy and latency for recovery, not only trajectory accuracy;
- compare against the official ACDC-VSLAM and floorplan localization winners on the exact held-out sequences, under the dataset's non-commercial license.

All current unit tests and the demo use synthetic, pre-matched records. No physical dataset is downloaded or scored by this module.