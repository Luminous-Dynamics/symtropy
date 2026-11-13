# Distributed Fibre Observability — SUF Benchmark Protocol v0.1

**Status:** literature-backed research protocol and synthetic campaign only
**Owner:** Symtropy #4972
**Programme:** SUF #4910
**Spatial evidence:** Sol-Atlas #755

## Central hypothesis

Existing communications fibre may provide a wide-area sensing substrate for some underground infrastructure, reducing the need to patrol every route continuously with mobile robots. This is selective; not all fibre plant is DAS-compatible.

## Literature baselines

### DAS fault/event screening

Kamaruzaman et al. (2026), Results in Optics 24, DOI: https://doi.org/10.1016/j.rio.2026.101093.

The study evaluated DAS on operational SCADA communication fibre along a 5.4 km medium-voltage underground cable corridor in Ipoh. Laboratory testing at 16 kV reported 22.27 dB SNR for direct-buried fibre and 13.54 dB for fibre installed in a buried conduit. A field event was localized around 2.8 km on the calibrated route, with 19.57 dB SNR. Its three-second periodicity matched the HV excitation sequence; ground microphone inspection and excavation independently verified the location. Data are available on request.

This is a bounded event/fault demonstration, not a theft detector or universal classification result. Coupling is a mandatory test factor.

### DAS route geolocalization

Cohen et al. (2026), IEEE Transactions on Geoscience and Remote Sensing 64, DOI: https://doi.org/10.1109/TGRS.2026.3729504.

The study uses traffic-induced quasi-static seismic signals measured by DAS, vehicle GPS/video trajectories, matched-filter initialization and physics-based strain-map inversion with neural trajectory optimization. The university publication record reports sub-metre accuracy, often tens of centimetres, with agreement against tap testing.

This is route geolocation, not fault/event detection. Do not transfer the result across fibre, conduit, soil or slack profiles without matched evidence.

## Paradigm shift

Use three complementary observability layers:

1. Distributed fibre sensing for wide-area indications and fibre-relative chainage.
2. A versioned spatial route model to map chainage into geographic coordinates with uncertainty and coverage states.
3. Targeted robot, surveyor or crew inspection to establish close-range physical truth.

This is wide-area fibre observability plus targeted mobile inspection, rather than robot patrol everywhere.

## Mandatory coupling/profile factors

Stratify direct burial, fibre in conduit, shared trench adjacent to target cable, slack and microbends, soil moisture/compaction, depth, road surface, traffic type, temperature, interrogator gauge length, pulse rate, sample rate, timebase and channel health. An unobservable/decoupled segment remains unknown; absence of a DAS event is not evidence that no event occurred.

## Metrics

**Event screening:** per-class detection, critical false negatives, false alarms per fibre-km-day, event chainage P50/P95/max, alarm latency, SNR, repeatability, uptime, missed/observable length.

**Route geolocalization:** cross-track P50/P95/max, along-route geometry error, 90/95% interval coverage, route completeness, blind length, calibration effort and cost per accepted mapped kilometre.

**Economics:** interrogator capex/lease, fibre access, dark-fibre availability, SCADA/telecom change management, compute/storage, false dispatch, investigation, avoided excavation and maintenance availability.

Do not collapse critical false negatives, blind length or uncalibrated geographic placement into a composite score.

## Fail-closed controls

- Reject stale, incompatible or ambiguous fibre route calibration.
- DAS chainage does not become a geographic coordinate without a versioned route model.
- Zero high-confidence event coordinates in declared unobservable or uncalibrated sections.
- Account for 100% of fibre length as observable, degraded, unobserved, unusable or unavailable.
- Bind results to fibre revision, interrogator, gauge length, sampling, optical path, clock, filters, raw data and owner permission.
- Operational tests require fibre owner, SCADA/telecom owner, cybersecurity, change approval and non-interference review.
- DAS alerts never operate protection, isolation, dispatch, excavation or repair.

## Proposed research targets (not achieved)

- Reproduce both published baselines before improvement claims.
- Reject all seeded wrong-route, stale-calibration, timebase and invalid-interrogator cases in the frozen finite corpus.
- Zero high-confidence geographic placements for ambiguous/unobservable route segments.
- 95% interval coverage on held-out profiles after calibration.
- Report false alarms per fibre-km-day and blind length explicitly.
- P95 cross-track error below 0.5 m is a stretch goal only after a matched reproduction and truth survey.
- Tamper/excavation detection targets remain unset until a separate, independently labelled corpus exists.

## Luminous boundaries

Symtropy #4972 owns DAS replay, coupling and observability qualification. Symtropy #4925 owns mobile robot localization; #4953 owns mobile in-pipe acoustic arrays; #4913 owns duct rigs and robot recovery. Sol-Atlas #755 owns spatial route evidence and disclosure. Symthaea may prioritize follow-up measurements but not dispatch. Mycelix preserves owner approvals and accepted-work evidence but has no electrical protection authority.

## Claim ceiling

This is a literature-backed protocol and synthetic campaign, not a Luminous DAS implementation, field reproduction, fibre compatibility assessment, theft detector, operational SCADA integration, cable protection function, survey certification or engineering approval.