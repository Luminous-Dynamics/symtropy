# EV Test Protocol and Metrology Evidence v0.1

**Status:** Rust-first test-record contract and synthetic evidence audit. Not a testing laboratory, conformity assessment, safety determination or certification service.
**Owner:** Symtropy [EV-TEST-PROTOCOL-001 #4524](https://github.com/Luminous-Dynamics/symtropy/issues/4524).
**Related:** Symthaea [pack metrics #11324](https://github.com/Luminous-Dynamics/symthaea/pull/11324), [cell-limited power envelope #11384](https://github.com/Luminous-Dynamics/symthaea/pull/11384); Mycelix [Mobility Commons #3709](https://github.com/Luminous-Dynamics/mycelix/issues/3709).

## Standards are not interchangeable

The contract preserves standard identifier, edition, source URL/digest, recorded status and lifecycle-stage snapshot. It deliberately does not assume a status that can change or silently map a new edition onto old test results.

- IEC 62660-1:2018 is the propulsion-cell performance/life testing reference; it covers capacity, power/energy density, storage life and cycle life. https://webstore.iec.ch/en/publication/28965
- IEC 62660-2:2018 covers propulsion-cell reliability and abuse testing; the declared article level is cell or cell block.
- ISO 12405-4:2018 covers pack/system performance, reliability and electrical functionality; its scope explicitly distinguishes cell testing as IEC 62660. The ISO lifecycle page currently shows the edition as published and under review while also exposing lifecycle events including 90.92 (“to be revised”) and 90.93 (“confirmed”). Preserve the exact source and status snapshot checked rather than encoding a guessed universal current stage. https://www.iso.org/standard/71407.html
- SAE J2464_202108 is a recommended abuse-testing practice and does not itself establish pass/fail criteria; it must not be turned into a synthetic pass/fail badge. SAE J2929 is a separate automotive RESS safety-criteria standard. https://saemobilus.sae.org/standards/j2464_202108-electric-hybrid-electric-vehicle-rechargeable-energy-storage-system-ress-safety-abuse-testing

The applicable standard and article scope are checked together: IEC 62660-1 → cell; IEC 62660-2 → cell/cell block; ISO 12405-4 → pack/system. The 2018 IEC 62660 editions remain explicitly recorded as their own editions. As of 10 November 2026, third-edition drafts are also circulating: prEN IEC 62660-1:2026 (CDV 21/1311) and prEN IEC 62660-2:2026 (CDV 21/1312). Both CDVs were circulated 25 September 2026 and list voting close as 18 December 2026; they remain committee drafts under study, not published standards. The audit allows them to be preserved as draft-protocol references only with qualification disposition NOT_ASSESSED. Draft, under-review, to-be-revised and withdrawn standards cannot support a current qualification disposition. Sources: https://www.evs.ee/en/pren-iec-62660-1-2026 and https://www.evs.ee/en/pren-iec-62660-2-2026. The v0.1 scope mapping is intentionally narrow and must be reviewed if standards or scopes change.

## Evidence chain

The record binds:
1. Test event, purpose, method, operator and actual start/end/execution time.
2. Standard identifier, edition, lifecycle snapshot, clauses, scope, source digest.
3. Specimen identity/configuration, parent pack, chemistry, preconditioning and condition digests.
4. Laboratory facility/scope and accreditation evidence where declared.
5. Each measurement instrument, serial identity, calibration certificate, traceability digest and calibration validity window.
6. Test conditions (temperature, SOC, current profile, pulse duration, rest) and environment.
7. Measurement channels with units, sampling rate, raw measurement digest, uncertainty and instrument reference.
8. Deviations, impact assessment and explicit authority approval; pending/rejected deviations block acceptance.
9. Derived results with calculation method digest, standard uncertainty, coverage factor and result digest.
10. Raw-data digest, report digest, independent review, reviewer scope and signature/attestation digest.

Physical-test records require independent report review and valid calibration at execution time. A synthetic fixture, simulation or vendor claim can be preserved but cannot claim a qualification disposition. A digest establishes byte identity, not factual truth or certification.

## Frozen synthetic corpus

Eighteen cases: eight accepted records and ten rejected controls. Accepted examples include IEC cell performance, ISO pack/system performance, IEC cell-block reliability, SAE J2464 abuse-practice evidence with qualification disposition not_assessed, SAE J2929 safety-criteria evidence, an explicitly non-physical synthetic fixture and two draft IEC 62660 third-edition protocol references preserved without qualification claims. Rejected controls include cell/pack scope substitution, an invented J2464 pass/fail result, expired calibration, unapproved deviation, channel-to-instrument mismatch, missing independent review, missing raw-data digest and two draft IEC 62660 records incorrectly claiming qualification.

These fixtures validate contract logic only. They do not establish any physical battery test result, standard compliance, product safety or qualification.

## Next physical-data milestone

Ingest one actual authorized IEC 62660-1 cell-performance report and one authorized ISO 12405-4 pack/system report as distinct evidence records. Verify measurement-chain and test-article identities against the source documents; do not infer contents from a report title. Then compare Symthaea's derived capacity/power predictions only under matched SOC, temperature, current/pulse profile, rest period, configuration and uncertainty. Keep SAE abuse evidence as a separate evidence purpose and never convert it into an ungrounded software pass/fail status.

## Run

    cargo fmt --manifest-path tools/ev-test-protocol-evidence-audit/Cargo.toml -- --check

    cargo test --manifest-path tools/ev-test-protocol-evidence-audit/Cargo.toml

    cargo run --manifest-path tools/ev-test-protocol-evidence-audit/Cargo.toml -- --schema schemas/transport/ev_test_protocol_evidence_v0_1.schema.json --corpus fixtures/ev_battery/test_protocol_evidence_scenarios_v0_1.json
