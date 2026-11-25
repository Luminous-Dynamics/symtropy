# Independent exhaustive structural-isomorphism oracle v0.1

**Status:** implementation staged on `research/ergo-stream-independent-oracle-v0.1`; Rust execution pending.
**Owner:** Symtropy / ErgoStream structural qualification.
**Composes with:** #7149, #7152, #7171, #7193, #7230 and Symthaea #13401 / #11171.

## Why this exists

The current Transfer Atlas needs an evaluator-side answer to a narrow question that is independent of the candidate validator and the heuristic matcher:

> Do these two complete finite structures admit any total, bijective, relation-preserving correspondence under the declared v0 relation transformations?

The candidate may submit a witness, and the existing validator checks that witness. The independent oracle instead searches the entire bounded witness space. This removes a common-mode dependency when checking whether a paired-world fixture is structurally positive or structurally negative.

This is deliberately not a general graph engine. It is a tiny exact reference for small whole-structure isomorphism.

## Implemented search

`src/oracle.rs` exposes `exhaustive_isomorphism_oracle(source, target)`.

1. Validate node IDs, relation IDs, relation participants and finite search bounds independently.
2. Reject cardinality mismatches without enumeration.
3. Partition candidate node images by an invariant incidence signature: counts of incident directed/undirected relations by arity.
4. Enumerate all remaining node bijections, in deterministic ID order.
5. For each complete node map, independently build source-to-target relation-kind compatibility:
   - directed relation kinds may preserve or reverse participant order;
   - undirected relation kinds use unordered incidence;
   - a relation kind mixing directed and undirected instances cannot receive one consistent transform under the current v0 contract.
6. Search for a one-to-one relation-kind assignment.
7. Compare normalized relation-instance multisets and construct a deterministic relation-ID bijection.
8. Return a witness proposal plus the number of complete node bijections examined.

The implementation does not invoke `validate_structural_correspondence`. One positive unit test passes its independently constructed witness to that validator as a separate cross-check.

## Boundedness and claim ceiling

- Maximum 8 nodes, 64 relation instances and 8 relation kinds.
- Worst-case node enumeration is bounded by (8! = 40{,}320); invariant signatures prune only mappings that cannot possibly be isomorphisms.
- This is whole-structure isomorphism only. It does not decide induced/subgraph embedding, graph edit distance, homomorphism, quotienting, simulation, bisimulation, mechanism transfer or causal validity.
- It does not validate observation provenance or learner/evaluator isolation.
- The oracle is evaluator/reference code. Do not link it into an untrusted learner process or expose its result before the candidate artifact is frozen.

## Known-answer fixtures

- Positive: relabelled nodes and relation kinds, one directed edge reversed, and one undirected edge.
- Negative: (K_{3,3}) versus the triangular prism. Both are 3-regular graphs with six nodes and nine edges, but they are not isomorphic. The oracle must examine all 720 node bijections because the simple incidence signature cannot distinguish their vertices.
- Transfer Atlas cross-check: independently classifies all four existing structural fixture families, including the important case where observations are isomorphic but the mechanism is not transferable.
- Symmetry / non-identifiability: an undirected six-cycle has 12 valid node bijections (six rotations and six reflections) among 720 explored bijections. The report retains one deterministic witness but exposes the full count of valid node bijections, so a downstream evaluator can distinguish a unique map from multiple valid maps. The count is per node mapping, not per possible relation-ID permutation.
- Malformed input: duplicate node IDs and unknown relation participants.
- Resource guard: nine nodes are rejected before enumeration.
- Cardinality mismatch: rejected without searching.

## Required follow-up before qualification

1. Run Rust 1.96 formatting, unit tests, doctests and strict Clippy on the exact branch head.
2. Differentially compare oracle outcomes with a separately sourced VF2/Bliss/Nauty implementation on an expanded bounded corpus.
3. Exhaustively enumerate all simple undirected graphs through at least six vertices and compare isomorphism classes; preserve the generated corpus and mismatch cases.
4. Add adversarial fixtures with repeated relation instances, hyperedges, relation-kind collisions, mixed orientation within one kind, duplicate participants and empty relations.
5. Use oracle outcomes as fixture-truth checks for the paired-world Atlas; keep the held-out causal/mechanism evaluator separate because structural isomorphism does not imply mechanism transfer.
6. Add scoped induced/subgraph oracles only as separately named validators with separate contracts. Do not silently treat this whole-structure oracle as validating those claims.

## Benchmark implications

The oracle is not a learner baseline for the main leaderboard. It is a correctness oracle and a diagnostic upper bound on tiny worlds. Report:

- candidate witness acceptance versus independent oracle truth;
- false accepted isomorphisms and missed isomorphisms;
- number of node bijections explored and runtime by graph size;
- results on degree/marginal-matched non-isomorphic pairs;
- exact source/target structures and fixture digests.

For larger structures, compare the production candidate against exact graph-isomorphism tools and learned graph matchers under matched observation, time and compute budgets. Do not report this bounded oracle as scalable or as evidence of SOTA capability.
