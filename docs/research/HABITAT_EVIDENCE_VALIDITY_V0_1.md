# Habitat and construction evidence validity contract v0.1

**Status:** synthetic benchmark contract and implementation proposal  
**Owner:** Symtropy evidence-invalidation benchmark, composed with Mycelix construction provenance and Sol-Atlas site-evidence contracts  
**Claim ceiling:** synthetic contract conformance only

## Paradigm shift

Replace “this datum has a timestamp” with “this exact evidence artifact supports this bounded claim, in this scope, under these declared preconditions.” Recency by itself does not prove validity. A survey may be recent yet have incomplete coverage, an incompatible coordinate frame, a revoked source, or a method that does not support the claim being made.

Replace mutable green checkmarks with explicit lifecycle states and dependency-aware reassessment. When a material substitution, terrain change, design revision, changed survey method, or commissioning update occurs, dependent decisions must be re-evaluated. Unrelated evidence should survive. Historical records should be superseded rather than silently rewritten.

Keep unknown, partial coverage, out of scope, stale, revoked, contradictory, and superseded distinct. Unknown is not confirmed absent; a hash is not proof that a source is truthful; a provenance receipt is not professional approval.

## Scope and ownership

- **Symtropy:** deterministic scenario and evidence-invalidation evaluation, with a direct independent oracle.
- **Mycelix:** provenance, chain of custody, signatory role/authority, approval, dissent, revocation, dispute, and accountable cross-organization decisions.
- **Sol-Atlas:** source coverage, coordinate reference system, geometry/scope, spatial projection, and visible unresolved geographic requirements.
- **Symthaea:** hypothesis generation, uncertainty exploration, and selection of potentially valuable observations; generated hypotheses do not become admissible evidence without an independent source and verification path.

This tool does not create a competing GIS, materials passport, ledger, engineering solver, or universal lifecycle ontology. It exercises the contract boundary between the systems.

## Executable synthetic slice

The frozen graph represents terrain evidence, a survey, a concept revision, a material batch, commissioning, and an unrelated furnishing record. Additional negative controls cover a revoked certificate, partial survey coverage, unknown groundwater status, and a scope mismatch.

For each scenario:

1. Bind event IDs, trigger types, changed artifact IDs, and explicit dependency edges.
2. Mark the old changed artifact revision as superseded when it had been valid and digest-consistent.
3. Invalidate only direct dependents whose edge explicitly names the event trigger.
4. Propagate invalidation through derived descendants. Do not infer dependencies from shared labels or physical proximity.
5. Recompute every claim-use disposition against source digest, current status, claim identity, scope, and coverage.
6. Compare the production reverse-graph traversal with a separate repeated-scan reference oracle and frozen expected results.
7. Emit deterministic, sorted results bound to exact manifest bytes and an explicit source revision.

Duplicate delivery of the same event ID and payload is idempotent. Reuse of one event ID with conflicting content is rejected. Input event ordering must not affect the invalidation sets.

## Benchmark gates

These are acceptance targets, not measured real-world performance claims.

| Gate | Acceptance target |
|---|---|
| Oracle agreement | 100% of frozen scenarios agree with the independent reference oracle |
| Hard invalidation controls | 100% of declared dependent evidence is marked stale or superseded as expected |
| False invalidation | Zero unrelated artifacts invalidated in the frozen precision controls |
| Claim integrity | Zero accepted claims with a digest mismatch, revoked/stale/superseded status, incomplete coverage, unknown state, or scope mismatch |
| Event delivery | Duplicate identical events are idempotent; conflicting duplicate IDs fail closed |
| Determinism | Event and irrelevant artifact input-order permutations produce identical normalized dispositions |
| Provenance | Every report binds the raw manifest SHA-256, per-artifact source digests, tool version, and source revision |
| Failure behavior | Missing dependencies, graph cycles, unknown enum values, undeclared fields, duplicate artifact IDs, and invalid schema versions are rejected |

## Next benchmark ladder

**L0 — Synthetic contract (this increment).** Validate graph semantics, digest binding, scope/coverage gates, and independent-oracle agreement.

**L1 — Multi-revision lifecycle replay.** Add explicit correction and supersession history; test event replay, out-of-order delivery, revocation, and scoped invalidation against a frozen temporal oracle. Never erase prior decisions.

**L2 — Circular intervention comparison.** Compose owner-defined physical flow and provenance contracts to compare preserve/repair, reuse, remanufacture, selective deconstruction, material recycling, energy recovery, and replacement. Use a common functional unit and explicit system boundary. Report material recovery by mass and quality/function retained, virgin input avoided, energy, water, greenhouse-gas impact, transport, contamination, residual waste, cost, service life, and uncertainty. Enforce safety and quality requirements as hard gates; expose Pareto trade-offs instead of hiding incompatible outcomes in one weighted score.

**L3 — Spatial resource matching.** Sol-Atlas can project compatible quantities, locations, source coverage, uncertainty, transport paths, and receiving-facility capability. An unmapped or out-of-scope area remains unknown rather than “empty.”

**L4 — Independently measured pilot.** Only after source licensing, material characterization, calibration, safety review, and a measured conventional baseline exist. Report recovery and first-pass reuse yields, cost per accepted functional unit, transport/processing energy, and the distribution of quality after reuse. Synthetic cases and simulation are not physical qualification.

## Research anchors

- ISO 59020:2024, *Circular economy — Measuring and assessing circularity performance*: system boundaries, declared indicators, data collection, and reproducible assessment. https://www.iso.org/standard/80650.html
- NIST, *Building Digitization and Semantic Interoperability*: lifecycle-wide machine-readable semantics for design, operation, and integration of heterogeneous building data. https://www.nist.gov/programs-projects/building-digitization-and-semantic-interoperability
- Costa and Hoolahan (2025), *Eight recommendations to adopt materials passports and accelerate material reuse in construction*: interoperable, stage-specific material passport data and deconstruction planning. https://doi.org/10.1038/s44296-025-00079-3
- KC et al. (2025), *Review of Material Passports and Their Application in Industrialised Construction*. https://doi.org/10.3390/su17125661

These sources motivate interoperability and reproducible metrics; they do not define the synthetic fixture's expected states or establish physical performance. Any real-world deployment needs domain-specific standards and qualified professional review.
