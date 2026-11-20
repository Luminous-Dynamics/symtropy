# ZA-UAS-OPS-GATE-001 — South African RPAS/UAS eligibility evidence contract v0.1

**Implementation:** `tools/material-flow-benchmark/src/za_uas_eligibility.rs`  
**CLI:** `mfb-check-za-uas-eligibility`  
**Fixture:** `fixtures/synthetic-za-uas-eligibility-v0_1.json`  
**Schema:** `schemas/za_uas_eligibility_v0_1.schema.json`  
**Programme owner:** [ZA-UAS-OPS-GATE-001 #6330](https://github.com/Luminous-Dynamics/symtropy/issues/6330)

## Purpose and claim boundary

The MFB shared-fleet oracle already treats weather and permission as hard gates. A South African candidate must not become flight-eligible merely because a synthetic scenario says `approved_fresh`. The jurisdiction-specific applicability/evidence package is a separate gate.

This v0.1 screen reports one of:
- `blocked`: the declared aircraft/operation is outside the supported profile or a required approval is revoked, expired, mismatched or out of scope;
- `indeterminate`: required evidence is absent, unknown, stale, not yet effective or otherwise not current;
- `eligible_for_external_operator_review`: the declared evidence package is structurally complete and the caller asserts current scoped evidence.

**The last state is not permission to fly.** The tool always sets `flight_authorized=false`, `dispatch_authorized=false`, and `dispatch_commands_emitted=false`. It neither verifies signatures nor determines the legal meaning of a novel aircraft. The accountable operator and SACAA retain all real approval/dispatch authority. A source URL or hash is not proof of an approval.

## Source snapshot (20 November 2026)

The implementation pins a source snapshot ID in each report. The dated reference set is:

1. SACAA UAS information and activity-class approval matrix: https://www.caa.co.za/industry-information/uas/
2. CAR 101.01.1 applicability (current consolidated display): https://api.acts.co.za/civil-aviation-act-2009/nr425_101_01_1_applicability.php
3. CAR 101.01.2 private operations (substituted by the Thirty-third Amendment, 2026): https://api.acts.co.za/civil-aviation-act-2009/nr425_101_01_2_private_operati.php
4. CAR 101.04.3 UASOC validity/status (substituted by the Thirty-third Amendment, 2026): https://api.acts.co.za/civil-aviation-act-2009/nr425_101_04_3_validity_and_.php
5. CAR 101.05.4 object/substance release or delivery: https://api.acts.co.za/civil-aviation-act-2009/nr425_101_05_4_releasing_objec.php
6. CAR 101.05.11 BVLOS: https://api.acts.co.za/civil-aviation-act-2009/nr425_101_05_11_beyond_visual-.php
7. CAR 101.05.16A multi-UAS operations: https://api.acts.co.za/civil-aviation-act-2009/nr425_101_05_16a_multi-unmanne.php
8. Government Notice 7844, Government Gazette 55226, 21 August 2026 — Thirty-third Amendment: https://www.gov.za/documents/latest?o=3&page=16

The current consolidated applicability display attributes the autonomous-UAS exclusion in 101.01.1(3)(a) to the 26th Amendment (R.3170, GG48228, 17 March 2023). The Thirty-third Amendment in August 2026 changed other Part 101 provisions, including 101.01.2 and 101.04.3. The current 101.04.3(7) text says an operation under Part 101 may only be conducted by a holder of a current UASOC **and OpSpec**. The gate therefore models OpSpec as a separate evidence item, not as an implied attribute of the UASOC. The rules must be tracked by individual regulation and source version; do not assume a later amendment to one provision changed all Part 101 applicability rules.

The SACAA public information also describes RLOS and reserves BRLOS for future use. The model deliberately rejects BRLOS under the v0.1 snapshot. BVLOS is not treated as synonymous with BRLOS; the profile requires the specific Director-approved UASOC Operations Manual scope and SA-CATS 101 evidence.

## Applicability and approval model

The contract distinguishes:
- activity class: commercial, corporate, non-profit, private;
- aircraft control: remotely piloted, autonomous, not managed in real time, unclassified;
- aircraft category: Part 101 Class 1A/1B/other Class 1, Class 2, outside scope, unknown;
- operating visibility: R-VLOS, VLOS, EVLOS, BVLOS, BRLOS, unknown;
- operational features: payload/object release, dangerous goods, multi-UAS, controlled airspace;
- evidence subject: aircraft serial digest, operator identity digest, operation-scope digest and Operations Manual revision where approval is manual-scoped.

The current public SACAA matrix shows the following high-level distinction:
- Commercial operation requires an ASL, UASOC, current OpSpec, RLA, RPL and Certificate of Registration.
- Corporate and non-profit rows show UASOC, current OpSpec, RLA, RPL and Certificate of Registration; they do not show an ASL requirement in that matrix.
- Private operations have narrow exceptions and conditions, including Class 1A/1B and R-VLOS. A private-use exception is not a commercial parcel-delivery pathway.

Additional operation-specific requirements:
- Object release/delivery requires the UASOC holder and Director-approved Operations Manual scope under 101.05.4.
- BVLOS requires Director approval in the UASOC Operations Manual and SA-CATS 101 evidence under 101.05.11.
- Multi-UAS operations require a UASOC, Director approval and the approved RPS/control arrangement under 101.05.16A.
- Controlled-airspace operations require the applicable Director-approved Operations Manual scope.
- Dangerous goods are not qualified by this v0.1 Part 101 screen; they remain indeterminate pending a separate Part 92 applicability/approval gate.

This is an implementation of a declared source snapshot, not a complete legal codification. A novel autonomous aircraft is always blocked pending a written competent-authority determination. That exclusion is not an exemption or an authorization.

## Evidence binding and states

Every approval record binds:
- unique evidence-record ID and source reference/version;
- aircraft serial digest;
- operator identity digest;
- operation-scope digest;
- Operations Manual revision digest when required;
- effective and expiry timestamps;
- upstream adjudication state.

`caller_verified_current` means the caller asserts that the underlying canonical record and authority have been checked. This Rust module does not validate the signature, source authority, legal applicability, revocation channel, or the authenticity of the evidence bytes. It checks the supplied identity/scope/validity structure only.

Mismatched subject/scope, revoked, expired or out-of-scope evidence blocks the declared operation. Missing, unknown, stale or not-yet-effective evidence is indeterminate. No unknown state can be converted to a pass by cost optimization.

## Run

```bash
cargo run --locked --manifest-path tools/material-flow-benchmark/Cargo.toml \
  --bin mfb-check-za-uas-eligibility -- \
  tools/material-flow-benchmark/fixtures/synthetic-za-uas-eligibility-v0_1.json
```

The eight-case fixture covers:
1. synthetic commercial R-VLOS delivery with a complete declared approval package;
2. autonomous commercial operation (blocked);
3. BVLOS missing Director and SA-CATS evidence (indeterminate);
4. private Class 1A R-VLOS non-delivery (external review only);
5. multi-UAS missing Director/RPS approvals (indeterminate);
6. BRLOS (blocked);
7. corporate VLOS without an ASL requirement;
8. stale commercial ASL evidence (indeterminate).

The exact-head workflow validates the schema and fixture, executes the CLI, checks 2 blocked / 3 indeterminate / 3 structurally complete-for-review cases, asserts no flight/dispatch authority, and hashes all inputs/source/report into the MFB evidence artifact.

## Integration boundary

- #6330 owns this South African applicability/evidence gate.
- #6109 owns EV carrier and pod product applicability.
- #6153 and #6177 own economic mode choice and shared-aircraft allocation; candidate aerial capability must be screened before any economic benefit is considered.
- #6083 owns sortie/carrier temporal co-availability.
- #6147 owns cargo identity, loading, release and handoff.
- Mycelix supplies operator/asset identity and evidence custody; it does not authorize flight.
- Sol-Atlas supplies sourced airspace/site/property/geographic context; spatial suitability is not an aviation approval.
- The responsible operator and SACAA retain all actual regulatory and operational authority.

## Next research work

1. Obtain a written SACAA determination for the intended aircraft control architecture, especially autonomous versus remotely piloted operation and the exact delivery/payload-release concept.
2. Confirm the specific Part 101 activity class, aircraft class, line-of-sight/control-link category, airspace and operating-site scope.
3. Obtain the actual approval documents and current Operations Manual/OpSpec revisions from an accountable operator; bind them to the exact aircraft serial, operator and mission scope.
4. Add a Part 92 dangerous-goods gate only after the relevant cargo categories and regulatory responsibilities are mapped.
5. Feed only externally reviewed eligibility and then current operational permission into the logistics optimizer. A complete static approval package is still not a live weather/airspace/mission authorization.

## Non-claims

No result here establishes legal compliance, SACAA approval, aircraft registration, pilot competence, airspace clearance, safe flight, BVLOS permission, dangerous-goods authority, a commercial business case, or permission to operate. All populated fixture evidence is synthetic.
