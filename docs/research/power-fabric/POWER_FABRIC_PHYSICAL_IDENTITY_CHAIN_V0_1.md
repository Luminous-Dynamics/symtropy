# Power Fabric physical identity chain and installation reconciliation v0.1

**Status:** research contract and synthetic qualification corpus; no Rust implementation or physical installation qualification claimed  
**Date:** 2026-11-15  
**Owner:** #5302  
**Parent build receipt:** PR #5262 at base head dcd3a188d104a18b22b9209988baa70ccfbb73d1  
**Composes:** #5245, #5218, #5282, #5283, #5171, #5234 and #5241

## 1. Purpose

A reproducible design build is not yet a reproducible account of a physical installation.

The current build receipt can bind a validated source IR, mapping policy, compiled ServiceGraph, equipment/configuration identities declared in that graph, compiler and toolchain inputs, standards profiles and upstream artifacts. It can bind the bytes of a generated diagram or guide to that build.

It cannot by itself establish that the named equipment was delivered, that a particular serialised unit is the unit shown in the model, that it was installed in the declared position, or that a later inspection reconciled the physical installation with the design.

This contract defines the missing identity bridge. It deliberately composes the existing review-attestation, evidence-integrity and invalidation work rather than inventing duplicate trust systems.

## 2. Core model: four identities, not one asset row

Power Fabric must keep these identities distinct:

| Identity | Question answered | Example |
|---|---|---|
| Design slot | What equipment is intended at this logical graph position? | inverter slot INV-01, required module contract |
| Product model | What product family/model/revision is specified or supplied? | manufacturer + catalog number + catalog revision |
| Physical instance | Which individual manufactured unit is this? | issuer namespace + serial/asset identifier + hardware revision |
| Installed assignment | Which physical instance was assigned to which design slot and location, and when? | instance A installed in INV-01 at cabinet east |

A product model is not a physical instance. A serial label is not proof of product authenticity. A device-authenticated identity is not proof of wiring or topology. An installation record is not an engineering review or statutory approval.

The design slot is stable across replacement. Physical instances are not. An assignment is a time-bounded relationship between them and is never overwritten.

## 3. Event-sourced lifecycle

The input is an append-only event set. The current or historical installation view is a deterministic projection at an explicit as-known-at cutoff.

Minimum event families:

- design_slot_created;
- product_ordered / unit_delivered;
- identity_observed;
- unit_installed;
- unit_removed;
- unit_replaced;
- unit_relocated;
- configuration_changed;
- inspection_recorded;
- independent_reconciliation_recorded;
- conflict_declared.

Every event binds:
- stable event ID and schema version;
- installation ID and revision;
- exact design-slot ID;
- physical-instance ID when the event concerns a physical unit;
- prior and resulting instance/assignment references for replacement or relocation;
- event/phenomenon time and record/ingestion time as distinct fields;
- actor reference and typed actor role;
- identity-resolution method;
- exact evidence references, each resolving to a content commitment/source receipt;
- predecessor event IDs or an explicit concurrency/conflict state;
- source build/graph revision when the event is derived from a design snapshot;
- review-attestation reference where independent reconciliation is claimed.

The event payload is immutable. Corrections append superseding/correcting events with explicit links; they do not rewrite old events.

Event time alone is not trusted time. Evidence admission and historical cutoff rules are consumed from #5171. A trusted timestamp token may establish that a digest existed at a time, but it does not prove the event description is true; any timestamp service is part of the separate evidence trust policy (RFC 3161 is a relevant protocol reference).

## 4. Identity-resolution strength

Identity methods must not be treated as interchangeable. v0.1 uses an explicit ladder:

1. **Declared:** a designer or system names an expected product or slot. No physical instance has been established.
2. **Supplier record:** a supplier/delivery record names a product and, where supplied, a serial number. This establishes the contents of that record, not physical presence or authenticity.
3. **Label observation:** a person or scanner records a visible barcode/QR/serial label. This establishes a label observation, not device authenticity or installation.
4. **Device-authenticated identity:** a device or secure element produces an identity assertion verified under an explicit vendor/root policy. This may strengthen instance identity but does not prove physical connection, correct topology or correct function.
5. **Qualified physical reconciliation:** a competent person inspects the physical unit and installation context, reconciles instance identity, location and scope against the design snapshot, and records methods, exclusions and evidence.
6. **Independent review disposition:** a separately validated attestation from #5282 identifies the exact reviewed snapshot, reviewer authority, scope, independence, limitations and expiry/revocation state.

The identity ladder is not a universal numerical score. A higher method does not automatically establish every proposition. The system records which proposition each piece of evidence supports.

## 5. State projection

The installation projection returns one explicit state per design slot:

- declared_only: design slot exists; no delivered or installed physical instance is established;
- delivered_unreconciled: delivery is evidenced, but the individual unit is not reconciled to the installed position;
- installed_unreconciled: an installation assignment is recorded, but required identity/inspection/review evidence is missing, stale, contradictory or outside scope;
- reconciled: all required identity, assignment, evidence and review predicates for the declared scope are satisfied at the selected cutoff;
- conflicted: multiple incompatible assignments, identity ambiguity, a wrong product/revision, causal conflict or irreconcilable evidence exists;
- unknown: required inputs or dependency inventory are incomplete.

A missing event is not evidence that no change occurred. A missing dependency is not an empty dependency set. Neither may produce a reconciled state.

The projection reports claim-level dispositions as well as the slot summary. A single aggregate status must not hide a particular unresolved connection, revision or scope.

## 6. State-transition invariants

1. A design slot may exist without a physical instance.
2. A product catalogue match may satisfy a model-level requirement but cannot identify an individual unit.
3. A delivery record cannot create an installed assignment.
4. A label observation cannot by itself establish authenticity, installed position or configuration.
5. A device identity assertion cannot establish electrical connection or topology.
6. An installed assignment without the required inspection/review remains installed_unreconciled.
7. A physical instance may not have two simultaneously active assignments to incompatible slots.
8. A design slot may not have multiple active physical instances unless its versioned module contract expressly permits a quantity or parallel-unit set.
9. Replacement is a remove-old/add-new transition. The old instance, old assignment, old evidence and old output receipts remain addressable.
10. Relocation changes location-dependent claims and outputs, but does not mutate intrinsic product history.
11. Hardware revision or configuration change creates a new configuration identity and requalifies all claims depending on those properties.
12. Conflicting concurrent events remain conflicted; event input order is never a resolution rule.
13. A late-arriving record may change a later projection but cannot leak into a historical as-known-at replay.
14. A caller-provided maturity enum, status string, hash or QR code cannot confer reviewer authority or physical reconciliation.

## 7. Change-impact composition

The identity chain does not implement a new dependency engine. It emits stable semantic dependency IDs and current fingerprints for the existing Power Fabric change-impact graph (#5218).

Example dependency classes:

- installation identity and revision;
- design-slot identity and contract digest;
- product model/catalogue revision;
- physical-instance namespace and identifier commitment;
- hardware revision and configuration digest;
- active slot-to-instance assignment;
- location and physical connection observations;
- inspection/reconciliation evidence;
- reviewer attestation and trust-policy snapshot;
- standards/jurisdiction applicability profile;
- compiler, adapter, build lock and renderer inputs.

The following output effects are expected only where the relevant output declares the corresponding dependency:

| Mutation | Expected affected outputs | Potentially retained outputs |
|---|---|---|
| Physical unit replaced | slot compatibility, source map, single-line projection, circuit directory, resident/emergency views, commissioning dossier | unrelated circuits and independent reports |
| Unit relocated | location/topology views, source map, circuit directory, access/emergency views | intrinsic product identity/catalogue facts |
| Hardware revision/configuration changed | compatibility, constraints, commissioning, every view that presents those attributes | unrelated units and location-independent product history |
| Product catalogue revision changed | model compatibility and all outputs relying on the catalogue facts | outputs with no dependency on that model |
| Reviewer attestation expires/revokes | review-dependent claims and projections | raw observations and immutable historical receipts |
| Standards correction/profile changes | outputs depending on the changed applicability profile | outputs outside that profile's dependency closure |

No output may remain current when its dependency inventory is incomplete. Historical receipts are preserved with their original disposition; current projections expose stale/unknown/conflicted state until reconciliation and rebuild complete.

## 8. Threat model and negative controls

The contract assumes an adversary or ordinary error may:
- duplicate a serial number across records or namespaces;
- reuse a removed asset's former slot assignment;
- substitute a visually similar but incompatible model;
- scan a label from a different device;
- provide a stale supplier catalogue or a forged inventory spreadsheet;
- omit an equipment/configuration mutation;
- backdate an event or submit evidence after a decision cutoff;
- reorder conflicting events to manipulate a first-write-wins reducer;
- reuse an old reviewer attestation for a new graph or equipment revision;
- claim that a QR code, hash, device response or software state proves physical installation or isolation.

Required controls:
- namespace every physical identifier (issuer/manufacturer/asset namespace + identifier);
- retain exact source evidence commitments, not only human-readable identifiers;
- keep event time separate from receipt time and evaluate both at the decision cutoff;
- require the authority verifier from #5282 for review dispositions;
- require a physical reconciliation method and explicit scope for installed-state claims;
- fail closed on missing, stale, ambiguous, conflicting or out-of-scope dependencies;
- do not equate model matching with physical identity matching;
- make event reduction deterministic under permutation of the same causally equivalent event set;
- make conflict explicit when causality is insufficient to order incompatible transitions.

Public fixtures must use synthetic product identifiers and serials only. Real customer serials, addresses, device credentials or site evidence must not be committed to public test data.

## 9. Frozen P0 scenarios and qualification targets

The machine-readable fixture in POWER_FABRIC_PHYSICAL_IDENTITY_CHAIN_FIXTURE_0_1.json contains the initial synthetic scenarios. Expected outcomes include design-only, supplier delivery, label-only identity observation, unreviewed installation, reconciled installation, wrong-model substitution, replacement, relocation, duplicate active assignment and late-arriving reconciliation evidence.

P0 targets (targets, not results):

- 100% expected disposition on the frozen scenario corpus;
- zero false physical-instance matches in the negative-control corpus;
- zero reconciled installations from design-only, catalogue-only or label-only evidence;
- 100% correct as-known-at cutoff reconstruction under late arrivals;
- 100% invalidation recall against an independently authored dependency oracle;
- zero invalidation outside the declared closure in complete synthetic fixtures;
- 100% deterministic projection for permutations of the same causally equivalent event set;
- 100% rejection or explicit conflict for duplicate/incompatible active assignments;
- semantic equality between full rebuild and incremental rebuild;
- report p50/p95 projection and reconciliation latency at 100, 1,000, 10,000 and 100,000 events; set latency targets only after matched baseline measurement.

Reject-all and accept-all controls must both fail. Report each scenario, mutation family and worst-performing stratum; do not rely only on a pooled accuracy score.

## 10. Relationship to existing systems

- #5245: source IR and electrical graph bridge;
- #5262 / #5258: build receipt and derived artifact provenance;
- #5218 / #5213: dependency-aware invalidation;
- #5283: automated derived output invalidation and regeneration;
- #5282: independent review authority and signed attestations;
- #5171: decision-cutoff evidence integrity, bitemporal admissibility and source-dependency independence;
- #5234: IEC prosumer and South African applicability profiles;
- #5241: matched, reproducible benchmark programme.

Do not fork credential validation, evidence trust, bitemporal freshness, or generic graph invalidation. Add adapters/typed interfaces to the owning contracts.

## 11. Implementation sequence

**P0 — contract:** schema, event semantics, frozen fixture, dependency mutation oracle, threat model and independent review of the contract.

**P1 — Rust projection kernel:** typed IDs, append-only event validation, deterministic as-known-at projection, transition constraints, conflict/unknown outcomes, stable dependency fingerprints, and property/adversarial tests. Integrate in the existing Power Fabric domain module.

**P2 — full/incremental reconstruction:** replay complete history, apply changes, regenerate affected artifacts, compare semantic outputs and preserve immutable receipts.

**P3 — realistic residential prosumer demonstrator:** synthetic grid + PV + battery + inverter + distribution board + critical loads; reconcile design slots to synthetic serialised instances; replace inverter; show exactly which evidence claims and outputs become stale. No live installation or energization.

**P4 — independent practitioner and laboratory review:** only after normative texts, authority scopes, inspection methods and safe test plans are validated. No field or regulatory qualification is implied by earlier phases.

## 12. Claim ceiling

This v0.1 contract is a research specification and synthetic fixture. It does not prove physical presence, product authenticity, topology, configuration, electrical safety, design correctness, standards conformity, reviewer competence, statutory approval or a Certificate of Compliance. A software projection must never authorize switching, isolation or energization.

## References

- IEC 61850-6:2009+AMD1:2018+AMD2:2024 defines the SCL exchange format for IED/device configurations, communication systems, substation function structures and their relationships: https://webstore.iec.ch/en/publication/103863
- IEC 61970-301:2020 defines the CIM base model for power-system objects and relationships: https://webstore.iec.ch/en/publication/62698
- SLSA Build Provenance v1.2 and in-toto provide useful patterns for subject identity, resolved dependencies, predicate types and signed attestations; they are precedents, not proof that an electrical installation is physically correct: https://slsa.dev/spec/v1.2/build-provenance
- RFC 3161 defines a trusted timestamp protocol indicating that a digest existed at a point in time; it does not establish the truth of the underlying event: https://www.rfc-editor.org/rfc/rfc3161.html
- IEC 60364-8-82:2022+AMD1:2026 CSV is relevant to prosumer low-voltage installations, including local sources/storage and grid-connected or islanded modes: https://webstore.iec.ch/en/publication/113148
- ECA(SA) announced SANS 10142-1:2026 Ed 3.03 availability; authoritative normative text and applicability still require professional review: https://ecasa.co.za/technical/notification-of-the-availability-of-sans-10142-12026-ed-3-03/

These references motivate architecture and versioning; they do not provide the detailed rule set or authority to declare an installation compliant.
