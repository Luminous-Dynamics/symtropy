# Multirate event-time qualification v0.1

Tracking: [SHIPYARD-MULTIRATE-EVENT-TIME-QUAL-001 #4947](https://github.com/Luminous-Dynamics/symtropy/issues/4947)

## Purpose and paradigm

This is the temporal evidence data plane between:
- #4643: source rights, stream roles and BAM-specific channel cadence;
- #4433: workpiece/pass/equipment identity and source-native event semantics;
- #4649: fresh predecessor scan and process event windows for next-pass planning;
- #4614: physical-model inputs and uncertainty provenance.

The paradigm shift is **event-time evidence, not row joins or eager resampling**. Each source keeps its own clock domain, timestamp, sequence, sample cadence and quality. The event join is a derived receipt recording the transform, uncertainty and exact membership decision; it does not rewrite source data.

V0 is synthetic only: no BAM raw intake, no NetCDF/HDF5 parser, no live controller, no weld-quality/NDT inference and no interpolation.

## Source facts

Fabry et al. (2026), Data in Brief 69, 113329, DOI https://doi.org/10.1016/j.dib.2026.113329, documents UTC timestamps on a common TwinCAT 3 Runtime / EtherCAT master clock with distributed clocks. It reports less than 10 µs drift for the relevant Beckhoff analog-to-digital terminal signals only. This is not a universal bound for upstream transports.

Published modality intervals differ:
- current / voltage: 0.1 ms;
- robot TCP: 8 ms (robot file minimum interval 1 ms);
- power-source OPC-UA: 100 ms;
- control-system power/gas/thermocouple: 1 ms;
- laser scan profile triggers: 50 Hz (20 ms);
- video triggers: 40 Hz (25 ms).

A shared time coordinate is not a shared sample index, sample cadence, zero uncertainty or permission to interpolate. The Zenodo archive (DOI 10.5281/zenodo.21824586) remains RIGHTS_PENDING; this workstream uses published metadata and synthetic fixtures only.

## Clock transform

Time is signed integer nanoseconds. The source-to-master transform is:

`master_ns = reference_master_ns + round_nearest_away((source_ns - reference_source_ns) × (1,000,000,000 + drift_ppb) / 1,000,000,000)`

All arithmetic uses checked i128 intermediates and checked i64 output. A nonpositive affine slope, overflow, invalid digest or source timestamp outside the calibration validity interval abstains.

Mapping uncertainty conservatively sums source timestamp uncertainty, transform calibration uncertainty, and drift uncertainty grown with absolute distance from the reference source epoch (rounded upward to whole nanoseconds). Common-clock declarations are never treated as zero-uncertainty certificates; zero or missing uncertainty components fail closed.

## Event membership

Events use half-open `[start,end)` master-time intervals, with explicit start/end boundary uncertainty. Each mapped sample also has a timestamp uncertainty envelope.

A sample is:
- definitely inside only if its entire uncertainty interval lies inside the conservative event interior;
- definitely outside only if its entire interval lies outside every possible event boundary;
- boundary-ambiguous if its interval overlaps a possible/actual boundary. Ambiguous membership abstains rather than choosing a convenient pass.

Source event time and receive/ingest time remain separate. A sample received after the decision cutoff cannot influence that decision. Late arrivals remain distinguishable from event-time timestamps and are not retimed.

## Stream completeness and quality

The conservative expected-minimum count uses event-interior duration divided by `nominal_period + 2 × allowed_jitter`. Coverage is compared with that expectation and capped at 1000 permille. Maximum gap includes event-window edges and internal valid-sample gaps. A stream too sparse to resolve the event abstains.

Only valid samples count toward coverage. Missing timestamp, clipped, dropped, invalid, late, stale and out-of-window records are counted separately. Missing timestamps are never replaced by epoch zero. No hold-last, interpolation or resampling is performed. Stale OPC-UA state is excluded; if coverage/gap policy then fails, the stream abstains.

Causal lead/lag is resolvable only when absolute lag exceeds the sum of timestamp uncertainty bounds. This is a temporal resolvability condition, not proof of causation.

## Receipt contents

Each stream receipt carries exact stream/clock identity, source-byte digest, calibration/transform receipt digest, raw and mapped timestamp ranges, sequence range, source/included/expected sample counts, coverage, max gap including event edges, maximum mapping uncertainty, duplicate/out-of-order/missing/clipped/dropped/invalid/late/stale/out-of-window counts, and a narrow `TEMPORAL_ALIGNMENT_ONLY` claim ceiling.

Event receipts sort stream receipts by stable stream ID so input array order cannot change output order. Alignment does not imply measurement validity, semantic agreement, process quality or authorization.

## Industry comparison

Moscatelli et al. (2026), *Vision-based real-time autonomous trajectory planning in robotic laser welding applications*, reports event-driven target generation and a nominal 35 ms perception-to-execution latency managed through adaptive synchronization in a robotic laser-welding setup. It is a trajectory-control result, not a sensor event-alignment benchmark; its reported seam deviation (<200 µm in that study) is not comparable to a temporal alignment error. DOI https://doi.org/10.1016/j.jmapro.2026.08.049.

A related 2026 latency-aware robotic laser-welding paper uses trajectory-speed-driven adaptive image acquisition to reduce redundant data and processing time while maintaining trajectory planning/tracking quality (DOI https://doi.org/10.1016/j.procir.2026.07.012). This supports event-driven, cadence-aware acquisition rather than imposing one high-frequency grid on every modality.

A 2026 closed-loop welding survey highlights a maturity gap between the forward perception-to-execution path and backward assessment-to-model/decision feedback. Correct source/arrival-time and event receipts are prerequisites for that backward loop: without them, inspection, repair and process signals can attach to the wrong pass or future knowledge can leak into past decisions.

Our SOTA target is not to beat the 35 ms trajectory-latency number with an unrelated metric. It is deterministic, uncertainty-bounded event membership across heterogeneous streams, with auditable calibration validity and zero false joins on the frozen adversarial corpus.

## Frozen synthetic oracle

The machine-readable corpus freezes 31 controls: one positive multi-rate known-answer case and 30 negative controls for clock identity, offsets/drift, calibration expiry, digest binding, uncertainty, overflow, sequence/timestamp ordering, missingness, receive time, future leakage, half-open boundaries, boundary ambiguity, cadence fabrication, stale slow-state, late-arrival retiming, quality flags, interpolation, causal-lag promotion, cross-WID joining and event-edge coverage.

The Rust library contains 39 unit/mutation tests. The corpus verifier checks exact control identity/cardinality and six source cadence declarations. These are software contract results, not hardware synchronization claims.

## Benchmark metrics and goals

Software qualification:
- 100% agreement with independently computed integer known-answer transforms and event memberships;
- zero false temporal admissions on the 30 frozen negative controls;
- zero fabricated timestamps, default epochs, implicit resampling or overflow wraparound;
- deterministic event receipt replay and stable receipt ordering.

Only after lawful physical data admission:
- offset/drift estimates and uncertainty by channel/clock domain;
- hardware vs software timestamp delta distributions;
- event-window boundary ambiguity rate;
- per-channel edge/internal gap and coverage distributions;
- arrival-lateness, out-of-order, duplicate and dropped-sample rates;
- false-join rate under injected pass/WID/clock faults;
- calibration expiry behaviour and uncertainty coverage;
- throughput/storage for 10^3–10^7 samples while preserving original cadence.

A zero false-join count on a finite synthetic corpus is not proof of actual hardware synchronization. Physical timing accuracy requires calibration certificates and independent timing experiments.

## Standards and procedure interface

ISO/TS 8182:2026 is published and addresses reporting arc-energy parameters in WPQR and transferring them into WPS production specifications for arc welding. A future arc-energy adapter can consume qualified current/voltage, run length/speed, weaving and temperature observations from this event-time layer; this verifier does not calculate or authorize a WPS energy window.

ISO/FDIS 15614-1 Edition 3 is in final approval but remains under development. The published ISO 15614-1:2017 remains the current published baseline until the new edition is published. Do not claim Edition 3 compliance or use event alignment as WPS/PQR authority.

## Claim ceiling

A PASS proves only that synthetic timestamps, clock transforms and event windows satisfy the declared integer-time and uncertainty contract. It does not establish actual hardware synchronization, sensor calibration, arc-energy conformity, causality, weld quality, NDT acceptance, WPS qualification or permission to use the BAM raw archive.
