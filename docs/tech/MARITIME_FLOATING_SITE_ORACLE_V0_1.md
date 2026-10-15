# Maritime / Floating-Site Independent Oracle Contract v0.1

**Status:** independent-verifier contract; a standalone Rust implementation and hostile/reference tests are now in the PR. No scenario executor or engineering-admission path is implemented by this document.  
**Parent:** [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612)  
**Scenario data:** [MARITIME_FLOATING_SITE_SCENARIO_V0_1.json](./MARITIME_FLOATING_SITE_SCENARIO_V0_1.json)  
**Scenario boundary:** [MARITIME_FLOATING_SITE_SCENARIO_V0_1.md](./MARITIME_FLOATING_SITE_SCENARIO_V0_1.md)

## Purpose

Specify how a future verifier should independently check the six frozen synthetic fixtures without importing or trusting Symtropy's runtime calculations. This separates three things that are easy to conflate:

1. the fixture's declared inputs;
2. the independently derived expected result for those inputs;
3. any result emitted by an eventual scenario runner.

The fixture's `expected` fields are assertions to verify, not a source of truth for the oracle. The runner must not be allowed to declare its own result correct merely because it repeats the fixture's expected disposition.

This document does **not** admit an execution profile, establish engine capability, validate physical models, or authorize a physical decision. Keep the scenario specification-only until the existing ENG-SYNTH / ENG-SYNTH-CRED prerequisites have the required exact-head qualification and the runtime capability census has been inspected.

## Implemented verifier

The companion implementation is `crates/tools/maritime-fixture-verifier`. It is deliberately tied to schema `0.1.0` / scenario revision 1 and freezes the current six-case corpus. It parses JSON through a duplicate-key-rejecting visitor; checks exact field sets, units, frozen assumptions and case ordering; independently derives each disposition; compares expected values; and reports the SHA-256 digest of the exact input bytes.

Run `cargo test --locked -p maritime-fixture-verifier` and `cargo run --locked -p maritime-fixture-verifier -- docs/tech/MARITIME_FLOATING_SITE_SCENARIO_V0_1.json`. The dedicated `.github/workflows/maritime-fixture-verifier.yml` runs formatting, hostile/reference tests, the frozen-artifact verification, Clippy and clean postflight on the exact PR head.

This implementation does not call the production physics engine and cannot issue model-credibility, physical-safety, certification, external-approval or actuation claims. A local run without `MARITIME_SOURCE_COMMIT` reports `UNBOUND_LOCAL_SOURCE`; it is not a repository evidence receipt. The tool's CI qualification remains whatever the exact-head workflow actually reports—pending or queued is not a pass.

## Independence requirements

An eventual oracle must:

- read the exact frozen fixture artifact as data and independently derive outcomes from each fixture's inputs and the declared contract;
- not import or call `PhysicsWorld`, production scenario reducers, production disposition-selection functions, or the code path being checked;
- not read a runner's claimed output and use that output as its own expected value;
- produce an explicit mismatch when the fixture's declared expected values disagree with its own derivation;
- fail closed on malformed input, duplicate fixture/entity IDs, duplicate object keys, unknown units, non-finite numbers, missing required fields, invalid numeric domains, unknown schema versions, or unsupported model semantics;
- distinguish a verifier defect / malformed artifact from a valid case-level disposition; never turn an oracle failure into a scenario PASS;
- bind every report to the exact source commit, scenario file path and blob/source digest, verifier version, schema version, case ID/revision, and output record;
- use bounded input size and bounded report size; avoid network access or fetching mutable external dependencies during verification;
- emit stable machine-readable output for repeated runs over the same bytes and verifier build.

The actual digest algorithm, canonical-output encoding, disposition vocabulary for malformed artifacts, and oracle implementation language must be frozen before executable qualification. Do not infer these from this draft or silently introduce a competing schema. A Rust implementation is preferred for production integration; an isolated checker may use another language only if its role and runtime dependencies are explicit. No production Python dependency is proposed here.

## Independent derivations for the six frozen fixtures

The oracle derives results from `inputs`, the declared entity constraints and the named contract. It compares its derivation with `expected`; it must not copy `expected.disposition` as its computed result.

| Fixture | Independent derivation | Required comparison |
|---|---|---|
| `baseline-declared-envelope-and-energy` | Compare 80 m ≤ 100 m, 12 m ≤ 18 m and 2.5 m ≤ 3.0 m. Independently calculate 4.0 MWh + 1.5 MWh = 5.5 MWh and compare 6.0 MWh available against that threshold. | `PassDeclaredChecksOnly`; no physical-qualification or approval claim. |
| `vessel-exceeds-berth-length` | Compare the input vessel length 105 m to the referenced berth's 100 m declared length limit. | `Infeasible` for this declared envelope constraint; no silent fallback. |
| `charging-interruption-energy-shortfall` | Compute charged energy = 2.0 MW × 1.0 h × 0.9 = 1.8 MWh; available energy = 3.0 + 1.8 = 4.8 MWh; required energy = 4.0 + 1.5 = 5.5 MWh. | `Infeasible`; report the 0.7 MWh shortfall against the declared threshold if that output field is admitted by the eventual schema. |
| `stale-configuration-currentness` | Compare run configuration identity `config-001` with current declared identity `config-002`. They differ. | `Indeterminate`; the old result cannot be carried forward. |
| `unsupported-nuclear-thermal-hydraulics-request` | Requested model is `NuclearThermalHydraulics`; available model evidence is `None`. | `Unsupported`; no rigid-body approximation or nuclear-safety claim. |
| `floating-site-backup-deficit` | Compute max(0, 1.2 MW − 0.5 MW) = 0.7 MW. | `ServiceDeficit` for instantaneous declared load only; no duration or power-quality inference. |

These calculations are deterministic arithmetic on invented values. They do not predict vessel performance, battery chemistry, hydrodynamics, reactor behavior, grid dynamics, structural response, or site safety.

## Data and arithmetic rules to freeze before implementation

- Interpret each numeric value together with its explicit unit. No implicit unit inference.
- Only the explicitly declared dimensional comparisons and arithmetic in the scenario contract are in scope. No hidden conversion, rounding, clipping, fallback, or physics-engine substitution.
- Preserve enough numeric precision to test the frozen input expressions; define numeric representation and comparison semantics in the executable schema before qualification.
- Reject non-finite values, overflow, invalid efficiency values outside the declared inclusive interval [0, 1], negative values where the field's domain disallows them, unknown units, unresolved entity references, duplicate case IDs, and conflicting revisions.
- A missing or stale configuration identity must not inherit an earlier successful result.
- A model request with absent evidence or an uncensused capability must not be reported as supported.
- Do not use tolerance to make a failing inequality pass. If tolerance is ever required, its source, units, direction and bounds must be explicitly versioned and tested.
- Do not silently normalize unknown fields, schema versions or duplicate JSON keys. The eventual parser policy must reject ambiguity.

The exact output state for malformed fixture data remains to be defined by the parent ENG-SYNTH contract; until then, malformed-artifact handling is a verifier failure, not one of the six case dispositions.

## Hostile and metamorphic cases for the future verifier

These are proposed verifier tests, not additional frozen production fixtures. They may be promoted only after the parent contract freezes the rules and output vocabulary.

1. **Boundary behavior:** length 100.0 m satisfies the declared component-wise limit; any representable value above it fails that constraint. Departure energy exactly 5.5 MWh satisfies the stated threshold; any representable value below it fails.
2. **Single-field mutation:** changing one input changes only the independently derived consequences of that input; unchanged case identity must not conceal a changed source digest.
3. **Configuration mismatch:** equal run/current revisions do not trigger the stale-revision condition; unequal revisions do, and prior outputs remain inadmissible.
4. **Missing and malformed values:** remove a required field; change `MWh` to an unknown unit; encode NaN/Infinity; duplicate an object key; duplicate a case ID; or reference a nonexistent entity. Every variant must fail verification before a PASS can be emitted.
5. **Unsupported physics:** rename or remove the requested model's evidence without supplying a qualified provider. The output must not improve to a supported state.
6. **Oracle independence:** deliberately corrupt each fixture's expected disposition and each derived numeric field in turn. The verifier must detect the mismatch rather than echoing the corrupted expectation.
7. **Result tampering:** change the runner's reported case ID, input digest, source identity, configuration revision, disposition or computed value. Verification must reject the mismatched report.
8. **Repeatability:** identical fixture bytes and verifier build produce byte-identical canonical reports under the frozen encoding. Any canonical encoding requirement must be separately reviewed before being claimed.
9. **Claim-ceiling invariants:** no fixture result implies physical truth, qualified model behavior, external approval, nuclear safety/security, cyber resilience, evacuation adequacy, or authorization to actuate physical equipment.

## Future report fields

Subject to the parent schema, a report should carry at least:

- verifier outcome distinct from case disposition;
- source commit and verifier identity;
- scenario file path, exact blob/source digest and schema version;
- fixture case ID and revision;
- independently derived values and disposition;
- observed runner result, if execution was actually performed;
- per-field comparison outcomes and a bounded mismatch list;
- explicit unsupported / not-assessed / unperformed fields;
- a claim ceiling identical to or narrower than the frozen scenario contract.

Do not emit a PASS-shaped record if verification was skipped, inputs were malformed, a required channel was absent, the output was truncated, or the source/result identities do not match.

## Qualification order

1. Re-resolve the exact current ENG-SYNTH / ENG-SYNTH-CRED / twin dependencies and their qualified heads. A successful check for an older source SHA does not qualify a newer head.
2. Freeze the schema, units, disposition vocabulary, numeric behavior and canonical report encoding.
3. Implement the independent oracle separately from the scenario runner; include the hostile cases above.
4. Test the oracle against intentionally corrupted fixtures and runner reports.
5. Execute only when the exact engine/model/profile capability census admits the requested contract.
6. Bind every run and verification receipt to exact commits and inputs; preserve negative controls and unsupported findings.
7. Have a reviewer check the claim ceiling separately from the arithmetic result.

## Claim ceiling

A future verifier PASS can show only that the frozen contract and arithmetic are internally consistent for the exact synthetic inputs and that reported outputs match an independent derivation under a qualified software contract. It does not establish physical fidelity, engineering suitability, safety, certification, external approval, or permission to operate or control a physical system.
