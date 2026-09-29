# Symtropy Formal Provenance Schema v0

This directory defines the repository-local proof knowledge graph (PKG) interchange format.

The PKG is an evidence/provenance layer. It is **not** a proof checker and it never upgrades an execution result into authority.

## Design rules

- Semantic identity, artifact identity, and execution identity are distinct.
- Theorem/lemma/invariant semantic identity is an explicit SHA-256 digest; labels are descriptive and are never the semantic key.
- Every `ProofArtifact` binds to an exact semantic target digest and an explicit `checked_by` `ProofChecker` edge.
- Proof execution records bind the exact artifact, checker, source commit/tree, source digests, subject head/tree, and result.
- Proof derivations are represented as DAG-compatible references; shared subproofs are not duplicated.
- Source-sensitive assertions bind to exact repository commit/tree and, where applicable, path/blob digest.
- Qualification evidence binds the qualification contract, verifier release, subject head/tree, execution/evidence identity, and semantic result.
- `Passed` is an execution result. `QualifiedPass` is an independently established authority state.
- Graph edges are provenance-monotone: inserting provenance can add knowledge, but cannot manufacture authority.
- Canonical JSON is UTF-8 with deterministic field ordering and normalized repository paths.
- Malformed, incomplete, stale, or mismatched evidence is rejected rather than repaired.

## Authority boundary

A `qualifies` edge is valid only when independently checkable qualification evidence establishes the authority transition. The graph itself does not infer qualification from a `Passed` execution.

## Proof DAG boundary

A `ProofArtifact` may identify child proof nodes by stable IDs. The artifact need not inline every derivation step. This keeps repeated subproofs shared and makes the graph compatible with certificate-based proof DAGs.

Recent ITP 2025 work independently supports tool-independent certificates and DAG proof encodings for independently checked reasoning results. See the linked research in the project issue.

## Semantic validator v0

Run the validator with the repository's Python 3 interpreter:

```text
python3 formal/provenance/validate.py formal/provenance/example-v0.json
```

The validator is deliberately stdlib-only and runtime-independent. It checks:
- node/edge uniqueness and endpoint existence;
- normalized source provenance and SHA-256 identity fields;
- relation-to-node-kind compatibility;
- evidence references and qualification bindings;
- proof-artifact dependency acyclicity;
- fail-closed qualification behavior for skipped/infrastructure/non-terminal executions;
- exact contract/verifier digests and exact execution/source bindings;
- historical `supersedes`/`invalidated_by` handling so stale evidence cannot establish current qualification;
- the authority boundary: the graph cannot manufacture `QualifiedPass`.

Machine-readable output has stable status classes (`Valid`, `SchemaInvalid`, `GraphInvalid`, `EvidenceInvalid`, `AuthorityNotEstablished`) and deterministic diagnostic codes.

The validator is a semantic layer above `schema-v0.json`; it does not replace an independent proof checker or the existing qualification system.


## Proof-impact engine v0

Run the advisory impact analyzer with:

```text
python3 formal/provenance/impact.py <canonical-or-interchange-graph.json> --changed <node-id> [<node-id> ...]
```

The impact engine consumes only semantically valid provenance. It computes a deterministic revalidation frontier across proof, checker, execution, verifier, contract, and evidence dependencies. It never grants, revokes, or infers qualification authority.

Machine-readable output uses `impact_status` values `NoImpact`, `RevalidationRequired`, and `GraphInvalid`.

Impact reason codes are dependency-specific rather than generic:
- `SEMANTIC_DEPENDENCY_CHANGED` — theorem/lemma/invariant semantics changed.
- `PROOF_ARTIFACT_CHANGED` — the checked proof artifact changed.
- `CHECKER_CHANGED` — the independent proof checker changed.
- `EXECUTION_CHANGED` — a proof execution identity changed.
- `SOURCE_DEPENDENCY_CHANGED` — a source commit/tree dependency changed.
- `VERIFIER_RELEASE_CHANGED` — the qualification verifier release changed.
- `QUALIFICATION_CONTRACT_CHANGED` — the qualification contract changed.
- `SHARED_SUBPROOF_CHANGED` — a shared proof dependency changed.
- `EVIDENCE_STALE` — evidence must be reconsidered because its provenance relationship changed.

Traversal is reverse-dependency based and deterministic: adjacency is sorted, breadth-first traversal chooses the shortest dependency path, and the first path wins ties. The output is therefore reproducible under graph node/edge reordering. A change to a source identity can propagate through explicit metadata bindings such as `source_commit_id` or `source_tree_id`; no raw digest string is treated as a graph dependency unless it is also represented by a node.


## Revalidation planner v0

The revalidation planner converts an impact frontier into an explicit, machine-readable work plan:

```text
python3 formal/provenance/revalidation_plan.py <graph.json> <impact.json>
```

It is deliberately downstream of semantic validation and impact analysis. It:
- preserves exact node identity and impact provenance paths;
- requires exact ProofExecution bindings and explicit ProofArtifact checker bindings;
- detects contradictory reference kinds;
- blocks superseded or invalidated historical evidence;
- distinguishes proof, execution, qualification-evidence, and dependency rechecks;
- emits deterministic records and fails closed on invalid inputs.

The planner authenticates its impact input before planning. It recomputes the canonical frontier from the validated graph and the declared `changed_node_ids`, then requires exact equality of the supplied impact payload. Tampered, incomplete, duplicated, unknown, or internally contradictory frontiers are rejected. Changed roots are deliberately not emitted as downstream impact records; they are the declared causes of the frontier and are therefore not silently converted into revalidation work units. A `Ready` plan means only that the dependency metadata is sufficiently bound to describe the revalidation work; it does **not** mean that any proof or qualification has passed.


## Qualification admission envelope v0

The admission envelope is the narrow handoff between authenticated provenance planning and the existing authoritative qualification machinery. It binds the canonical impact digest, plan digest, exact changed roots, subject repository/head/tree, exact node identity digests, reference identities, and expected qualification result. It recomputes the impact frontier and revalidation plan instead of trusting caller-supplied records.

Run: `python3 formal/provenance/admission.py <graph.json> <impact.json> --subject-repository Luminous-Dynamics/symtropy --expected-result QualifiedPass`.

The envelope is an admission identity, not an authority result. QualifiedPass/QualifiedFail remain outcomes established only by the approved default-branch qualification verifier. The envelope never upgrades a Passed execution by itself. The implementation fails closed on missing identity digests, inconsistent subject head/tree values, tampered frontiers, stale/historical evidence, or mismatched bindings. Canonical JSON ordering does not affect the admission digest.
