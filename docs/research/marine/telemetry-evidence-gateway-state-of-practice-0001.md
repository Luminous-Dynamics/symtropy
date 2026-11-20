# Marine Telemetry Evidence Gateway: State of Practice and Qualification Protocol

**Date:** 20 November 2026  
**Track:** Symtropy #6315 / `marine-telemetry-evidence`

## 1. What the industry already provides

The gateway must not claim to invent onboard networking or marine telemetry interoperability.

- **NMEA 2000 v3.000 (with amendments):** NMEA's current public page describes improvements to interoperability requirements and PGNs across navigation, battery management, engines, alerts, AIS/DSC, electric propulsion, HVAC, lighting and environmental domains. NMEA 2000 is licensed and certification-based. Official reference: https://www.nmea.org/nmea-2000.html
- **IEC 61162-450:2024:** specifies digital interface requirements and test methods for high-speed shipboard Ethernet communication between navigation/radiocommunication equipment and other ship systems. It is a network/interface standard, not a claim that a value is calibrated or physically correct. Official reference: https://webstore.iec.ch/en/publication/72731
- **PredictWind (current product/help material):** describes eight-model route analysis and crew-facing briefings, with comfort variables including RMS roll, vertical acceleration and slamming. Its help documentation says it automatically reruns without the requested comfort limits if no route satisfying them can be calculated. References: https://www.predictwind.com/features/ai-weather-router and https://help.predictwind.com/en/articles/2884622-weather-routing-change-weather-routing-settings

The telemetry gateway therefore does not compete with those products as a route engine, sensor network stack, or dashboard. It qualifies the evidence *before* a measurement updates the vessel's mission state.

## 2. Recent SOTA that raises the planning bar

Two 2026 studies materially raise expectations for the downstream planner:

1. Zhang et al., *Risk-aware ship weather routing with ensemble forecasts on adaptive hexagonal grids*, Ocean Engineering, 15 October 2026, DOI https://doi.org/10.1016/j.oceaneng.2026.127661. The paper combines adaptive H3 routing, traffic-separation topology, edge-level IMO second-generation intact-stability pruning, 50 ECMWF ensemble members and member-coherent CVaR. It reports that optimizing on a failed deterministic forecast can be worse than weather-blind routing. This supports our mandatory weather-blind comparator and member-coherent route risk; it is not a reason to claim those ideas as novel.
2. Gao et al., *Towards spatio-temporal integration: a coordinated optimization method for voyage planning and energy management for all-electric ship with energy storage system*, Journal of Energy Storage, 30 June 2026, DOI https://doi.org/10.1016/j.est.2026.122206. It coordinates voyage and energy scheduling using day-ahead optimization plus intraday DDPG-QP. This establishes that route/energy coupling is already an active baseline; the remaining whole-liveaboard opportunity is service continuity across water, food, waste, spares and people—not merely adding another battery reward.

## 3. Paradigm shift: evidence-qualified state, not mutable telemetry

A device message should be treated as a time-bounded claim with a chain of provenance:

`raw frame → decoder identity → normalized signal + unit → source identity → calibration + uncertainty → freshness/clock checks → failure-domain independence → qualification receipt`

The gateway preserves the source event time, gateway receive time and monotonic ingest/sequence order separately. It refuses to infer that:
- a recently received packet is a recent measurement;
- two devices are independent because they have different source IDs;
- a hash proves the publisher or calibration laboratory is authentic;
- a unit label alone establishes a valid conversion;
- multiple disagreeing values should be averaged;
- an old value remains safe merely because no new value arrived.

The initial implementation is adapter-neutral. It carries no proprietary PGN definitions and does not decode NMEA 2000 or IEC 61162-450 messages. A licensed, qualified maritime integrator must implement and verify the transport adapters against the actual network, device and applicable standard.

## 4. Current executable boundary

The Rust tool currently validates:
- canonical signal-contract, source-registry, observation and batch digests;
- exact raw-frame SHA-256 against captured frame bytes;
- device identity and evidence reference, firmware digest, decoder ID/version/digest, transport, gateway and calibration-chain binding;
- calibration validity at observation time and decision cutoff;
- declared unit, signal range, measurement uncertainty and quality;
- event time, gateway receive time, time-sync evidence, clock uncertainty and observation freshness;
- gateway ingest-index order, per-source sequence order, monotonic event/receive times and reboot/boot-epoch transition receipts;
- a persisted last-accepted sequence watermark from the prior batch;
- exact maximum pairwise-disjoint failure-domain source count, capped at 16 sources per signal;
- source disagreement, with a conflict result rather than majority vote or averaging.

The report is read-only. It returns `QUALIFIED`, `DEGRADED/ABSTAIN`, `CONFLICT/ABSTAIN`, `INVALID`, or `SYNTHETIC_ONLY`; it does not authorize actuation.

The tool emits replay checkpoints (epoch, last sequence, event timestamp and receive timestamp). A trusted integration must persist those checkpoints in its independently protected source registry before the next batch. The validator itself does not own persistent state or authenticate the checkpoint issuer.

## 5. Benchmark ladder

| Test dimension | Research acceptance target |
|---|---|
| Raw/canonical evidence identity | 100% of digest tampering detected |
| Freshness and event time | 100% of stale or future-at-cutoff fixtures excluded |
| Sequence/replay | 100% same-epoch replay and cross-batch replay rejected |
| Reboot continuity | 100% epoch changes require a monotonic ordinal and external transition receipt |
| Source integrity | 100% firmware, decoder, transport, identity and calibration-chain mismatches excluded |
| Units and calibration | 100% wrong-unit, invalid-range and expired-calibration cases unusable |
| Sensor independence | zero shared-declared-domain sources counted as independent witnesses |
| Disagreement | zero conflicting observations averaged or majority-voted |
| Unknown state | zero missing/stale critical values coerced to zero or silently retained |
| Latency/throughput | report p50/p95/p99 and throughput on declared hardware; set deployment limits only after measurement |

The 20 authored Rust tests cover synthetic known answers and negative controls. They are not hardware conformance, calibration, publisher authentication or vessel-safety tests.

## 6. Integration architecture

The next stack boundary is:

1. **Interface adapter:** receives frames from a licensed NMEA 2000 or IEC 61162-450 implementation.
2. **Evidence gateway:** verifies source identity, time, sequence, calibration, decoder and independent-source quorum.
3. **Mission-state estimator:** consumes only a valid qualification receipt; tracks its validity interval and provenance.
4. **Forecast/mission planner:** proposes plans using the qualified state and forecast evidence.
5. **Independent trajectory/service verifier:** rejects any candidate that violates time-indexed service/resource floors or other hard gates.
6. **Actuator gateway:** remains a separate, independently reviewed safety boundary; this project never calls it.

An expired or conflicting telemetry receipt must invalidate only the state claims that depend on it, but those dependent variables become `UNKNOWN` until refreshed or a separately approved degraded-mode policy applies. No last-known-value hold is implicit.

## 7. Limitations and next physical work

- SHA-256 establishes byte identity, not source authenticity, key ownership, calibration-lab authority or physical truth.
- A failure-domain registry is only as complete as its declared dependencies; unmodelled common causes remain unmodelled.
- The exact independence search is limited to 16 contributing source records per signal. Above that limit it returns no usable independent quorum rather than approximating.
- A real adapter requires licensed standards, interoperability/certification test equipment and the actual NMEA/Ethernet gateway/transducer fleet.
- Calibration certificates, secure identity/firmware attestation, clock synchronization and network security require independent provisioning and verification.
- Progression must be: adapter conformance → bench-injected stale/replay/fault testing → hardware-in-the-loop → passive read-only vessel observation → supervised operational evaluation. No autonomous actuation or navigation authority follows from these software tests.

The proposed differentiator is not another marine data bus. It is a reproducible evidence-qualification boundary that makes stale, replayed, uncalibrated, correlated or conflicting sensor inputs explicit before they can alter the liveaboard mission state.
