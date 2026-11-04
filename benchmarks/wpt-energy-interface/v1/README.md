# WPT energy-service interface reference profile v1

Status: proposed interoperability reference profile with synthetic examples. It is not a published standard and is not evidence of hardware interoperability or compliance.

## Research motivation

The April 2026 review *Wireless energy conversion in wireless energy internet* (Nature Reviews Electrical Engineering 3, 274–287; DOI 10.1038/s44287-026-00279-6) identifies standardized energy and information interfaces as a missing element for large-scale deployment. It also reviews direct wireless energy conversion into mechanical, thermal and optical loads, primary-controlled architectures and simultaneous power/data transfer.

Existing Symtropy work defines receiver architectures (#3473), cross-laboratory measurement (#3477), source-to-load accounting (#3472), dynamic service (#3493), traffic/power scheduling (#3590), transition energy receipts (#3600) and state-derived storage (#3634). This profile connects those systems at the service-contract boundary.

## Current architecture benchmark: primary-only control

Dimitriadou et al., *Dynamic charging system with reduced power semiconductor device count*, Wireless Power Transfer 13:e031 (published online 30 October 2026), DOI 10.48130/wpt-0026-0023, proposes a single high-frequency inverter supplying multiple track-based resonant couplers, a series reactive power compensator and an AC switching subsystem. Coil selection uses primary-side active-power/coupling signatures rather than a real-time vehicle–roadside communication link. The reported evaluation is ANSYS Maxwell finite-element analysis plus MATLAB/Simulink and PLECS co-simulation—not a demonstrated full-scale roadway hardware trial. Its model assumes one vehicle per converter set, 10 transmitters per set, and reports approximately 11.2 kW simulated output in its in-series operating mode against an 11.5 kW design target.

This is a required architecture/control baseline. It motivates #3640's separation of local physical self-sensing from authenticated identity, authorization and revocation. A coupling or active-power threshold is not an identity credential, and the absence of a live communication link must not disable independent protection.

The source identifies real trade-offs worth including in matched studies: inverter/semiconductor count, litz wire length, ferrite placement, adjacent-coil mutual coupling, output ripple, ZVS/ZCS switching losses, capacitor voltage stress, thermal dissipation and installation cost.

## Paradigm shift

Interoperability is not simply support for a frequency, coil standard or power class. A source and receiver must agree on:
- what useful physical service is requested;
- where the source energy and useful output are measured;
- how much service is needed, by when and under what power trajectory;
- permitted power direction and capability envelope;
- receiver identity, authorization, positioning and command freshness;
- hard protection conditions and safe-state transitions;
- calibrated metering, uncertainty, stored-energy changes and evidence provenance.

## Typed output domains

V1 defines four service types: battery DC energy, mechanical shaft work, process thermal energy and optical radiant energy. They are all expressed in joules, but their boundaries and measurement methods are different. The schema binds each service type to one quantity and one method. A battery-side DC measurement cannot be substituted for useful shaft work or delivered process heat.

The type-profile map defines the relevant measurement boundary and excluded substitutions for each domain.

## Session state machine

The proposed normal sequence is:

    DISCOVERED -> OFFERED -> AUTHENTICATED -> RESERVED
    -> POSITION_CONFIRMED -> ENERGIZED -> RAMPING_DOWN
    -> DEENERGIZED -> VERIFIED

Invalid identity, replay/stale commands, missing calibration, unsafe positioning, failed sensors or unresolved protection must prevent energization. Fast local protection remains independent of network availability. If a system is already energized, it must demonstrate ramp-down and de-energization before terminal abort/verification.

## Event contract

The closed JSON Schema binds each event to:
- event/session identity, monotonic sequence, timestamp and previous-event digest;
- typed service request, power budget, deadline, direction and both energy boundaries;
- source/receiver capability, topology and firmware digests;
- authentication, authorization, freshness, nonce and credential evidence;
- positioning uncertainty and receipt;
- protection/FOD/sensor/hard-limit status and safe action;
- source energy, useful-service energy, auxiliary energy, stored-energy delta, residual, uncertainty and calibration receipts.

The Rust conformance evaluator validates structure, content digests, previous-event hash links, per-session sequence/time continuity, legal state transitions, service type/method, source capability and conditional energization gates. Settlement remains UNRESOLVED when calibrated measurement is missing or the energy closure residual exceeds its declared uncertainty. JSON Schema alone does not enforce these temporal and cross-field semantics.

## Executable conformance

Run the frozen full-session corpus with:

    cargo test --locked -p wpt-energy-interface-validator
    cargo run --locked -p wpt-energy-interface-validator -- benchmarks/wpt-energy-interface/v1/known-truth.jsonl

The corpus contains complete valid sessions for all four end-use profiles, unauthorized/stale/unsafe energization attempts, a service-method mismatch and an unmeasured settlement. Event SHA-256 digests are computed over canonical JSON payloads with the event_digest field omitted; each subsequent event links to the prior event digest.

## Proposed research benchmark targets

- 100% of valid sessions negotiate explicit service type, source boundary and useful-output boundary before energization.
- Zero unsafe false accepts across frozen authorization, replay, stale-command, positioning, FOD, sensor and hard-limit negative controls.
- 100% fail-safe inhibit/abort when mandatory evidence is missing or unresolved.
- At least three independently implemented source/receiver adapters; target >=99.9% valid/invalid conformance with zero unsafe false acceptance.
- Steady/dynamic energy residual goals remain <=0.5% / <=1.0% when the metrology budget justifies them; otherwise publish UNRESOLVED.
- p99 communication/positioning latency is derived from vehicle speed, segment pitch and safety deadlines rather than assigned one universal milliseconds target.
- No hard safety or authorization failure is offset by efficiency, throughput or service score.

Targets are hypotheses, not standards thresholds or demonstrated performance.

## Standards mapping

See standards-coverage.md. This profile is a proposal and does not claim IEC, ISO or SAE compliance. It cannot override existing electrical safety, EMC, exposure or jurisdictional requirements.

## Claim ceiling

Software, schema, protocol and synthetic-conformance work only. No live energy transfer, energized public-road testing, human exposure campaign, cybersecurity certification, measured performance or standards compliance is claimed.
