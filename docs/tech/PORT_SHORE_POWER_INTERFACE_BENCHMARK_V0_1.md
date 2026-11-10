# Port Shore-to-Ship Electrical Interface Benchmark v0.1

**Status:** synthetic interface/commissioning contract seed. No live electrical solver, switching controller, protection relay logic, connection procedure or authorization to energize/de-energize equipment.

## Ownership and composition

Parent: [PORT-SHORE-POWER-INTERFACE-001 #4448](https://github.com/Luminous-Dynamics/symtropy/issues/4448).

- Symtropy [MARITIME-JIT-001 #1858](https://github.com/Luminous-Dynamics/symtropy/issues/1858): voyage/arrival/berth/shore-energy window allocation.
- Symtropy [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612): deterministic, effects-disabled replay.
- Symtropy [MARITIME-S131-PROFILE #4336](https://github.com/Luminous-Dynamics/symtropy/issues/4336): S-131 facility and berth descriptions; no live electrical commitment.
- Symtropy [MARITIME-ASSET-TWIN-COMPOSITION #4406](https://github.com/Luminous-Dynamics/symtropy/issues/4406): federated facility/ship/equipment/maintenance source composition.
- Symthaea [MARINE-POWER-TRANSIENT #11028](https://github.com/Luminous-Dynamics/symthaea/issues/11028): vessel-side power transient and power-quality envelopes.
- Facility electrical owner [SMDC-ELECTRICAL #3846](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3846): shore-side electrical system, protection, transfer and commissioning.
- Mycelix [MARITIME-CROSS-STACK-GOLDEN-TRACE #5974](https://github.com/Luminous-Dynamics/mycelix/issues/5974): provider reservation/commitment, issuer authority, custody, idempotency and revocation.
- Sol-Atlas #654: read-only spatial/temporal facility projection.

This benchmark owns only the interface contract, source/authority composition, commissioning-state logic and synthetic adversarial corpus. It does not own shore or vessel electrical system design, switching, interlock hardware, protection coordination, or power-flow models.

## Current standards baseline

### Published LV shore-connection standard

**IEC/IEEE 80005-3:2025**, Edition 1, was published 8 December 2025. Its official IEC product scope covers design, installation and testing of low-voltage shore connection systems on shore and onboard ships, including shore-to-ship interface equipment, transformers/reactors, frequency converters, ship-side systems, protection, control, monitoring, interlocking and power management. Its stated application includes ships up to 1 MVA, three-phase shore systems rated 250 A and above, and nominal 400–1,000 V AC. Its exclusions include inland navigation vessels, HVSC and dry-docking/out-of-service supply.

- ISO product record: https://www.iso.org/standard/72114.html
- IEC product record: https://webstore.iec.ch/en/publication/30632

### Published HV shore-connection baseline

**IEC/IEEE 80005-1:2019 + AMD1:2022 + AMD2:2023** remains the published HVSC baseline. It covers shore and ship distribution, interface equipment, transformers/reactors, frequency converters, control/monitoring/interlocking and power management; the stated practical application is typically ships requiring 1 MVA or more or with HV main supply.

- ISO product record: https://www.iso.org/standard/64717.html
- IEC consolidated publication: https://webstore.iec.ch/en/publication/88253

### Draft maturity

**IEC/IEEE CD 80005-1 Edition 3** is a Committee Draft under development and intended to replace Edition 2. It is not a published standard. A benchmark or product must not describe the draft as current/in-force conformity.

- Draft record: https://www.iso.org/standard/93914.html

Exact normative standards are copyrighted. This benchmark relies on public catalogue metadata and abstracts only. Obtain lawful copies and review flag/port/national requirements, class rules, equipment certificates and owner procedures before any conformity claim.

## Paradigm: from shore-power feature to commissioned connection

Represent shore power as a **stateful, source-owned physical interface contract**, not a facility attribute and not a JIT booking slot.

Keep six independent claims:
1. Installed facility/berth description (S-131).
2. Exact ship-side electrical capability and installed configuration.
3. Shore-side live supply and available capacity at the correct connection point.
4. Compatibility of connector/cable/converter/earthing/protection system.
5. Actual physical connection and owner-authoritative protection/interlock state.
6. Provider-authorized reservation/energy commitment with capacity, time, issuer and revocation.

A reservation cannot create electrical readiness. A feature cannot prove live capacity. Voltage/frequency match cannot prove earthing, protection or interlock readiness. A communication message is not itself a physical protection function. A synthetic simulation state cannot be promoted into a physical energized state.

## Typed contract v0.1

Each interface case binds:
- port, facility, berth and exact shore connection point IDs/revisions;
- vessel ID, configuration revision, electrical installation revision and upstream capability envelope;
- LVSC/HVSC class, standard ID/edition/amendment and maturity state;
- AC/DC class, voltage band, frequency class, phase, current/power rating classes, earthing/isolation arrangement, connector family and cable/handling class;
- transformer/reactor/frequency converter and connection equipment IDs/revisions;
- protection, earth-fault, interlocking and safety-loop evidence IDs, commissioning status and freshness;
- communication profile/endpoint, update freshness and authority role, separate from physical safety function;
- live shore capacity/derating observation, meter boundary and time/uncertainty;
- owner-approved reservation, vessel/berth/connection scope, time interval, capacity class, expiry and revocation;
- vessel load/transient envelope reference and applicability state;
- actual state reports/sensors with source, observation time, retrieval time, owner and conflict behavior.

All fixtures use symbolic synthetic ratings/classes. They are not real equipment specifications or connection instructions.

## Commissioning and readiness state

A read-only evaluator should distinguish:
- described;
- technically compatible;
- capacity available;
- reservation committed;
- connection physically inspected;
- protective/interlock evidence present;
- authorized to connect by the responsible owner;
- energized (owner-reported physical state);
- operating;
- de-energized;
- disconnected;
- closeout complete.

The v0 benchmark's positive disposition is only `INTERFACE_CONTRACT_VALID_SYNTHETIC_ONLY`. It must not report `ready_to_energize` or issue switching permission.

Unknown or contradictory evidence blocks promotion. State transitions require exact source record, issuer/authority, configuration revision, valid time and idempotent event identity.

## Frozen corpus v0.1

The corpus contains 32 deterministic synthetic cases:
- one valid synthetic interface contract;
- missing live capacity, uncommitted/revoked/expired reservations;
- wrong port/berth/connection/vessel/configuration identity;
- LV/HV class, voltage, frequency, phase, current/power and connector incompatibility;
- converter/equipment revision mismatch;
- unknown earthing, missing protection, unknown/bypassed interlocks and stale commissioning;
- stale communications and replayed messages;
- shore/ship breaker and physical sensor contradictions;
- supply derating/outage after reservation;
- vessel transient envelope mismatch;
- time/metering-boundary errors;
- simulation promoted as physical readiness;
- Edition 3 draft falsely reported as published.

The corpus is a test specification/fixture seed. It is not yet an electrical-system validator, standards conformance suite, control function or HIL/PHIL qualification.

## Metrics and comparison

### Synthetic interface/evidence contract
- 100% independent-oracle agreement on the frozen corpus;
- zero false compatibility/readiness promotions for any hard mismatch/Unknown;
- exact vessel/berth/connection-point/facility revision matching;
- zero replay/duplicate events causing duplicate transition/reservation;
- explicit unsupported/Unknown for unrecognized system class or standard maturity.

### Later physical/electrical qualification (separate owner programme)
- voltage/frequency/power-quality deviations against the exact applicable requirements and equipment configuration;
- harmonic/transient response and converter behaviour under declared test profile;
- protection/interlock response, isolation/earthing continuity and fault clearing under approved controlled tests;
- HIL/PHIL results with exact hardware/configuration and qualified test authority.

Synthetic waveforms and fixture states are not electrical certification. No switching or protective action is executed by this benchmark.

### Port/JIT outcomes
- share of berth windows with source-qualified compatible connection evidence;
- reservation-to-physical-readiness reconciliation rate;
- connection readiness misses and root disposition;
- energy-window utilization, berth dwell, idle/anchorage energy and service delays;
- p50/p95/p99 update/commitment latency and stale-state detection;
- resilience under capacity derating, outage, revocation and conflicting observations.

Optimize accepted port service, not energy sold, booked slots or estimated emissions in isolation.

## Claim ceiling

A synthetic PASS establishes only the declared interface/commissioning evidence contract. It does not establish IEC/IEEE conformity, real electrical compatibility, insulation/earthing/protection safety, power quality, safe energization, supplier capacity, reservation commitment, class approval or authority to connect/energize/disconnect. Physical operation remains under the responsible port electrical owner, ship electrical authority and qualified personnel.
