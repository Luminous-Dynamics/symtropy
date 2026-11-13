# DAS Data-Lifecycle and Evidence-Retention Benchmark v0.1

**Workstream:** DAS-DATA-LIFECYCLE-QUAL-001 (#5003)  
**Owners:** Symtropy #4972 / data lifecycle #5003; Sol-Atlas #755 for evidence representation  
**Status:** research protocol and synthetic campaign plan only  
**Date:** 2026-11-13

## 1. Research objective

Minimize the cost of moving, processing, retaining and retrieving DAS data while preserving the evidence needed for event detection, localization, uncertainty estimation, independent adjudication and replay.

The objective is not maximum compression. It is minimum total lifecycle cost subject to pre-registered non-inferiority constraints for critical event probability of detection, localization error, uncertainty coverage, source integrity and replay.

## 2. Multi-resolution evidence tiers

| Tier | Content | Requirement |
|---|---|---|
| T0 Source | Vendor-native interrogator output and exact acquisition configuration | Immutable when available, permitted and acquired |
| T1 Archive | Lossless chunked copy of selected/all raw source windows | Byte-digest verified; source parent preserved |
| T2 Normalized | Declared unit/time/axis normalization | Versioned reversible mapping where practical; never overwrite source |
| T3 Derived | Features, spectrograms, strain maps, embeddings, event candidates | Parent artifact, exact processing code/config and distortion/error report |
| T4 Decision | Claim level, coverage, uncertainty, decision value, adjudication and retention receipt | Source and derivation references remain resolvable |

High-rate waveforms should be stored as immutable objects in a declared container (vendor-native, HDF5 or Zarr where suitable), not expanded into an unbounded number of REST observations. A SensorThings observation can reference the source/derived object, time interval, observed property, sensor and feature of interest.

## 3. Candidate acquisition/retention strategies

- S0 continuous full-rate raw retention;
- S1 continuous lossless compression;
- S2 continuous lower-rate screening plus high-rate pre/post ring buffer;
- S3 event-triggered high-rate capture;
- S4 edge feature extraction plus selective raw retrieval;
- S5 lossy compression at pre-registered distortion budgets;
- S6 downsampled monitoring;
- S7 feature-only retention (negative/control candidate unless the declared task does not require waveform replay);
- S8 adaptive policy selected by event consequence, data quality, expected information and retention cost.

All strategies must use the same source population, acquisition windows, event labels and held-out sessions. The source baseline is replayed before/after transformation.

## 4. Safety and integrity rules

- A digest binds the exact bytes acquired; it does not prove publisher identity or measurement truth.
- A derived artifact must reference its exact parent digest, transformation method, version and parameters.
- Lossy transforms create new artifacts and cannot replace or be represented as raw source.
- Missing, dropped, unavailable, unobserved or unqualified samples remain explicit; never fill them as zero or "no event".
- An event-triggered design must declare pre-trigger context, post-trigger duration, refractory/merge behavior and overflow policy.
- When data quality is unknown or degraded, a detector cannot clear an event simply because the reduced representation lacks a signature.
- Rights, purpose, retention and disclosure rules are applied to raw and derived artifacts independently.
- SCADA/protection remains outside the data pipeline's write authority.

## 5. Benchmark metrics

**Evidence integrity**
- percentage of acquired chunks with verified byte digests;
- complete parent/derivation lineage;
- missing/dropped samples and unresolved time gaps;
- deterministic replay fidelity;
- retention/deletion and access-control correctness.

**Signal/decision preservation**
- event POD and critical miss rate by class/magnitude;
- false alarms per observable fibre-km-day;
- detection latency P50/P95;
- chainage and geographic error P50/P95;
- 90%/95% interval coverage and interval width;
- rare-event retention;
- human adjudication agreement and dispatch workload.

**Systems economics**
- bytes per channel-hour, fibre-km-day and qualified observable km-year;
- effective compression ratio;
- ingest/egress throughput, queue age and query latency;
- CPU/GPU time, memory, energy and cost per fibre-km-day;
- retrieval cost and time to obtain an adjudication window;
- cost per independently accepted actionable event.

**Robustness**
- behavior under backpressure, outage, recovery and tier migration;
- ring-buffer completeness under trigger bursts;
- OOD/degraded detector behavior;
- ability to reproduce a result from an exact source digest.

## 6. Non-inferiority and fair comparison

Before evaluation, freeze:
- event classes, magnitude bands and consequence tiers;
- source data, independent split groups and reference truth;
- acceptable critical miss-rate margin;
- localization and uncertainty guardrails;
- storage/compute budget and declared cost units;
- compression algorithm/version/parameters;
- trigger thresholds, buffer lengths and overflow behavior.

A reduced representation qualifies only if the pre-registered statistical guardrails hold on held-out acquisition sessions. Compression ratio alone is never a pass. Report every missed event and every replay failure.

## 7. Proposed targets (not achieved results)

- Zero unaccounted sample loss in the declared lossless path under the frozen load/outage envelope.
- 100% digest verification and parent-reference resolution in the finite campaign.
- Zero high-confidence negative event claims from unobserved, missing or unqualified intervals.
- No degradation beyond the pre-registered non-inferiority margin for critical event POD, localization and interval coverage.
- 100% of triggered windows preserve the declared pre/post context or expose an explicit overflow/incomplete disposition.
- Cost savings are reported as total lifecycle cost per qualified observable km-year and per independently accepted actionable event.
- Feature-only retention is explicitly ineligible for tasks requiring raw waveform replay.

## 8. Reference literature and data

- Rafi et al. (2026), *Big data architectures for distributed acoustic sensing: a review of storage, real-time processing, and scalable analytics*, DOI 10.1016/j.rio.2026.101028.
- OOI Regional Cabled Array multi-span DAS 2025–2026 data, DOI 10.5281/zenodo.19930457. This is a subsea observatory population and is suitable for stress testing long-duration ingestion, interruptions and data volume—not for terrestrial utility detection claims.

## 9. Claim ceiling

This protocol does not establish that a Luminous pipeline exists, that any compression method is safe, that storage costs have been reduced, or that DAS event performance survives data reduction. Results must be generated from exact, rights-cleared data and independently reproduced.
