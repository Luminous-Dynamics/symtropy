# Independent Dependency Graph Adequacy Benchmark v0.1

**Status:** synthetic reference evaluator; exact-head validation pending.  
**Owner:** [ORBITAL-DEPENDENCY-GRAPH-ADEQUACY-001 #7089](https://github.com/Luminous-Dynamics/symtropy/issues/7089).  
**Implementation:** [draft PR #7093](https://github.com/Luminous-Dynamics/symtropy/pull/7093), stacked on [typed assurance bundle #6598](https://github.com/Luminous-Dynamics/symtropy/pull/6598).  
**Claim ceiling:** agreement with a bounded synthetic dependency grammar only.

## 1. Why a separate graph adequacy layer is necessary

#6598's invalidation adapter answers:

> Given a dependency graph, which nodes and descendants must be requalified when a source artifact or scope changes?

That is not the same as:

> Does the dependency graph contain all edges that ought to exist?

If the edge from a producer to a downstream decision claim is absent, a perfectly implemented graph traversal cannot invalidate the unlinked descendant. Correct invalidation over an incomplete graph can therefore leave stale evidence looking current.

This issue owns dependency graph adequacy, not invalidation propagation and not action-universe completeness:
- #6598: exact digest/scope pins and selective downstream invalidation.
- #7001/#7013: independent action-domain generation and common-mode action omissions.
- #7089/#7093: independent dependency grammar and missing-edge detection.

## 2. Three representations, not two agreeing manifests

The v0.1 corpus separates:

1. **Producer manifest:** node identity, produced artifact ID/type, exact artifact digest and scope digest.
2. **Consumer manifest:** transformation operation, output artifact, input artifact IDs and scope digest.
3. **Declared assurance graph:** explicit producer-node → consumer-node edges, each bound to its artifact ID.

A transformation grammar is authored separately from those records. For each registered operation it declares the upstream artifact types that must affect that transformation. The evaluator resolves these required types against the producer inventory and checks both consumer inputs and graph edges.

This allows the benchmark to plant a common-mode omission in the consumer and graph records while retaining an independent obligation in the grammar.

The grammar is a finite synthetic reference model, not a proof that all real engineering dependencies have been identified.

## 3. Frozen v0.1 transformation grammar

| Consumer operation | Required upstream artifact types |
|---|---|
| `build_closure` | `domain_manifest` |
| `build_domain_oracle` | `domain_manifest` |
| `evaluate_feasibility` | `closure_report`, `domain_oracle_report` |
| `select_query_policy` | `feasibility_report`, `domain_oracle_report` |
| `build_loss_model` | `closure_report` |
| `compute_regret` | `feasibility_report`, `loss_model` |
| `calibrate_regret` | `regret_report`, `loss_model` |
| `compose_decision_frontier` | `query_policy`, `regret_report`, `calibration` |
| `ingest_campaign` | none |

Every required artifact type must resolve to exactly one producer. Missing or ambiguous producer evidence blocks closure. Producer and consumer scope digests must match before the edge can support the claim.

## 4. Distinct failure states

- `GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR`: the supplied graph and consumer inputs match all obligations in this finite grammar.
- `MISSING_DEPENDENCY_EDGE`: the consumer manifest lists a required input but the assurance graph omits the edge.
- `COMMON_MODE_DEPENDENCY_OMISSION`: both the consumer manifest and assurance graph omit a grammar-required input.
- `CONSUMER_MANIFEST_OMISSION`: graph edge exists but consumer's declared input list omits it.
- `UNDECLARED_DEPENDENCY_EDGE`: an extra graph edge / input is present; record over-approximation rather than silently accepting or removing it.
- `DEPENDENCY_MODEL_INADEQUATE`: operation is not covered by the registered grammar.
- `DEPENDENCY_SOURCE_UNAVAILABLE`: required artifact type has zero or multiple producers.
- `DEPENDENCY_SCOPE_MISMATCH`: producer and consumer have different scope digests.
- `INVALID_GRAPH`: malformed identity, duplicate edge, missing endpoint, output mismatch or cycle.

Do not convert an unknown operation into an empty dependency set. Unknown means model inadequacy, not “this operation depends on nothing.”

## 5. Frozen challenge corpus

Seven synthetic cases:
1. Complete graph positive control.
2. Missing graph edge while the consumer still declares the input.
3. Common-mode omission: graph and consumer declarations omit the same required parent.
4. Consumer-only omission while the graph edge remains.
5. Extra edge / over-approximation.
6. Unsupported transformation, which must yield model inadequacy.
7. Producer/consumer scope mismatch.

The Rust suite includes a row-order permutation property. A separately authored JavaScript implementation independently derives obligations from the operation grammar and compares the complete output to the Rust receipt.

## 6. Metrics and prospective gates

Targets, not achieved field results:

| Metric | Target |
|---|---:|
| Planted required-edge omissions detected in bounded grammar cases | 100% |
| Common-mode dependency omissions escaping the independent grammar | 0 |
| False complete results on seeded graph blockers | 0 |
| Unrelated graph nodes invalidated by a missing edge | 0 |
| Exact Rust / JS oracle mismatches | 0 |
| Input-permutation result differences | 0 |
| Unknown transformation accepted as complete | 0 |

For larger suites, report edge-level precision and recall, false-positive over-approximation, unresolved producer count, scope mismatch, runtime, memory, graph fanout, maximum path and reviewer time. Report p50/p95/p99 at 10 / 100 / 1,000 / 10,000 / 100,000 nodes and edges. Keep graph omission detection separate from invalidation correctness.

## 7. Known limitations and next steps

The current grammar derives dependencies from operation classes and producer artifact types. It does not establish:
- that the operation grammar contains every transformation used by a real project;
- that a producer's declared artifact type accurately captures all causal inputs;
- that an omitted capability or ontology is not shared by both producer and consumer manifests and the grammar;
- that receipt bytes, signatures, timestamps or source repositories are authentic;
- that an edge proves a real-world causal dependency;
- that a synthetic test is physical or operational evidence.

Next steps under #7089:
1. Add independently generated challenge manifests from a second implementation path.
2. Add required dependency families for policy versions, authority roles, calibration, schemas, data-source passports and operating-envelope inputs.
3. Plant shared omissions across producer, consumer and graph generators and require a separate inadequacy state.
4. Introduce conservative dependency supersets and measure precision/over-invalidation rather than only recall.
5. Bind a graph-adequacy receipt as a prerequisite to any claim that downstream evidence is current.
6. Add external domain-specific challenge sets and blinded reviewer trials before using real operational evidence.

A pass is only `GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR` for the named corpus and exact source head. It must not be generalized to “the project's dependency graph is complete.”
