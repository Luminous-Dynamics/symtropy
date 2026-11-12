# Multi-Hop Representation Chain Composition for Scan-to-BIM v0.1

**Issue:** [#4821 SCAN-BIM-CHAIN-COMPOSITION-001](https://github.com/Luminous-Dynamics/symtropy/issues/4821)  
**Parent:** [#4471 Representation Continuity](https://github.com/Luminous-Dynamics/symtropy/issues/4471)  
**Composes:** per-transition continuity audit in `tools/scan-bim-replay-contract/src/continuity.rs`; critical-property retention #4763/#4772; IFC delivery #4271/#4273; survey evidence #1607; component identity #3331.  
**Status:** P0 strict linear-chain contract, two-hop fixture, 40 Rust continuity tests and focused workflow added; exact-head qualification pending.

## 1. Research gap

A sequence of individually valid transformation receipts is not necessarily a valid transformation chain.

For example, a raw scan → surface mesh transition and a surface mesh → spatial graph transition can each pass their local checks while the second transition actually consumes an unrelated mesh, declares the wrong input representation, resets uncertainty, or silently drops a relation from the intermediate representation. Per-transition correctness does not establish compositional correctness.

This programme adds a second level of assurance:

`valid(edge_1) ∧ valid(edge_2) ...` is necessary, but not sufficient.

We must also prove that the edges compose into one connected path with preserved identity, evidence lineage, relation inventory and uncertainty state.

## 2. P0 linear-chain contract

Each transition has an explicit `parent_transition_ids` field. In the v0.1 linear profile:

- The root transition declares an empty parent list.
- Every later transition declares exactly one parent: the immediately preceding transition in the receipt.
- Its `from_representation` equals its parent's `to_representation`.
- It consumes at least one exact parent output artifact with identical artifact ID, SHA-256 and evidence role.
- It cannot skip the immediate parent and treat an older output as a substitute.
- Every entity produced by the parent's correspondence inventory is present in the next transition's source correspondence inventory.
- Every target relation produced by the parent's relation audit is present in the next transition's source relation inventory.
- The next transition's uncertainty `before` state exactly matches the parent's uncertainty `after` state: status, metric ID and numeric bound. Any evidence-conditioned recalibration remains a separate explicit transition update with its own receipts.
- Root, terminal and total transition IDs are included in the audit report.

Earlier raw evidence may remain in later source-artifact lists to preserve claim provenance. It is not a substitute for consuming the immediate parent's output.

### Explicit v0.1 scope boundary

This profile is a strict linear chain. Branches, merges, many-parent transformations, and asynchronous artifact DAGs are not guessed from transition array order or file names. They are outside v0.1 and must remain unresolved until a separately specified DAG profile adds stable chain IDs, explicit parent sets, cycle detection, merge semantics and multi-parent uncertainty/relation reconciliation.

Likewise, this consumer only checks declared receipt semantics. It does not authenticate external artifact bytes, parse IFC, validate geometry, recompute survey covariance, prove checkpoint independence, or verify that a claimed source inventory is physically complete.

## 3. Frozen P0 fixture

The existing rights-safe synthetic continuity fixture now contains both:
- the original single-hop raw-observations → surface-mesh receipt; and
- a two-hop raw-observations → surface-mesh → spatial-graph receipt.

The second hop consumes the exact mesh output from the first, carries forward the 2.5 mm conservative uncertainty bound, preserves the wall-hosts-opening relation, and derives an inferred wall-axis claim from the mesh. The measured wall-axis claim at the mesh remains measured because it retains direct raw scan evidence. The graph-derived claim is not silently promoted to measured.

The focused Rust module has **40 unit tests**, including ten multi-hop additions/updates:
1. accept a connected two-hop chain and report root/terminal IDs;
2. reject a disconnected second hop;
3. reject a representation-kind jump;
4. reject uncertainty reset between hops;
5. reject a missing upstream entity;
6. reject a missing upstream relation;
7. reject reordered transitions;
8. allow original measurement evidence to remain as provenance when the immediate parent is also consumed;
9. reject inferred-to-measured promotion without new direct measurement evidence;
10. reject a later reuse of an artifact ID with changed bytes.

The schema makes `parent_transition_ids` required for every transition, with an empty list on the root. The Rust type is closed and unknown fields remain rejected.

## 4. Benchmarks — report path quality, not just final-file validity

Keep these metrics independent; no weighted score may hide a disconnected transition or hard information loss.

| Track | Measurement | P0 target |
|---|---|---|
| Path connectivity | Required hops with exact parent-output consumption / total hops | 100% |
| Representation compatibility | Invalid parent-output → child-input kind transitions | Zero accepted |
| Artifact continuity | Exact ID/SHA/role consistency across each edge | 100% |
| Entity continuity | Prior target entity IDs reconciled in next source inventory | 100% for complete inventories |
| Relation continuity | Prior target relation IDs reconciled in next source inventory | Zero silent loss |
| Uncertainty continuity | Parent after → child before status/metric/bound equality | 100% |
| Claim origin | Unsupported promotion to measured across any hop | Zero |
| Determinism | Canonical report under equivalent serialized orderings | 100% |
| Fail-closed behavior | Disconnected/reset/incompatible chain outcomes | 100% seeded controls rejected |

### P1 direct-vs-composed transformation benchmark

Use the same source model, requirements, task tolerances and reference evidence to compare:

- **Direct path:** source observations → final IFC/tool target.
- **Composed path:** source observations → point cloud/mesh → graph/parametric model → IFC → controlled edit → re-export.

Measure per-hop and cumulative:
- exact artifact lineage and producer/consumer compatibility;
- entity survival, splits, merges, replacements and unresolved correspondences;
- relation/topology preservation and unsupported semantics;
- critical requirement/property retention from #4772;
- uncertainty growth and evidence used for any update;
- geometry and dimensional deviations under predeclared exchange-profile tolerances;
- edit/import/export failures, manual repairs, review minutes and total turnaround.

A direct path and a composed path may differ. The objective is not to assume equivalence but to identify where information is lost, approximated, altered or made uneditable. The final output should include a path-loss ledger with the first hop at which each claim/property/entity/relation changed state.

Use whole-building/capture group splits. Multiple exported versions or repeated views of the same building are not independent buildings. Pin tool versions, configuration, schema/exchange profile and reference-custody process.

## 5. Industry and standards context

The currently published official IFC release is IFC 4.3.2.0 (ISO 16739-1:2024); IFC 4.4 and IFC X modularisation remain active development tracks rather than releases to silently assume as universally supported. Keep the declared exchange profile explicit and test the same path under every supported profile. Official references:
- buildingSMART IFC specifications: https://technical.buildingsmart.org/standards/ifc/
- buildingSMART standards/committee programme: https://www.buildingsmart.org/standards/

The buildingSMART IFC Validation Service describes separate syntax, schema, normative-rule and industry-practice validation layers; it does not replace geometric/coordination visualization or an end-to-end evidence/identity chain. The Scan-to-BIM programme should compose the existing BUILT-INTEROP syntax/schema/IDS/semantic-diff owners and add this path-composition layer rather than duplicating validators:
- IFC Validation Service: https://validate.buildingsmart.org/
- BUILT-INTEROP eight-gate report contract: #2056/#2057
- IFC schema validation: #2059/#2060
- paired source-target semantic preservation: #2073/#2074
- IFC structural diff: #2117/#2126
- typed eight-gate report assembly: #2061/#2063

## 6. Promotion gates and claim ceiling

**P0** establishes deterministic chain-contract behavior on synthetic receipts. Exact-head Rust formatting, tests, strict Clippy and the focused multi-hop workflow must pass before P0 can be called qualified.

**P1** requires actual cross-tool and cross-representation paths, independently held reference evidence, explicit tool/version/profile pins, sufficient independent buildings and a separately held evaluator.

Neither P0 nor a successful IFC parser/round-trip proves physical survey accuracy, structural safety, fire performance, building-code compliance, legal survey status, return-to-service readiness or authority to mutate an authoritative asset record. Those claims remain tied to their specific professional, regulatory and evidence owners.
