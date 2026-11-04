# WPT energy-service interface reference profile v1

Status: proposed interoperability reference profile with synthetic examples. It is not a published standard and is not evidence of hardware interoperability or compliance.

## Research motivation

The April 2026 review *Wireless energy conversion in wireless energy internet* (Nature Reviews Electrical Engineering 3, 274–287; DOI 10.1038/s44287-026-00279-6) identifies standardized energy and information interfaces as a missing element for large-scale deployment. It also reviews direct wireless energy conversion into mechanical, thermal and optical loads, primary-controlled architectures and simultaneous power/data transfer.

Existing Symtropy work defines receiver architectures (#3473), cross-laboratory measurement (#3477), source-to-load accounting (#3472), dynamic service (#3493), traffic/power scheduling (#3590), transition energy receipts (#3600) and state-derived storage (#3634). This profile connects those systems at the service-contract boundary.

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

The schema validates structure and typed fields. Cross-event ordering, sequence continuity, time monotonicity, conditional energization gates and energy-ledger reconstruction require an independent conformance evaluator; they must not be inferred from JSON Schema alone.

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
