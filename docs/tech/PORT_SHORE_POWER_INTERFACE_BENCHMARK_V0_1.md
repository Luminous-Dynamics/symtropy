# Port Shore-to-Ship Power Interface Benchmark v0.1

**Status:** synthetic contract and verifier seed. Not an electrical design, installation instruction, switching controller, protection system, certification or permission to energize.

## Ownership

Parent: [PORT-SHORE-POWER-INTERFACE-001 #4448](https://github.com/Luminous-Dynamics/symtropy/issues/4448).

- Symtropy #1858 owns port-call/JIT allocation and shore-energy window planning.
- Symtropy #1883 owns staged port-energy investment/dispatch and uptake scenarios.
- Symtropy #4336 owns the S-131 harbour facility source profile.
- Symtropy #4406 owns federated asset/evidence projection and commissioning.
- Symthaea #11028 owns shipboard multi-timescale power quality and transient-envelope models.
- Facility electrical owner #3846 owns actual shore-side electrical design, protection and commissioning.
- Mycelix owns owner-authorized energy reservation/commitment, participant authority and revocation.
- Sol-Atlas #654 owns read-only spatial/temporal facility projection.

This benchmark owns only the compatibility/commissioning evidence contract and synthetic verifier. It does not issue switching commands or control a live connection.

## Paradigm and scope

Separate six propositions: S-131 source-described facility exists; exact ship/berth interface is technically compatible; live grid capacity and power quality are adequate; provider accepted a vessel/berth/connection-specific energy reservation; port/ship protection, earthing, safety loop, interlocks and commissioning evidence are current; and the connection has actually been physically made/energized by accountable authorities.

The v0.1 verifier stops at synthetic ready-for-connection evidence. It explicitly refuses to certify energized state or operate breakers/interlocks. Communications monitoring is separate from independent emergency/safety circuits.

## Standards / regulatory baseline

- IEC/IEEE 80005-3:2025 Edition 1 was published 8 Dec 2025; LVSC scope includes 400–1,000 V AC three-phase systems at 250 A and above, ships requiring up to 1 MVA; inland vessels, HVSC and dry-docking/out-of-service supply are excluded. ISO: https://www.iso.org/standard/72114.html; IEC: https://webstore.iec.ch/en/publication/30632
- IEC/IEEE 80005-1:2019 + AMD1:2022 + AMD2:2023 is the consolidated published HVSC baseline. ISO: https://www.iso.org/standard/64717.html; IEC: https://webstore.iec.ch/en/publication/73907
- IEC/IEEE CD 80005-1 Edition 3 is under development at Committee Draft stage 30.99, approved for registration as DIS on 11 May 2026; it is not published: https://www.iso.org/standard/93914.html
- IEC/IEEE 80005-2:2016 is a published non-emergency monitoring/control data-interface standard under ISO systematic review; the communications path is not the independent safety loop: https://www.iso.org/standard/59133.html
- AFIR Regulation (EU) 2023/1804 Article 9 requires qualifying TEN-T ports to cover at least 90% of relevant annual calls by 31 Dec 2029, subject to vessel-class/call thresholds: https://eur-lex.europa.eu/eli/reg/2023/1804
- FuelEU Maritime Article 6 requires in-scope ships at berth in AFIR Article 9 ports to connect/use OPS from 1 Jan 2030, and at any port developing OPS capacity from 1 Jan 2035, subject to scope/exemptions: https://transport.ec.europa.eu/transport-modes/maritime/decarbonising-maritime-transport-fueleu-maritime/questions-and-answers-regulation-eu-20231805-use-renewable-and-low-carbon-fuels-maritime-transport_en
- EMSA updated shore-side electricity guidance (11 Nov 2025) also discusses battery charging, swapping, power banking and microgeneration: https://www.emsa.europa.eu/electrification/sse.html

## Frozen corpus

The corpus contains 42 unique synthetic cases: 2 valid controls (LVSC and HVSC) and 40 adversarial controls. It covers identity/revision, LV/HV scope, voltage/frequency/phase/current, connector/cable/transformer, earthing/protection/interlocks, communication vs safety-loop separation, grid freshness/capacity, accepted/revoked reservation, power quality, transient envelope, owner approval, draft-standard claims, metering boundary, simulated-evidence promotion and energization claim ceiling.

## Metrics

- Zero false ready-for-connection dispositions on hard identity, compatibility, protection, authority, freshness and capacity negatives.
- Exact ship/berth/connection/facility revision match.
- Zero replayed reservation/readiness transition double applications when event integration is added.
- Explicit Unknown/Blocked/OutOfScope when evidence is missing or unsupported.
- Report installed capacity, technically compatible ship-call coverage, reserved windows, successful connection rate, metered MWh, unserved demand, power-quality excursions and applicable AFIR/FuelEU denominators separately.
- No energy/emissions saving claim without measured energy boundary and comparable port-call baseline.

A contract PASS does not establish IEC/IEEE conformity, physical safety, protection performance, grid service, provider commitment or authorization to energize.