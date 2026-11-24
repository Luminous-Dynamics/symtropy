# SAFRB read-only pilot evidence request

**Version:** 0.1  
**Research cut:** 24 November 2026  
**Purpose:** define the smallest safe data package for a first operator/terminal/energy-service validation.  
**Mode:** read-only exports or read-only APIs only; no control-system credentials, write access or operational actuation.

## 1. Pilot boundary

Start with one explicitly bounded chain:
- one import/terminal or refinery receipt boundary;
- one named tank or product-storage group;
- one downstream pipeline/transfer segment;
- one inland or local receiving boundary;
- one critical service load, such as backup generation or shore-power demand.

A pilot does not need a complete national fuel dataset. The first objective is to reconcile one physical flow and service boundary end to end. Expand only after every source and measurement boundary is understood.

## 2. Requested source packages

| Package | Minimum fields | Why required | Evidence class / owner |
|---|---|---|---|
| Asset and resource register | Stable asset/resource ID, type, site, product/grade compatibility, capacity unit, installed rating, operable rating, status, owner/operator, validity interval, parent/connected asset IDs. | Defines the physical network and avoids name-only joins. | Engineering/asset register; owner-maintained. |
| Tank stock observations | Tank ID, product grade, observed volume, ullage/working capacity, level, temperature/density correction where applied, observed-at timestamp, valid-from/to, meter/gauge ID, calibration status, uncertainty, quality status, accessibility and source event ID. | Establishes time-aligned measured usable inventory by grade and tank. | Instrument/historian + quality/terminal records. |
| Product quality and batch | Batch/parcel ID, grade/specification, sample ID, laboratory method, sample time/location, results, uncertainty, certificate issuer, quality release/hold status, associated tank/cargo IDs. | Prevents wrong-grade or quarantined product from being counted as usable. | Quality laboratory and receiving/terminal authority. |
| Cargo and vessel milestones | Cargo ID, product/grade, declared and measured quantity, bill-of-lading reference, supplier/trader reference, loading port, vessel reference, nomination, loaded, departure, ETA updates, arrival, berth allocation, discharge start/end, received quantity, survey/measurement record and exception status. | Separates contracts, nominations, in-transit cargo, arrival, discharge and receipt. | Importer, agent, port/terminal source systems. |
| Berth and terminal capacity | Resource/berth/tank/loading-arm IDs, capacity units and denominator, time window, installed and operable capability, maintenance/outage windows, provider offer revision, entitlement/access reference, reservation, accepted commitment and delivered-use record. | Separates physical capacity from access rights and existing allocations. | Operator-owned resource register and Mycelix-qualified authority/commitment receipts. |
| Pipeline and transfer events | Edge/segment ID, grade compatibility, flow/pressure limits, pump station, line-fill, start/end times, measured transfer volume, meter identity, batch/sequence, outage and maintenance state, power availability, destination receipt. | Constrains time-expanded flow and avoids assuming nominal capacity equals available throughput. | Pipeline operator, engineering and historian. |
| Demand and service | Service-zone ID, product or energy carrier, observed delivery/consumption, interval start/end, unit, criticality/service class, forecast vs measured flag, measurement source, substitute/alternative availability and service quality requirement. | Converts fuel movement into explicit service outcomes and demand shortfall. | Customer/operator service records. |
| Generator and conversion | Generator ID, fuel grade, rated power, run hours, measured fuel draws, LHV/assay, efficiency or performance curve, auxiliary loads, operating state, output Wh/kWh, meter and calibration references, maintenance/test-cell record. | Validates fuel-to-useful-electricity conversion independently. | Facility operator, OEM and independent meter/lab records. |
| Grid and shore power | Connection ID, interval import/export energy, max demand/power, outages/curtailment, shore-power connection and availability, tariff/contract reference, meter and calibration. | Adds alternative carriers and external constraints to the service boundary. | Utility, port and facility meters/contracts. |
| Disruption and recovery events | Event ID, affected assets, start/end time, observed impact, actual vs estimated status, source, cause as confirmed/unconfirmed, mitigation, restoration time and evidence references. | Calibrates scenario chronology and recovery without assuming a causal explanation. | Incident logs and operator event reports. |
| Cost, tariff and emissions (optional for first pilot) | Cost item, quantity basis, currency, price/tariff, tax/fee treatment, validity interval, carbon-factor source, system boundary and uncertainty. | Allows economic/carbon comparisons after physical service is reconciled. | Finance, tariff owner, regulator or cited factor source. |

## 3. Required metadata on every exported table/event

Each record should carry, directly or through a stable manifest:
- source system and source record ID;
- issuer/owner and responsible operating boundary;
- original artifact filename/object ID and cryptographic digest;
- schema/version and any transformation applied;
- quantity unit and conversion basis;
- product grade/carrier;
- observation time, event time, publication time and retrieval time as separate fields where relevant;
- timezone and timestamp precision;
- validity interval;
- measurement instrument, calibration status and uncertainty where applicable;
- evidence class: measured, operator-reported, contractual/authority, derived, forecast or synthetic scenario;
- access classification, retention terms and approved purpose.

Do not replace unknown values with zero. Preserve the original record and keep derived/reconciled records linked to their parents.

## 4. Minimum first submission

A bounded first pilot can begin with:
1. 30–90 consecutive days of time-aligned tank level/volume and receipt/dispatch events for one product grade.
2. At least one complete inbound cargo chain where nomination, loading, departure, arrival, berth/discharge and tank receipt milestones are available.
3. One pipeline/transfer segment's operating envelope and dated movement/outage records.
4. One receiving service zone with measured demand and, where present, generator fuel/output measurements.
5. The asset/resource register and a list of data gaps or known source conflicts.

If there are no measured cargo/transfer or generator-output records, the case can still test parsing and structural feasibility, but cannot claim physical validation.

## 5. Security, privacy and operational restrictions

- Use approved read-only exports or read-only API scopes.
- Do not provide PLC/DCS/SIS credentials, actuator endpoints, SSH keys, passwords, private certificates or unrestricted VPN access.
- Do not provide personal information, crew/personnel records or customer-identifying data unless necessary, lawfully authorized and minimized.
- Facility/network topology should be tokenized or generalized where disclosure creates security risk; retain a separately controlled mapping with the operator.
- Do not expose security-sensitive pipeline pressure-control details or facility vulnerabilities in public outputs.
- The pilot must not issue control commands, modify inventory, allocate fuel, make bookings, dispatch vessels, schedule pipeline flow or change generator settings.
- Operator remains the authority for all physical states and actions.

## 6. SAFRB reconciliation and acceptance checks

The initial read-only analysis should return:
- grade-specific inventory by tank and observation time;
- cargo milestone reconstruction with missing and contradictory events visible;
- measured receipt/dispatch mass-balance residuals with meter uncertainty;
- physical vs access/contractual capacity comparison;
- existing allocation and reservation consumption by resource/time window;
- explicit unknown/unverified quantities and their decision impact;
- service-level fuel-to-Wh reconciliation where independent generator data exist;
- a frozen input manifest, source digests, transformation versions and deterministic replay result.

A source is not accepted as measured stock merely because it appears in a dashboard. A cargo is not stock because it is nominated, loaded, in transit or at berth. A delivery is not usable product until receiving, grade/quality and accessibility gates pass.

## 7. Data quality thresholds for pilot admission

- No duplicate source record IDs inside a declared source namespace.
- All quantities carry a unit; all product inventory carries a grade.
- Timestamps must include a timezone/declared local-time mapping and interval semantics.
- Every measurement must identify a sensor/meter or measurement method and its calibration/uncertainty status.
- Every access/commitment claim must link to an issuer/authority receipt and exact resource/time window.
- Every aggregate must be reproducible from identified rows or a documented transformation.
- Source contradictions remain open until resolved by the data owner; the model does not select a winner by confidence score.
- Unknown pipeline, berth, tank or service capacity remains unknown; no interpolation into a real operating limit.

## 8. Pilot outputs and decision questions

Questions a pilot can answer without operational control:
- How much fuel was physically available by grade at each agreed boundary and time?
- Which cargoes or transfers were delayed, missing, duplicated or mis-reconciled?
- Which physical capacities were operable, and which were contractually accessible to the participant?
- What part of a candidate supply path was constrained by physical capacity vs rights/offer/commitment?
- How much useful electricity/service was supported by the measured fuel and conversion evidence?
- Which additional measurement would change a safe decision enough to justify its cost and delay?

The first pilot is successful when the operator can reproduce the reconciled quantities, identify the remaining unknowns and explain every rejected or qualified claim. It is not successful merely because a model produces a plan.

## Claim ceiling

This request defines a safe, bounded input package. It does not imply any specific operator has agreed to provide data, any current South African inventory is known, or that a simulation is physically validated. A physical validation claim requires independent source records, measurement uncertainty, instrument/quality evidence and operator review.
