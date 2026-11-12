# OLI scientific run-receipt contract v0.1

**Status:** draft interoperability contract; planned example only; no production validator or signer is claimed  
**Owner:** Symtropy / OLI shared evidence infrastructure  
**Schema:** `tools/oli-run-receipt/schema/oli-run-receipt-v0.1.schema.json`  
**Example:** `tools/oli-run-receipt/examples/gray-scott-planned-receipt-v0.1.json`

## The architectural shift

OLI should not treat a classifier output, a simulation run, an instrument measurement, an oracle result and a signed document as the same kind of evidence.

The run receipt is a **record of an activity and its provenance**. It captures:
- which protocol and exact input manifest were used;
- which actors, software, instruments and toolchain participated;
- which immutable inputs and outputs were consumed or produced;
- what observation opportunity and false-negative limitations applied;
- what a model predicted, separately from any independent oracle evaluation;
- whether replay was attempted and what equivalence was achieved;
- which parent runs, corrections, supersessions or retractions apply.

It does not decide whether an entity is alive, intelligent, conscious, safe, legally recognized or worthy of authority. A digest establishes byte identity only. A valid signature authenticates an attestation under a verification policy; neither creates scientific truth.

## Why these standards compose well

This is a small OLI-specific envelope designed to map onto existing provenance formats rather than inventing another global provenance ontology.

- **W3C PROV-O / PROV-DM** provide the Entity–Activity–Agent model and derivation, usage and generation relations. The receipt applies those separations to samples, protocols, software, runs, outputs and responsible actors.
- **RO-Crate 1.2** provides a JSON-LD packaging model for research data and workflow context. Process Run RO-Crate adds a practical representation of workflow execution, inputs, outputs, software, instruments, timestamps and agents. OLI can export a completed receipt as a RO-Crate.
- **in-toto Attestation Framework / DSSE** can wrap an authenticated statement and predicate. The OLI receipt may be the predicate or link to one. Signature verification remains a separate result.
- **SLSA provenance** is useful specifically for software build/source provenance: build definition, builder identity, dependencies and artifact digest. It should not be stretched into a generic certificate that a laboratory observation or life inference is true.

References:
- W3C PROV-O: https://www.w3.org/TR/prov-o/
- W3C PROV-DM: https://www.w3.org/TR/prov-dm/
- RO-Crate 1.2: https://www.researchobject.org/ro-crate/1.2/
- Workflow Run RO-Crate: https://www.researchobject.org/workflow-run-crate/
- in-toto Attestation Framework 1.2: https://github.com/in-toto/attestation/tree/v1.2
- SLSA Provenance 1.2: https://slsa.dev/spec/v1.2/provenance

## Three distinct result layers

### 1. Activity facts

What was executed, with which code, toolchain, inputs, instruments, configuration, seed, sample, timing, environment and outputs. The receipt captures raw artifacts and their content hashes when available.

### 2. Model assessments

A model may report a property assessment such as `supported_within_scope`, `insufficient_evidence`, `not_observable_with_current_instrument` or `model_inadequate`. Scores must identify their semantics, calibration and uncertainty. There is deliberately no global life/intelligence score.

### 3. Oracle adjudication

A separate functional oracle, blinded evaluator, independent implementation or human adjudicator can evaluate a prediction. This is recorded in a separate collection with its own evaluator identity, method, software and evidence artifacts.

A semantic validator must reject an assertion of independent evaluation when:
- predictor and oracle evaluator resolve to the same actor identity;
- the oracle method is the same inference path relabelled;
- oracle evidence is absent or derived only from the predictor's own score;
- the ground-truth target or evaluator scope is unspecified.

The JSON Schema expresses structure. These cross-record rules require a semantic validator and are not enforced merely because JSON validation succeeds.

## Replay and correction are first-class

Replay equivalence is not binary:
- `bitwise_identical`: identical outputs under a declared byte-level comparison;
- `numerically_equivalent`: outputs within a declared numerical tolerance;
- `statistically_equivalent`: distributions meet a declared statistical equivalence protocol;
- `not_equivalent`: replay disagrees under the declared comparison;
- `not_assessed`: no equivalence conclusion.

Every result retains toolchain, precision, hardware/accelerator, parallelism, PRNG algorithm, seed and stream definition.

Receipts are append-only records. A correction, superseding run or retraction creates a new receipt that points to the prior receipt and states the rationale. Do not silently overwrite a past scientific result or reuse its identifier.

## Observation and false-negative obligations

The receipt carries observation coverage separately from the result:
- spatial and temporal coverage;
- observation duration, sampling interval and target-process timescale;
- detection limit with units;
- detection probability at reference abundance, with calibration artifact;
- known miss mechanisms (abundance, dormancy, heterogeneity, aliasing, preservation, sensitivity, matrix interference, signature mismatch, unknown);
- unsampled regions and sample hierarchy.

This preserves the distinction between "not detected", "not observable with this instrument", "not tested", "model inadequate" and "absent". A completed receipt must link the coverage record to actual input/measurement artifacts and calibration evidence.

## Hashing, signatures and identity

All content-addressed artifacts use SHA-256 fields. The receipt's canonical JSON digest should be computed using RFC 8785 JSON Canonicalization Scheme before it is referenced by a signed envelope. Key ordering or whitespace must not change the canonical digest.

A follow-on implementation should:
1. validate the JSON Schema;
2. perform semantic cross-record checks;
3. canonicalize with RFC 8785;
4. calculate the receipt digest;
5. optionally produce/verify an in-toto DSSE envelope under a declared trust policy;
6. optionally export an RO-Crate package containing the receipt and referenced artifacts.

The schema does not contain a self-referential digest field. The canonical digest is computed externally and stored in a detached index/attestation.

## Initial scope and non-goals

This increment contains the schema and a planned Gray–Scott example. It does not implement a signer, trusted registry, run orchestrator, universal artifact resolver, RO-Crate exporter, in-toto verifier or SLSA builder. It does not imply that a run is successful or scientifically valid. The example uses null artifact digests because the exact artifacts have not been materialized and hashed.

## Qualification roadmap

1. JSON Schema validation and negative schema fixtures.
2. Semantic Rust validator for identity separation, evidence links, timestamps, correction lineage, calibration links and output/input hash consistency.
3. RFC 8785 canonical digest tests with cross-language golden vectors.
4. Detached attestation envelope with explicit signer and verification policy.
5. RO-Crate exporter and import round-trip.
6. Mycelix adapter for immutable custody, correction, disputes and rights metadata.
7. Symthaea adapter for model assessments and oracle separation.
8. Sol-Atlas adapter for spatial/temporal footprint, sample hierarchy and instrument coverage.
9. Symtropy adapter for simulation configuration, seed/PRNG, solver, numerical precision and output digests.

The first competitive target is not "more signatures". It is **100% provenance completeness and replayability for every admitted OLI benchmark result**, with explicit abstention whenever a material part of the observation or verification chain is missing.
