# Optical ISRU evidence portability profile v0.1

**Status:** typed profile validator and minimal JSON-LD exporter authored; exact-head validation pending  
**Owner:** Symtropy  
**Parent:** #5724 — ISRU-EVIDENCE-INTEROP-001

## Design principle

Do not invent a proprietary laboratory interchange format.

Use RO-Crate 1.3 as the portable research-data package, W3C PROV-O as the provenance relation model, QUDT identifiers for units, and SiLA 2 / OPC UA as candidate operational instrument interfaces. RO-Crate 1.3 publishes a JSON-LD metadata envelope with a root Dataset, data entities and contextual entities. Its profile mechanism explicitly allows domains to constrain expected entity types and mandatory properties while retaining the common interchange format.

References:
- RO-Crate 1.3: https://www.researchobject.org/ro-crate/specification/1.3/
- RO-Crate profiles: https://www.researchobject.org/ro-crate/specification/1.3/profiles.html
- W3C PROV-O: https://www.w3.org/TR/prov-o/
- QUDT units: https://qudt.org/
- ADePT 2026 interoperability perspective: https://www.nature.com/articles/s42004-026-01932-9

## Standards architecture

1. RO-Crate 1.3 is the portable package and metadata envelope.
2. PROV-O represents entities, activities, agents, derivation, usage and attribution across instruments, batches, samples, methods, calibration, observations, assays and decisions.
3. QUDT provides resolvable unit identifiers rather than free-text labels such as "grams" or "percent".
4. SiLA 2 / OPC UA remain candidate equipment orchestration interfaces; this profile does not invent a competing instrument command protocol.
5. The Luminous profile adds only the optical-ISRU-specific types, measurement rules and conformance constraints.

## Provenance roles and required relations

| ISRU concept | PROV-O role |
|---|---|
| Batch, sample, instrument, calibration certificate, raw data, product lot, assay and acceptance receipt | Entity |
| Preparation, calibration, exposure, QCM/witness measurement, capture, purification, assay and decision | Activity |
| Laboratory, operator, software agent and independent authority | Agent |

The profile requires:
- a measurement result wasGeneratedBy its measurement activity;
- the measurement activity used its instrument, sample/product, calibration certificate and applicable method/reference;
- a sample wasDerivedFrom its feedstock batch;
- a product lot wasGeneratedBy a capture/purification activity and wasDerivedFrom source material;
- an assay result wasGeneratedBy an independent assay activity;
- an acceptance receipt wasGeneratedBy an acceptance activity;
- the acceptance activity used the product lot and independent assay; and
- the acceptance activity wasAssociatedWith an authority agent.

The exporter uses the PROV-O direction correctly: used is Activity to Entity; wasGeneratedBy is Entity to Activity; wasAssociatedWith is Activity to Agent; wasDerivedFrom is Entity to Entity.

## Measurement non-substitution rules

The typed measurement claims distinguish QCM areal mass, optical-witness transmittance/reflectance, temperature and gas proxies, captured mass, purified mass and independent product-assay mass.

A QCM reading is local to its crystal. It cannot be exported as optical transmission loss. Optical-witness results do not automatically establish production-optic throughput. Intermediate gas or temperature signals are not captured product. Captured mass is not purified product. Only a qualified independent assay plus an authority-backed acceptance receipt can support accepted product.

Optimizer fantasies are represented explicitly as synthetic fantasy entities, not as physical measurement claims, and cannot participate in the physical evidence provenance graph.

Calibration is bitemporal. A measurement with unknown, expired, superseded, revoked or scope-mismatched calibration fails metrology conformance even when its data file and hash are intact.

Unknown historical values are explicit unknowns with reasons. The profile must not infer missing calibration, custody, instrument identity, method, sample origin or acceptance authority.

## Conformance levels

Conformance is per evidence claim, not one badge inherited by the complete crate.

| Level | Meaning | Required checks |
|---|---|---|
| P0 Package-valid | Typed package is internally well-formed | Stable IDs, profile version, artifact paths/media types/hash shape, RO-Crate root Dataset, unique nodes/claims, valid PROV relation roles, explicit unknowns and fantasy separation |
| P1 Provenance-complete | Evidence graph is connected from crate root | No orphan nodes, required claim entities exist, provenance references resolve |
| P2 Metrology-qualified | Measurement is eligible for engineering evidence | Instrument/sample/calibration/activity links, calibration eligible at measurement time, verification and qualification receipts, valid QUDT units, uncertainty/timestamp checks, modality/claim typing and independent assay chain |
| P3 Replayable | Computational analysis can be independently reconstructed | Protocol, method, software/firmware, stage activity, raw data and configuration artifacts |
| P4 Cross-lab comparable | Comparison supports declared independent-lab use | Laboratory identity, method, reference material and QUDT unit for every measurement |

P0 does not imply provenance completeness. P1 does not imply metrology qualification. P2 does not prove process safety. P3 does not prove source measurement accuracy. P4 supports comparison under declared methods and conditions; it does not assume laboratories are physically equivalent.

## Rust implementation

The standalone reference is research/asteroid-optical-isru-v0.2/portable_evidence.rs. Its regression suite now includes authority-receipt resolution and authority-attribution negative controls.

It implements typed Entity/Activity/Agent nodes, PROV relation-role checks, a provenance connectivity check, typed optical/QCM/proxy/product claims, explicit calibration dispositions, a separate fantasy collection, P0-P4 findings, and a minimal flattened JSON-LD exporter with the published RO-Crate 1.3 context plus PROV-O, QUDT and the Luminous namespace.

It is not a general JSON-LD parser or complete RO-Crate validator. It does not yet parse external crates, perform RDF entailment, verify signatures or recompute file hashes from payload bytes, resolve external URIs, authenticate authorities, or operate SiLA 2 / OPC UA endpoints. Independent RO-Crate and PROV validators must be used before claiming standards conformance.

### Reproduce locally

    rustc --edition=2021 --test research/asteroid-optical-isru-v0.2/portable_evidence.rs -o /tmp/isru-portable-evidence-tests
    /tmp/isru-portable-evidence-tests

Tests are authored and are not considered executed until the dedicated exact-head workflow passes. JSON-LD string-generation tests are not a substitute for an independent RO-Crate validator.

## Interoperability benchmark

The 2026 self-driving-lab review highlights scalability, generalizability and provenance-complete experimentation as shared infrastructure needs. ADePT treats interoperability as a distinct capability dimension. We should benchmark the evidence path itself:

1. Export a complete synthetic campaign crate from batch through assay and product acceptance.
2. Validate it with two independent RO-Crate 1.3 implementations and an independent PROV-O graph checker.
3. Re-import it into a second implementation and compare canonicalized graph semantics and file digests.
4. Run frozen negative controls: missing batch, missing calibration, retrospective revocation, scope mismatch, QCM-to-optical substitution, fantasy-to-observation promotion, proxy-to-product promotion, broken custody, missing method/reference.
5. Reconstruct the analysis in a clean environment and compare exact output digests.
6. Compare two laboratories using held-out batches and material families, reporting uncertainty calibration and residuals rather than schema agreement alone.

Report export/import fidelity, round-trip graph agreement, unresolved-reference rate, provenance completeness, required evidence coverage, share of claims at P0-P4, replay success, time to independent audit, cross-lab measurement bias and uncertainty coverage, and false qualification on frozen adversarial controls (target zero).

## Production integration sequence

1. Replace the prototype exporter with a standards-tested JSON-LD library and RO-Crate 1.3 validator.
2. Publish a persistent versioned profile URI and machine-readable Profile Crate.
3. Pin the QUDT release and allowed unit URI set in every crate manifest.
4. Map Symtropy batch/sample/instrument/calibration/witness/QCM/assay records into PROV-O without mutating historical observations.
5. Bind receipt IDs to Mycelix verification receipts and signed issuer statements.
6. Bind raw file artifacts to independently recomputed digests and custody records.
7. Build independent exporters/importers and cross-lab fixtures.
8. Integrate SiLA 2 / OPC UA at the equipment boundary; never let the optimizer or evidence exporter bypass rig interlocks.
9. Freeze the first evidence-portability benchmark corpus and negative-control manifest before claiming conformance.

The goal is a portable scientific evidence graph that independent laboratories can consume, validate, challenge and reproduce without trusting the original optimizer or repository.