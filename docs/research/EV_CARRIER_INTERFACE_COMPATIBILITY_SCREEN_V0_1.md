# EV-CARRIER-ICD-001 — exact-revision interface compatibility screen v0.1

**Parent:** [EV-CARRIER-ICD-001 #6308](https://github.com/Luminous-Dynamics/symtropy/issues/6308)  
**Implementation:** `tools/rural-industrial-electrification-evaluator/src/lib.rs`  
**CLI:** `ev_carrier_interface_compatibility_screen`  
**Input:** `fixtures/ev_carrier_interface_contract_v0_1.json`  
**Claim ceiling:** read-only, structural compatibility screen; no production release or physical authority

## 1. Why this increment exists

The carrier programme now has:
- a base EV design and mission envelope;
- incremental pod/aircraft mass, traction-energy and electrical-load screening;
- a module-specific manufacturing process/cell-capacity overlay;
- shared-aircraft regional logistics allocation;
- a defined owner for the physical Interface Control Document.

The missing layer is an executable check that binds a module to the *exact* vehicle configuration and interface revision it claims to mate with. Matching product names or connector labels is not sufficient evidence of physical compatibility.

This first slice is intentionally a contract evaluator. It is not a CAD mating check, structural finite-element analysis, electrical protection study, thermal/fire analysis, network security test, homologation or physical release.

## 2. Exact interface contract

Each port declares:
- globally unique-in-configuration port ID and reciprocal mate-port ID;
- interface family and revision;
- exact mating profile ID;
- provider / consumer / bidirectional direction;
- polarity profile where relevant;
- operating range with units;
- provider capacity limit and module capacity requirement with units;
- whether the module port is required;
- one or more evidence receipts scoped to the exact vehicle configuration, module configuration and interface revision.

Supported families are mechanical structure, HV DC, LV DC, data/control, thermal/fluid, cargo retention, UAV docking, AMR docking, environmental and service.

The v0.1 quantity schema is deliberately generic, but units must agree exactly. It does not perform dimensional conversions or infer that two differently named profiles are equivalent.

### 2.1 Operating envelope

The vehicle port is treated as the provider envelope and the module port as the accepted consumer envelope. The provider's entire declared range must fit inside the consumer's accepted range. For example, a vehicle bus operating at 350–450 V DC fits a module input acceptance of 300–500 V DC; a 250–480 V provider envelope does not.

This is a range-screening rule only. It does not establish transient response, ripple, insulation, fault current, creepage/clearance, thermal rise, protective coordination, HVIL behavior, connector ratings or safety.

### 2.2 Capacity

Where a family needs a load or power capacity, the vehicle declares a capacity limit and the module declares a required capacity in the same unit. The limit must meet or exceed the requirement.

A missing capacity or operating range needed for a release-critical family produces `indeterminate`; it is not zero-filled.

### 2.3 Evidence scope

An interface can only be marked compatible when both mating sides have an evidence receipt with:
- evidence class `independent_test` and result `passed`;
- exact tested vehicle and module configuration IDs plus the physical test-article serials;
- exact interface revision;
- issuer identity, source reference, test-procedure ID/revision and acceptance-criteria ID;
- report, measurement-dataset and equipment-calibration-bundle SHA-256 digests;
- an uncertainty-budget reference and UTC test-completion timestamp;
- explicit limitations (an empty list means none are declared).

A failed independent test bound to the exact pair is a known incompatibility. An inconclusive, stale, revoked, measured-only or synthetic record cannot qualify the interface. Vehicle-side ports can carry multiple receipts because one physical port may have been qualified against several distinct modules. One test record is not silently reused as proof for another configuration pair.

The report carries the selected full evidence receipt for both the vehicle and module side of every interface finding, including receipt ID, issuer, exact test-article serials, procedure revision, source reference, report/measurement/calibration digests, uncertainty reference, completion timestamp, result and limitations. This makes the trace usable by a later evidence-admission workflow.

Every evaluation includes a normalized `evaluation_time_utc`, and every `independent_test` receipt must carry `valid_until_utc`. The evaluator compares these timestamps deterministically: completion after the evaluation instant is rejected; a receipt expiring at or before the evaluation instant is indeterminate; an independent-test receipt with no expiry is invalid. Synthetic fixture records explicitly use no expiry because they cannot qualify a real interface. The evaluation instant is part of the canonical input digest and is returned in the compatibility report.

The screen checks digest syntax, timestamp shape and scope fields. It does **not** retrieve report bytes, recalculate their contents, authenticate signatures, validate calibration certificates or establish issuer authority. Those checks remain a separate evidence-admission responsibility.

## 3. Result semantics

| Result | Meaning |
|---|---|
| `compatible` | Every required module interface matches the exact vehicle interface revision and both sides have exact-scope independent-test evidence. |
| `incompatible` | A declared physical/profile/revision/direction/polarity/range/capacity/target-configuration mismatch is known, or a complete vehicle inventory lacks a required port. |
| `indeterminate` | Qualification is missing/stale/revoked/unknown, a required range or capacity is missing, or the interface inventory is incomplete. |

Known incompatibility takes precedence over missing evidence. A known mismatch cannot be hidden by declaring the rest of the configuration unknown. Conversely, a synthetic or measured-only input cannot become compatible simply because arithmetic checks pass.

The report always sets:
- `physical_authority_granted = false`;
- `actuators_commanded = false`;
- `production_release_granted = false`;
- `evidence_digest_content_verified = false`;
- `source_signatures_authenticated = false`.

A compatible result is a compatibility receipt for an exact declared contract, not permission to energize, mate, unlock, launch, drive, tow or operate equipment.

## 4. Synthetic fixture and negative controls

The committed fixture has one synthetic vehicle interface inventory and four module examples:
1. detachable UAV pod — syntactically complete, matching profiles, but synthetic evidence only → `indeterminate`;
2. integrated drone bay — known HV polarity mismatch → `incompatible`;
3. mobile drone-station trailer — wrong target vehicle revision and mount profile → `incompatible`;
4. AMR handoff — incomplete module interface inventory and unqualified evidence → `indeterminate`.

These are test declarations only. They do not describe a measured carrier, selected connector, qualified mounting pattern, approved electrical envelope, trailer rating or certified AMR dock.

The unit suite also constructs a positive *test-only* case with a passing, exact-scope independent-test receipt to prove the compatibility branch, plus failed and inconclusive negative controls. Those in-memory receipts are synthetic fixtures for the test harness and must not be mistaken for real qualification records.

## 5. Paradigm shift: modularity must be earned through verified interfaces

A universal chassis connector or one standard voltage rail is not automatically optimal. Commonality should be selected only where structural load, voltage/current, thermal rejection, data/control semantics, operating environment and service requirements genuinely overlap.

The design objective is a **portfolio of compatible but bounded modules**:
- a common mechanical datum family where chassis load paths and tolerance stacks support it;
- explicit HV/LV profiles and power budgets with no live-mate assumption;
- authenticated data/control pairing and safe behavior on link loss;
- thermal and fire isolation boundaries that remain module-specific where duty cycles differ;
- independent cargo, UAV, AMR and service interfaces;
- configuration-specific qualification and requalification after interface changes.

This is stronger than maximizing raw part commonality. A common connector that creates a single high-consequence failure mode is not a design improvement.

## 6. Industry and research comparison

A 2026 flexible-EV-assembly process-design paper proposes clustering parts into families by functional, structural and process similarity, then feeding process feasibility back into the design stage. That is a useful research direction for later interface-family and manufacturing co-optimization, but it does not establish that any Luminous module or factory is qualified. DOI: [10.1177/09544070261470943](https://doi.org/10.1177/09544070261470943).

For South Africa, the opportunity is to combine the industrial base with a modular product architecture and qualification evidence. InvestSA describes an established automotive supplier ecosystem of more than 500 component manufacturers, seven global OEMs assembling locally and a technology-neutral SA Automotive Master Plan 2035 objective to deepen local content. Those are sector-level context figures, not a supplier quote or a claim that a low-volume EV carrier automatically qualifies for incentives. https://www.investsa.gov.za/key-sectors/automotive-and-components/.

The dtic's Automotive Investment Scheme describes grant conditions for qualifying productive-asset investment and volume thresholds for some OEM applicants; actual eligibility depends on the current programme rules and applicant structure. https://www.thedtic.gov.za/financial-and-non-financial-support/incentives/automotive-investment-scheme/.

The IEA reports around 440,000 electric trucks produced globally in 2025, with more than 90% of production concentrated in China and only around 3% traded internationally. This reinforces why local manufacturing, service support and product-market adaptation matter more than a global generic cost assumption. https://www.iea.org/reports/global-ev-outlook-2026/manufacturing-and-trade.

These comparisons change our competitive strategy: avoid trying to win by making a bespoke vehicle with every subsystem integrated from day one. Build a controlled platform with qualified module boundaries, local serviceability and verifiable change impact.

## 7. Benchmark ladder

### Contract correctness
- 100% unique port identities and reciprocal mate links.
- 100% explicit interface family, profile, units and revision.
- 100% of release-critical ranges/capacities either populated with evidence or explicitly indeterminate.
- Zero unknown/stale/revoked/synthetic evidence classified as compatible.
- Zero wrong vehicle/module revision pairs classified as compatible.
- Zero known profile, polarity, direction, range or capacity mismatches classified as compatible.
- Byte-identical canonical reports under port/module/evidence input-order permutations.

### Qualification coverage
- 100% of required ports mapped to a test procedure and acceptance criteria.
- 100% of mechanical/HV/thermal/safety-critical changes trigger dependent-test impact analysis.
- 100% exact-configuration receipts bind to test article serials, procedure revision, calibrated equipment, uncertainty and independent approval before production release.
- Zero production releases based solely on this screen.

### Future performance comparisons
- Measure interface qualification cycle time, number of unique port/profile variants, engineering change requalification scope, false-compatible and false-incompatible rates, mate/de-mate cycles, service replacement time, and cross-module reuse.
- Compare common interfaces with purpose-specific interfaces on lifecycle cost, maintenance, production tooling, tolerance robustness and safety-case complexity.
- Do not optimize the number of unique interfaces in isolation. Minimize lifecycle burden subject to physical compatibility, safety and service constraints.

## 8. Next increments

1. Replace illustrative profiles with the owner-approved ICD port table and source-linked drawings/specifications.
2. Add per-family qualification matrices and requirement-to-test links.
3. Bind physical test receipts to serialized test articles, instrument calibration and independent review.
4. Add change-impact propagation: an interface revision invalidates dependent compatibility receipts until requalified.
5. Add tolerance-stack and coordinate-frame evidence for mechanical datums.
6. Add HV protection, thermal/fire, EMC, environmental and cybersecurity safety-case evidence through the existing specialists.
7. Connect module BOM/process identity to Mycelix Mobility Commons #3709 and Battery Passport #6062 without creating a second asset registry or passport compiler.
8. Feed qualified interface/process families into EV factory design #2355/#6193 and carrier operating scenarios #6153/#6219.

## 9. Non-claims

No physical interface has been qualified by the fixture. The output is not a CAD fit check, structural rating, electrical safety finding, thermal/fire approval, EMC result, functional-safety or cybersecurity approval, airworthiness, roadworthiness, homologation, incentive eligibility, supplier quote, plant release or manufacturing authorization.
