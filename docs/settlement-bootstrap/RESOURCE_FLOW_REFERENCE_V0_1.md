# CRASH-BOOT resource-flow reference corpus v0.1

**Status:** synthetic reference semantics; not an engineering model, life-support design, operational checklist, or survivability claim.

This tranche makes one part of [CRASH-BOOT-00](https://github.com/Luminous-Dynamics/symtropy/issues/1618) independently inspectable: resource accounting under bounded inputs, service-capacity limits, disconnected groups, shared dependencies, conflicting evidence, and unqualified resources.

## Why this is the next slice

A settlement planner for 1 to 30,000+ people must not get credit for multiplying one camp's average demand by a population count. It must separately track (a) resource inventory, (b) per-time demand and inflow, (c) installed versus functioning service capacity, (d) whether services can reach each group, and (e) whether the evidence and model are qualified for the question being asked.

The public corpus is deliberately small and hand-inspectable. Its 30,000-person case is a topology test, not an extrapolation that assumes all services scale linearly:

- machine-readable vectors: `docs/settlement-bootstrap/fixtures/resource_flow_reference_v0.1.json`;
- independent standard-library oracle: `scripts/verify_crash_boot_resource_flow_v0_1.py`;
- focused CI: `.github/workflows/crash-boot-resource-flow-reference.yml`.

The checker validates the fixture and arithmetic contract only. It does not run Symtropy's physics engine, estimate human life-support needs, validate an environmental model, or authorise an action.

## Frozen accounting semantics

For a single resource over one declared time interval, the fixture checks:

```text
demand =
    population × qualified per-person rate
  + fixed system load
  + declared growth/production load
  + declared losses
```

Every quantity is an interval with compatible dimensions. Lower and upper demand bounds are calculated from the corresponding bounds. Qualified inflow is handled separately from demand; neither gross stock nor installed capacity is automatically usable.

The 12-person interval example is **synthetic only**:

- demand: 24–43 abstract resource units/day;
- usable inventory: 120–180 abstract units;
- qualified inflow: 0–8 abstract units/day;
- net depletion envelope: 16–43 units/day;
- arithmetic runway envelope: 120/43 to 180/16 days (about 2.79–11.25).

This is rectangular interval arithmetic, not a probability distribution, percentile, independent-random-variable assumption, or real survival estimate. It is only valid for the bounds stated in the fixture. Correlations, discrete events, changing populations, failures, and time-varying demand require a richer model and must not be hidden inside this example. A separate scale probe checks only the synthetic per-person contribution at 1, 12, 100, 1,000, 10,000 and 30,000 people; it explicitly does not claim fixed loads, service capacity or network topology scale linearly.

The reference capacity gate has 30 units/day available. The minimum supported demand is 24; the maximum is 43. Because the maximum exceeds the capacity, the result is **not certified for the full bound**—not an average-demand PASS.

## Failure modes the oracle freezes

1. **Population and topology:** scale probes are 1, 12, 100, 1,000, 10,000 and 30,000. The scale arithmetic covers only the synthetic per-person contribution, not total system demand, rated capacity, distribution or measured performance.
2. **Local distribution at 1,000:** synthetic total demand and nominal capacity are both 2,000 units/day, yet one disconnected 400-person habitat receives zero. Aggregate capacity must not mask local service failure.
3. **Local distribution at 30,000:** three habitats have total demand of 60,000 units/day and delivered capacity of 60,000. The central 10,000-person habitat is 2,000 units/day short while the south habitat has 2,000 units/day of surplus; its transfer path to the central habitat is unavailable. Therefore total delivery equality still does not establish subgroup coverage.
4. **Unknown environment:** unknown atmosphere and pressure remain UNKNOWN; Earth values are never silently inserted.
5. **Qualification of stock:** quarantined, treatment-unqualified inventory remains visible as gross stock but contributes zero to currently allocatable stock until the necessary release evidence exists. This is an allocation rule, not a statement that the physical material has vanished.
6. **Dimensions:** mass flow and volume flow are incompatible without explicit, qualified conversion evidence.
7. **Common-mode failure:** three pumps sharing one failed power bus are three unavailable components, not three independent backups.
8. **Inventory reconciliation:** a lost transfer acknowledgement cannot cause the same lot to be counted at both source and destination.
9. **Evidence conflict:** stale and contradictory observations stay unresolved without separate adjudication evidence; last-write-wins is forbidden.

## Dependency graph and recovery gates

The JSON fixture also contains a separate `dependency_graph_reference` contract and a `service_evidence_reference` contract. Edges mean **required dependencies**: every declared prerequisite must be available and qualified before a dependent node is counted as available. Alternative groups are explicit: a service is supported only if at least one all-of group has every member in `VERIFIED_AVAILABLE` state.

The six graph cases freeze the following behaviors:

1. A failed power bus propagates through the water pump, water loop, and clinic-hygiene service.
2. Two pumps on the same failed source do not count as independent redundancy.
3. A separately qualified manual fallback can remain available while the powered pump is lost.
4. A repair report does not restore service while the repaired controller remains `REPORTED_REPAIRED_UNVERIFIED`; dependent services remain unavailable pending requalification.
5. A cyclic dependency graph is rejected instead of guessing a fixed point.
6. A reference to an undeclared dependency is rejected, with the missing identifier retained for diagnostics.

This is a deliberately small graph-logic oracle, not a complete reliability model, fault-tree analysis, probabilistic risk assessment or diagnosis engine. It assumes the graph and qualification states have already been supplied by an authoritative evidence process; this code does not authenticate physical sensors, prove that a manual fallback exists, estimate failure rates, validate redundancy independence, or direct repairs. A future implementation needs typed node/edge identities, evidence lineage and freshness, units/capacity, time-dependent transitions, cycle detection, common-cause groups, and explicit repair/requalification receipts.

## Local artifact-digest verification canary

The fixture includes a SHA-256 known-answer canary with literal UTF-8 payload bytes and a pinned expected digest, plus a one-measurement mutation control. A second payload pins a synthetic configuration digest. This checks the local hashing implementation and a tamper-detection negative control.

One service-evidence record is also **content-bound**: its `artifact_payload_utf8` field holds a canonical, sorted-key compact JSON claim object; the verifier recomputes SHA-256 against `artifact_digest`, checks the fixture-local canonical byte representation, and compares every bound field to the parsed payload. Negative controls reject a changed claim with unchanged bytes, changed bytes with an unchanged digest, and noncanonical bytes even when their new digest is correct. The remaining evidence cases still use syntax-shaped digests to isolate other field semantics. The canonical JSON rule here is a narrow fixture contract, not a claim of conformance to an interoperable JSON canonicalization standard. This is not a general authenticated artifact registry.

Neither the canary nor the content-bound vector authenticates issuer identity, validates a digital signature, fetches an external artifact, establishes trusted time, proves calibration status, or shows that the evidence describes a physical asset. There is no production trust root or complete signature/authentication scheme.

## Canonical JSON boundary and interoperability gate

The content-bound example now spells its measurement as the integer token `1`, rather than `1.0`, so the pinned ASCII-key claim bytes are consistent under the Python fixture serializer and ECMAScript JSON number serialization. The Python oracle enforces its compact sorted-key representation. A second, dependency-free Node.js test helper (`scripts/verify_crash_boot_payload_vectors.mjs`) independently checks the artifact digest, bound fields, byte cap and hostile-input controls; it also exercises three JCS known-answer vectors for ECMAScript number serialization, UTF-16 property ordering and negative zero. Regression assertions reject whitespace changes even after re-hashing, duplicate property names, non-finite numeric tokens, negative-zero spelling, a payload string that cannot be encoded as UTF-8, malformed syntax, overflow/depth parsing failures and over-limit claim envelopes.

The Node.js helper uses ECMAScript serialization primitives and UTF-16 sorting for these known-answer tests, but **neither the narrow Python rule nor this limited cross-check is represented as full RFC 8785 (JCS) conformance or production authentication**. A separately reviewed interoperable evidence profile with the full applicable conformance corpus remains necessary. The Python standard library documents that `json.loads` normally retains only the last occurrence of a repeated object name and accepts `NaN`/`Infinity` through its default constant parser; Python 3.11 also added a configurable limit for integer-string conversion. The verifier therefore cannot rely on the default decoder alone to enforce a portable evidence-byte contract ([Python `json` documentation](https://docs.python.org/3/library/json.html), [Python `sys` documentation](https://docs.python.org/3/library/sys.html#sys.get_int_max_str_digits)).

[RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html) also specifies ECMAScript-compatible primitive serialization and sorts object member names by UTF-16 code units, not simply the host language's default string ordering. It constrains input to the I-JSON subset, including no duplicate member names and IEEE-754-compatible numbers, and forbids NaN/Infinity. Equivalent key/value objects may otherwise serialize differently between languages and produce different signature/hash bytes. See [Section 3.1](https://www.rfc-editor.org/rfc/rfc8785.html#section-3.1), [Sections 3.2.2–3.2.3](https://www.rfc-editor.org/rfc/rfc8785.html#section-3.2.2), and the [verified errata on negative zero](https://www.rfc-editor.org/errata/rfc8785).

Before evidence payloads are exchanged or signed across Rust/Python/JavaScript implementations, adopt a separately reviewed canonicalization profile and run interoperable known-answer vectors—including numeric edge cases, escaped strings, Unicode property ordering, duplicate members, non-finite numbers and negative zero—using an implementation tested against the profile's conformance corpus. Do not promote this fixture helper into a production trust component by renaming it or by relying on this narrow regression set.

The nine current negative controls are declarative entries in `service_evidence_reference.artifact_binding_rule.canonicalization_negative_controls`. Each entry pins an ID, mutation operation, whether the altered payload digest is recomputed, and the expected rejection status. Besides serialization ambiguities, the set includes malformed syntax, an exponent that overflows finite numeric representation, deeply nested JSON, and a claim payload larger than the explicitly pinned 65,536-byte local envelope limit. The size cap is for the embedded claim envelope only—not for separate evidence artifacts—and must be re-evaluated for any new evidence profile. The oracle converts parser value/depth errors into rejection outcomes rather than uncaught exceptions and rejects over-limit payloads before hashing/parsing. It pins the complete ID/mutation/digest-policy/verdict mapping before replaying them so a fixture edit cannot quietly weaken expected outcomes. This metadata makes the cases portable for future independent checkers without claiming that the checker itself implements JCS.

## Service evidence and repair qualification reference

The `service_evidence_reference` section of the JSON corpus adds twenty-one synthetic cases for evidence-currentness and service-state admission. The checker applies a declared evaluation time and maximum age and tests:

- fresh, in-scope evidence with an eligible method, valid calibration, compatible unit and an in-range measurement;
- stale evidence, a future observation timestamp, revoked/superseded evidence and malformed digest syntax;
- mismatched asset revision or configuration, missing method/calibration context, expired calibration, incompatible units and an out-of-criterion result;
- a reported repair without a post-repair test; a passing-looking post-repair record bound to the wrong asset revision; a current-revision post-repair receipt that qualifies under the local fixture rule; and a malformed configuration digest;
- one content-bound record and negative controls for a claim mutation that is not reflected in the bytes and a payload mutation that is not reflected in the digest;
- multiple current observations that disagree, including reversal of record order to establish that the verdict is not last-write-wins.

The result `VERIFIED_AVAILABLE_CANDIDATE` is intentionally named as a *candidate*. It means only that the fixture's local rule checks accepted the record fields. For the one content-bound case, the checker also confirms that a local payload's UTF-8 bytes match the pinned digest and that its parsed claim fields equal the record fields. The other evidence cases use syntax-shaped digest values to isolate field semantics.

This does not authenticate an issuer, validate a signature, fetch external evidence, establish trusted time, prove calibration status, or confirm that a physical device satisfies an acceptance criterion. A production contract still needs independently pinned artifact bytes, issuer authorization/revocation, trusted chronology, schema/toolchain pins, context-of-use criteria and protected evidence storage.

Timestamp handling is explicit: an observation in the future is rejected, age is measured against the declared evaluation time, and expired validity intervals are not considered current. These fixtures do not define freshness windows for real pressure vessels, water treatment systems, crew medical records or other assets; those policies must be justified per evidence type and hazard.

A `POST_REPAIR_TEST_PASSED` record must include a repair receipt ID, repair-attempt ID, repaired asset revision, repair configuration digest, post-repair test ID and result, and the asset revision tested. All revision/configuration bindings must match the current target, and the test result must be `PASS`. A status flag alone does not restore service. The current-revision receipt outcome remains a local synthetic contract result: no actual repair, instrument, signature, or physical measurement is verified.

For conflicting sources, the synthetic contract returns `UNKNOWN_CONFLICTED` rather than resolving by arrival order. This is intentionally conservative; real evidence reconciliation requires source-specific trust, independence and measurement-uncertainty policies and cannot be inferred from this small checker.

## Research implications

The contract follows the structure of established human-spaceflight systems engineering without pretending their values transfer to an unknown planet:

- NASA's [ECLSS overview](https://www.nasa.gov/reference/environmental-control-and-life-support-systems-eclss/) treats air revitalization, oxygen generation, water recovery, waste, ventilation, and fire response as distinct but coupled subsystems.
- NASA's [ISRU overview](https://www.nasa.gov/overview-in-situ-resource-utilization/) supports keeping deposit detection, accessibility, extraction, processing and usable output distinct.
- NASA's active [Standard for Models and Simulations, NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009) and [2026 implementation handbook](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009) emphasize model credibility for a declared use, including evidence and uncertainty. Reproducible arithmetic by itself is not model validation.
- NASA's [Spaceflight Human System Standard, Volume 2](https://www.nasa.gov/reference/nasa-std-3001v2/) (July 15, 2026 edition) is a relevant reference for crew interfaces, habitability and environmental health in designed spaceflight systems—not a complete standard for an arbitrary extraterrestrial environment.
- NASA's [planetary-protection overview](https://sma.nasa.gov/sma-disciplines/planetary-protection/) motivates retaining unknown biological status and contamination pathways as explicit constraints.

These references establish subsystem boundaries and verification expectations, not extraterrestrial deployment readiness, acceptable exposure limits for an unknown world, or numerical inputs for this corpus.

For the evidence/provenance layer, three general-purpose standards offer useful *design patterns*, not ready-made certification of physical service records:

- W3C [Verifiable Credentials Data Model v2.0](https://www.w3.org/TR/2025/REC-vc-data-model-2.0-20250515/) separates issuer claims, holders and verifiers, and defines a `credentialStatus` extension point for states such as suspension or revocation. It is a candidate interoperability reference if Mycelix later needs portable signed evidence claims; credential format alone does not make a sensor, issuer or test trustworthy.
- The [in-toto Attestation Framework Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md) binds an attestation to explicit subject artifact digests and a typed predicate. Its broader framework separates predicate, statement, authentication envelope and bundle. This separation is useful for distinguishing what was tested, the claim made about it, and the mechanism authenticating the claim. It is a software-supply-chain design pattern, not a life-support standard.
- W3C [PROV-O](https://www.w3.org/TR/prov-o/) provides a general vocabulary for provenance entities, activities and responsible agents, and is a possible mapping target for cross-project audit events.

Do not adopt any of these standards wholesale without a separate profile defining canonicalization, signature suite, key/issuer authorization, revocation freshness, offline operation, privacy, exact physical scope, and context-of-use policy. PR #1632 adds a content-hash binding example only; the service evidence cases still do not verify issuer signatures or a production trust chain.

## Admission and non-claims

This file set does **not** relax the gates tracked by [WORLD-START product PR #1422](https://github.com/Luminous-Dynamics/symtropy/pull/1422), [its exact-subject qualifier PR #1423](https://github.com/Luminous-Dynamics/symtropy/pull/1423), or [model-suitability issue #1173](https://github.com/Luminous-Dynamics/symtropy/issues/1173). Their exact-subject qualification must be checked independently; an unfinished or queued run is not a pass.

No new production planner, actuation, medical triage, habitat control, resource recommendation, alien-chemistry processing, or command authority is introduced. The next step after this reference corpus is independent review of the input contract and negative controls, then an independently implemented verifier and broader scale/distribution vectors. Domain experts and physical testing remain prerequisites for any eventual real-world use.
