# EV factory queue-age fairness frontier v0.1

**Purpose:** quantify how the changeover-aware policy's elapsed-age override trades setup reduction against throughput, per-variant waiting and sequence integrity. This is a sensitivity screen, not a production wait-time SLA.

## Why this exists

A maximum queue-age threshold is a policy parameter, not a free improvement. Shorter thresholds should give older work priority sooner, potentially sacrificing setups; longer thresholds may improve same-variant batching while tolerating longer waits. A single selected threshold hides this trade-off. The benchmark therefore reports a *frontier* rather than declaring one threshold optimal.

The selector checks queue age only when a server reaches a dispatch opportunity. A threshold of 900 seconds does not guarantee service within 900 seconds: busy servers, machine failures, output blocking, starvation upstream and downstream congestion remain outside the local selector's control.

## Executable comparison

The EV factory CI workflow compares the same immutable FIFO baseline and same explicit 20-seed set against `minimize_changeover` at thresholds **300, 900, 1800 and 3600 seconds**. It preserves identical fixture, station capacities, supplier arrivals, quality and failure streams. The 900-second arm reuses the ordinary paired-policy run; the other three arms are simulated directly from the same fixture with only the threshold changed.

For each threshold and each of the six baseline/one-extra-server scenarios, the report stores seed-level paired deltas and 95% paired t-intervals for:
- good output rate and completed good units;
- total station changeovers;
- line-wide release-order inversion count; and
- each product variant's mean, p95 and maximum queue wait.

A report verifier checks the exact threshold list, policy names, paired seed set, scenario count, sample counts and per-variant dimensionality. Output is hash-manifested with the existing simulation artifacts. No weighted score may hide a worsened wait tail or sequence-integrity regression.

## Decision and holdout discipline

This 20-seed threshold sweep is an initial screening experiment only. It may rule out obviously poor settings; it must not be used to claim a universal optimum. Before selecting a default:
1. Predeclare at least 100 replication seeds and stress strata (variant mix, utilization, buffer capacities, failures/repairs, demand bursts).
2. Freeze a development set, select the threshold there, and report the selected configuration on untouched holdout seeds/scenarios.
3. Retain a Pareto view across throughput, changeovers, per-variant mean/p95/max wait and order inversions. If no setting dominates, present the trade-off rather than hiding it inside one weighted number.
4. Require a prospective site-calibrated shadow-mode trial before any production integration; this synthetic simulator never authorizes MES/PLC dispatch.

## Relation to research and industry

Published dynamic job-shop work shows that selecting among rules as conditions change can outperform a single rule on its benchmark, while robustness differs between periodic and real-time selection. That motivates evaluating the operating regime and a portfolio, not claiming the same effect on this line: Marques, Figueira and Guimarães, *Computers & Industrial Engineering* 210 (2025), [10.1016/j.cie.2025.111471](https://doi.org/10.1016/j.cie.2025.111471). A later research step may add periodic portfolio selection, but only when job attributes, state/action visibility and holdout protocol are defined.

NIST's 2026 smart-manufacturing roadmap highlights integration across heterogeneous sensing/control systems and demands for trustworthy, explainable and reliable operation ([roadmap](https://www.nist.gov/publications/2026-roadmap-artificial-intelligence-and-machine-learning-smart-manufacturing)). For this work, reproducible inputs, paired comparisons, explicit claim limits and no production authority are part of the benchmark, not optional polish.

## Claim ceiling

The sweep is a deterministic software workflow around stochastic replication over a synthetic discrete-event model. Results are not site calibration, a factory digital twin, a guaranteed service level, an industry-SOTA result or authorization for physical control. The threshold list and intervals describe only this frozen test protocol.
