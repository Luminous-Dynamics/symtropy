# AERO-MASS-PROVENANCE-STATES-001 — Mass evidence and loading states v0.1

**Parent:** [AER-01 whole-aircraft closure #3945](https://github.com/Luminous-Dynamics/symtropy/issues/3945)  
**Work item:** [#3984](https://github.com/Luminous-Dynamics/symtropy/issues/3984)  
**Base arithmetic:** [mass-properties PR #3977](https://github.com/Luminous-Dynamics/symtropy/pull/3977)  
**Mission screen adapter:** [PR #3979](https://github.com/Luminous-Dynamics/symtropy/pull/3979)  
**Status:** research implementation; not flight or certification evidence

## Why this layer exists

The Rust mass ledger in #3977 computes mass, CG and a symmetric inertia tensor correctly for supplied point-mass and rigid-body values. PR #3979 recomputes that ledger at the mission-screen boundary and rejects incomplete or stale configuration identity.

Those components did not yet distinguish an exact aircraft loading state from the base configuration, nor record whether an item value came from a physical measurement, manufacturer data, an approved manual, a derivation, an estimate or a synthetic fixture. A correct calculation over an inapplicable or stale item value is not a correct aircraft result.

This increment adds a separate evidence and loading-state layer which **composes with** `mass_properties`. It does not fork or replace the mass/CG/inertia arithmetic.

## V0.1 implementation

Rust module: `src/mass_evidence.rs`

Types include:
- `MassItemEvidence`: item/category, assembly relationships, evidence class and admission, immutable source reference/revision/locator/digest, aircraft effectivity, source mass/position/inertia and standard uncertainty.
- `MassEvidenceLedger`: exact configuration and aircraft family/model/serial identity plus an ordered evidence generation.
- `AircraftLoadingState`: an immutable dispatch/takeoff/intermediate/landing/diversion/go-around/failure/maintenance snapshot.
- `LoadingItemState`: explicit Installed/Removed/Unknown status; full state mass and geometry for installed items; original source units; per-state value source record.
- `MassPropertiesStateReport`: state/configuration/source-generation digests, the recomputed existing `MassPropertiesReport`, unresolved IDs, and explicit uncertainty status.

The ledger records and loading-state rows must be strictly sorted by item ID and cover one another exactly. Omitted items do not silently disappear. A removed item remains an explicit row. The binary canonical encoding is length-delimited and uses stable big-endian scalar encodings; the state digest binds state kind, aircraft/configuration identity, source generation, ordered item states, source records and source units.

## Evidence and applicability semantics

Evidence classes are separate:
- Directly measured;
- Manufacturer reported;
- Approved manual or type data;
- Derived from measured components;
- Engineering estimate;
- Synthetic fixture;
- Unknown.

Admission states are separate:
- Admitted for screening;
- Pending review;
- Rejected.

Pending, rejected or unknown-class evidence prevents a state from being marked `CompleteForScreening`. This local admission label is not approval by SACAA, FAA, EASA, a design organization or an authorized person.

An item must match the ledger's aircraft family, model and configuration SHA-256. Optional serial applicability is an explicit allow-list. A changed source revision or value changes the source-generation or state identity. Source revisions named `latest` are rejected.

The source SHA-256 identifies the bytes **claimed** by the source record. The Rust layer does not download or authenticate the source bytes. A valid digest does not establish source authenticity, currency, applicability or physical truth. Those require separate acquisition, review and admission.

## Source capture and physical measurement provenance

Each source record now retains a UTC capture timestamp in the strict `YYYY-MM-DDTHH:MM:SSZ` form. Directly measured evidence additionally requires the physical article identifier, instrument reference, calibration certificate reference/revision and calibration-record SHA-256, plus the measurement timestamp. These fields are included in the canonical source-generation and loading-state digests.

The software validates timestamp shape/calendar ranges and calibration digest syntax. It does not retrieve or authenticate source/calibration bytes, verify calibration status with a metrology authority, or establish measurement traceability by itself. A local `AdmittedForScreening` value remains a review assertion, not an approval.

## Units and uncertainty

Canonical computation units are kg, m and kg m². Original source values/units remain next to the canonical values and are checked by conversion:
- mass: kg, g, lb;
- position: m, cm, mm, ft, in;
- component inertia: kg m², g cm², lb ft².

A source-unit mismatch fails closed. Coordinate frames/datum must be declared and consistent by the upstream configuration; v0.1 does not transform arbitrary frames.

Total-mass standard uncertainty is only emitted as RSS when every installed item's mass uncertainty is present and the ledger declares the independence model. If that model is not supported, or an item/state lacks uncertainty, the report says uncertainty was not assessed. v0.1 does **not** propagate uncertainty into CG or inertia; those output fields remain null. A state whose mass/geometry differs from its base evidence must not inherit a false uncertainty claim from the base item; the follow-up patch is to be qualified by tests before use.

## Loading-state behavior

For each state, the module recomputes the existing mass-properties report:
- known mass + known geometry: total mass, CG and inertia can be calculated;
- unknown mass + known geometry: total mass, CG and inertia are withheld;
- known mass + unknown geometry: total mass may be reported, but CG and inertia are withheld;
- unknown installation: the state is indeterminate;
- removed item: excluded from that state but retained in the exact manifest;
- installed aggregate + any included installed child: rejected as double counting.

No approved CG envelope is synthesized. A computed CG is not declared inside an operational envelope unless an applicable, reviewed envelope is supplied through a separate authority-controlled input.

## Independent fixtures

`data/mass_properties/mass_state_fixtures_v0_1.json` has synthetic takeoff, landing fuel-burn, occupant relocation and module-removal cases plus hostile controls for:
- unknown mass;
- unknown location;
- pending evidence;
- stale source generation;
- wrong serial effectivity;
- unit mismatch;
- aggregate/child double counting.

`scripts/verify-mass-state-fixtures.py` independently recomputes total mass, CG and inertia from raw JSON with only Python's standard library. It does not import the Rust production code. Rust unit tests cover the parallel typed implementation. This is a finite synthetic oracle, not independent physical measurement.

## External engineering context

- The FAA Weight & Balance Handbook explains datum, arms, moments and CG calculations. It is a useful educational/engineering reference, not South African approval: https://www.faa.gov/sites/faa.gov/files/2023-09/Weight_Balance_Handbook.pdf
- FAA AC 120-27F is listed as active and provides operator weight-and-balance programme guidance for the specified US operating parts; it is not a design approval pathway for this project: https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentID/1035868
- EASA published CS-23 Amendment 6 and AMC/GM Issue 5 on 15 April 2026 and noted a correction on 5 May 2026. This is a current EASA reference only; the applicable South African certification basis must be agreed with SACAA: https://www.easa.europa.eu/en/document-library/certification-specifications/group/cs-23-normal-utility-aerobatic-and-commuter-aeroplanes
- SACAA's current Engineering Unit page lists separate TGMs for Type Certification, Type Acceptance, Design Organisation Approval, Experimental Certificate of Airworthiness and Amateur-Built Aircraft. The applicable product/applicant route needs an early SACAA discussion: https://www.caa.co.za/industry-information/airworthiness/

The FAA handbook/AC and EASA CS-23 are reference materials. They are not silently treated as SACAA accepted means of compliance.

## What is not yet done

- No actual Wren 2 or Wayfarer 4 POH/AFM loading records, datum geometry, equipment list, serial-specific source package or calibrated weighing evidence has been admitted.
- No aircraft-specific mass, CG, inertia, envelope or loading limit is computed.
- No approved weight-and-balance program, dispatch release or maintenance process is created.
- No trim, control, structural loads, aeroelastic, mission chronology, hybrid dispatch, thermal or flight-envelope model is coupled to the state report.
- No source bytes are fetched or verified by this module.
- No CG/inertia uncertainty propagation or correlated uncertainty model is implemented.
- No physical weighing, metrology, component test, wind-tunnel or flight test is represented.

## Next steps

1. Finish exact-head format, compilation, test, Clippy and independent-oracle qualification.
2. Bind state-specific value evidence and uncertainty to each installed/removed transition; do not reuse base uncertainty when a state has changed.
3. Add explicit source evidence for removal/maintenance states and hostile negative controls.
4. Extend the independent oracle to rigid-body inertia and multi-axis location movement.
5. Bind state report identity into #3979 only after the report's evidence status is complete.
6. Recompute takeoff, intermediate, landing, diversion and go-around states through #3900 dispatch and the broader #3945 closure contract.
7. Acquire exact Wren 2 and Wayfarer 4 controlling data; actual aircraft values remain unknown until that happens.

**Claim ceiling:** This layer qualifies evidence metadata, unit conversion, identity binding, finite loading-state arithmetic and fail-closed controls on synthetic inputs. It does not establish actual aircraft weight, approved balance, stability, handling qualities, airworthiness, certification or flight permission.
