# Compositional Scan-to-BIM Representation Continuity v0.1

**Status:** P0 multi-hop composition auditor, CLI, two-hop known-answer fixture and hostile unit tests implemented; exact-head Rust/CI qualification pending.  
**Owner:** #4890 — SCAN-BIM-COMPOSITIONAL-CONTINUITY-001  
**Composes:** #4471 representation-transition continuity; #1607 survey/transform uncertainty; #3331 persistent identity; #4271 IFC delivery; #4763 critical-attribute retention; #4116/#1226 blind replay.

## 1. The paradigm shift: validate the path, not only the steps

A receipt can show that every individual transformation is locally plausible while the full path is invalid. Example: point cloud → mesh preserves a 2.5 mm bound, but mesh → spatial graph declares its input uncertainty as 2.0 mm. Each transition can pass its local rule if the downstream output is still conservative relative to its own declared input. The path nevertheless lost 0.5 mm of upstream uncertainty without new evidence.

The same issue exists for artifact identity: if a consumer transition appears before its producer in the serialized receipt, a one-pass validator may not yet know that the source artifact was previously emitted. Transition order is a serialization detail, not a causal property of the representation graph.

This extension introduces **path compositionality**:
- build the complete producer/consumer artifact graph before evaluating any handoff;
- derive dependencies from exact artifact identity (ID, digest and role);
- check representation-kind compatibility across every handoff;
- carry the conservative uncertainty state across each path edge;
- report branching, merging, roots, terminals, disconnected components and cycles;
- make semantic chain reports invariant to transition-array order.

It consumes #4471's per-transition auditor. It does not duplicate local claim, identity, relation or uncertainty checks.

## 2. P0 implementation

Adds `tools/scan-bim-replay-contract/src/continuity_chain.rs` and the CLI `scan_bim_continuity_chain_audit`.

The CLI first runs #4471's existing per-transition audit, then performs the chain-level audit. Both consume the same exact receipt bytes. The chain report includes the exact input SHA-256, but its semantic node ordering and graph counters are canonical.

### Chain algorithm

1. Index every output artifact from every transition. An artifact ID must have exactly one producer.
2. Inspect every source artifact. If an output with that ID exists, its SHA-256 and evidence role must match exactly.
3. For representation payload roles (registered observation, geometry output, representation output), producer output kind must match consumer input kind.
4. Derive producer→consumer dependencies independent of serialized transition order.
5. Build an artifact-sharing component graph. Branches that share a source artifact remain in the same component; disconnected chains remain visibly separate.
6. Topologically sort dependencies. Cycles or nodes blocked behind cycles make the composition state invalid.
7. Count root-to-terminal transition paths for acyclic graphs; use saturating arithmetic to avoid overflow.
8. Compare upstream `uncertainty.after` to downstream `uncertainty.before` for every dependency edge.

### Uncertainty handoff rule

For a known upstream conservative absolute bound (u_{up}), the downstream before-bound (u_{down}) must satisfy:

[
u_{down} ge u_{up}
]

unless the downstream transition explicitly declares an evidence-conditioned update with a new independent-checkpoint artifact, estimator and calibration profile. That exception is reported separately; this auditor does not independently authenticate or calibrate the checkpoint.

If an upstream bound is UNKNOWN, a downstream KNOWN before-state is rejected for a representation-only transition. Evidence-conditioned unknown-to-known is allowed only when the existing #4471 transition contract's additional-checkpoint requirements are satisfied. UNKNOWN remains visible in the composition report and cannot be interpreted as zero.

### Composition states

- `composed`: one connected acyclic artifact/transition graph with coherent handoffs.
- `fragmented`: more than one disconnected component; no single end-to-end path is claimed.
- `unresolved`: the graph is connected and acyclic, but at least one uncertainty handoff remains unknown.
- `invalid`: one or more transitions are blocked by a cycle.

Digest, role or representation-kind mismatches and unjustified uncertainty resets fail closed with an error, rather than producing a preserved-composition report.

## 3. P0 known-answer fixture and hostile tests

The committed fixture is `research/scan-bim-representation-continuity-001/fixtures/compositional_known_answers_v0_1.json`. It describes a synthetic two-hop path:

`raw_observations → surface_mesh → spatial_graph`

Expected report:
- 2 transitions;
- 1 producer/consumer dependency edge and 1 artifact handoff;
- 1 root transition and 1 terminal transition;
- 1 root-to-terminal path and 1 connected component;
- 1 known uncertainty handoff, 0 unknown handoffs;
- `composition_state = composed`.

The 10 Rust tests cover:
1. valid two-hop path with accumulated uncertainty;
2. transition-order permutation invariance, while exact input SHA changes;
3. intermediate digest mismatch even with the consumer serialized first;
4. known uncertainty reset between hops;
5. UNKNOWN→KNOWN without new evidence;
6. disconnected path components;
7. self-cycle detection;
8. ambiguous multiple producers;
9. evidence-conditioned uncertainty update with new checkpoint evidence;
10. legitimate branching from a shared source into two terminal paths.

The existing #4471 transition audit still runs first and retains its own 31 tests.

## 4. Metrics and proposed P1 targets

Keep each metric separate:
- artifact producer/consumer handoff coverage;
- artifact ID/digest/role mismatch count;
- representation-kind mismatch count;
- graph component, root, terminal and path counts;
- cyclic/blocked transition count;
- known, unknown and evidence-conditioned uncertainty handoff counts;
- uncertainty-reset count;
- canonical chain report equality under transition-array permutations;
- cross-tool path completion and information-loss counts.

P0 goals:
- 100% expected results on the committed known-answer path;
- zero silent intermediate digest/role changes;
- zero representation-only uncertainty resets;
- zero UNKNOWN→KNOWN representation-only promotions;
- 100% semantic report invariance under transition-array permutations;
- 100% detection of cycles, ambiguous producers and disconnected components.

No weighted score may compensate for a provenance or uncertainty handoff failure.

## 5. P1 research and industry comparison

The next serious evaluation should run multiple hops through at least two independent authoring/exchange toolchains, not merely export one IFC file:
1. raw/survey evidence → registered point cloud;
2. registered points → mesh/primitive;
3. mesh/graph → parametric BIM;
4. parametric BIM → IFC;
5. IFC import → edit → export in a second toolchain.

Capture the exact input/output artifact at every hop, coordinate frame, transform/configuration identity, entity crosswalk, relationship inventory, uncertainty state and critical property-retention report. Compare each intermediate handoff, not only the first and final representations.

Pin versions and profiles for Leica Cyclone 3DR, Trimble RealWorks and Autodesk ReCap/Revit. Keep geometry quality, PRR/SFI/SCR/ADS/EL, uncertainty continuity, persistent identity and relation retention as separate scorecards. Use independent survey checkpoints and evaluator-held references. Report per-building path-completion, orphan lineage, uncertainty resets, lost properties/relations, review minutes and rework.

Relevant external context:
- Wang et al. (2026) identify measurement support, object identity, spatial relations, semantics, uncertainty and model state as information that can change across Scan-to-BIM representation interfaces: https://doi.org/10.3390/buildings16204095
- The 2026 transformation-fidelity study shows why IFC/gbXML/COBie exchange must be measured on property retention, semantic fidelity, schema compliance, attribute degradation and exchange latency—not syntax alone: https://doi.org/10.1016/j.rineng.2026.112704
- buildingSMART's current official IFC release remains IFC 4.3.2.0; IFC 4.4.0.0 is a working draft, while the IFC 5 work is a next-generation development effort: https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/
- NIST's July 2026 Digital Twins Workshops summary identifies interoperability, VVUQ, cybersecurity and workforce readiness as persistent digital-twin challenges: https://doi.org/10.6028/NIST.IR.8620

## 6. Claim ceiling

P0 checks declared transition and path semantics on synthetic metadata. It does not open/hash referenced artifact bytes, parse IFC, verify geometry, establish physical metrology, authenticate checkpoints, calibrate covariance, validate code/structural safety or authorize model mutation. A `composed` report means the declared artifact graph and uncertainty handoffs are internally coherent under the P0 contract; it does not prove that the external artifacts or declared physical claims are true.
