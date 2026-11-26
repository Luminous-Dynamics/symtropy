# Independent exhaustive structural-isomorphism oracle v0.1

**Status:** full-witness ambiguity extension staged on `research/ergo-stream-full-witness-ambiguity-v0.1`; exact-head Rust execution pending.
**Owner:** Symtropy / ErgoStream structural qualification.
**Composes with:** #7149, #7152, #7171, #7193, #7230 and Symthaea #13401 / #11171.

## Full-witness ambiguity extension — validation state

The extension now reports full semantic witness classes separately from total node-map count. It enumerates relation-kind assignments and participant transforms, quotients ID permutations of duplicate incidence records, and returns `BoundedUnknown` rather than claiming uniqueness after the 65,536-class cap.

New integration fixtures cover unique node map / ambiguous relation-kind assignment, directed preserve-versus-reverse ambiguity, duplicate-instance quotienting, mixed-orientation rejection, repeated hyperedge participant multiplicity and budget exhaustion. The formatting diff from the first attempt has been applied; exact-head tests and strict Clippy remain pending. The base #7432 oracle has now passed its own Rust 1.96 format/tests/Clippy workflow.

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
8. Count distinct full-witness equivalence classes, rather than stopping at the first compatible mapping; preserve one deterministic representative witness.

The implementation does not invoke `validate_structural_correspondence`. One positive unit test passes its independently constructed witness to that validator as a separate cross-check.

## Boundedness and claim ceiling

- Maximum 8 nodes, 64 relation instances and 8 relation kinds.
- Worst-case node enumeration is bounded by (8! = 40{,}320); invariant signatures prune only mappings that cannot possibly be isomorphisms.
- Full witness-class enumeration is separately bounded at 65,536 classes. Reaching the next valid class yields `BoundedUnknown`; the report must not claim uniqueness from a truncated count.
- The report separates `valid_node_mapping_count` from `valid_witness_class_count`. A node map may admit multiple relation-kind assignments or participant transforms.
- This is whole-structure isomorphism only. It does not decide induced/subgraph embedding, graph edit distance, homomorphism, quotienting, simulation, bisimulation, mechanism transfer or causal validity.
- It does not validate observation provenance or learner/evaluator isolation.
- The oracle is evaluator/reference code. Do not link it into an untrusted learner process or expose its result before the candidate artifact is frozen.

## Known-answer fixtures

- Positive: relabelled nodes and relation kinds, one directed edge reversed, and one undirected edge.
- Negative: (K_{3,3}) versus the triangular prism. Both are 3-regular graphs with six nodes and nine edges, but they are not isomorphic. The oracle must examine all 720 node bijections because the simple incidence signature cannot distinguish their vertices.
- Transfer Atlas cross-check: independently classifies all four existing structural fixture families, including the important case where observations are isomorphic but the mechanism is not transferable.
- Five-node exhaustive differential check: enumerate all 1,023 non-empty labeled simple undirected graphs on five nodes, canonicalize each independently over all 120 vertex permutations, recover the 33 non-empty unlabeled isomorphism classes (the empty graph is excluded because the existing v0 structural-transfer contract requires at least one relation), then compare the oracle on all 1,089 ordered class pairs.
- Symmetry / non-identifiability: an undirected six-cycle has 12 valid node maps (six rotations and six reflections) among 720 explored bijections; its 12 full witness classes remain ambiguous.
- Relation-kind ambiguity: two source kinds with identical incidence can be assigned to two target kinds in two distinct ways even when the node map is unique.
- Participant-transform ambiguity: a symmetric directed relation multiset can admit both preserve-order and reverse-order transforms for one relation kind.
- Duplicate-instance quotient: permutations of relation IDs among repeated identical incidence instances do not inflate semantic witness-class count; a deterministic ID pairing is returned.
- Hyperedge multiplicity: repeated participant IDs remain significant in relation signatures.
- Budget exhaustion: a symmetric eight-node/eight-kind fixture exceeds the 65,536 witness-class cap and returns `BoundedUnknown`, never false uniqueness.
- Malformed input: duplicate node IDs and unknown relation participants.
- Resource guard: nine nodes are rejected before enumeration.
- Cardinality mismatch: rejected without searching.

## Required follow-up before qualification

1. Run Rust 1.96 formatting, unit tests, doctests and strict Clippy on the exact branch head.
2. Differentially compare oracle outcomes with a separately sourced VF2/Bliss/Nauty implementation on an expanded bounded corpus.
3. Extend the current all-pairs differential check from the 33 non-empty five-node classes to six-node graph classes; preserve the generated corpus and mismatch cases, and keep the six-node computation behind an explicit test/time budget.
4. Continue adversarial differential testing for repeated relation instances, hyperedges, relation-kind collisions, mixed orientation within one kind, duplicate participants and empty relations. Initial relation-kind / transform / duplicate-instance / mixed-orientation / hyperedge / count-budget cases are now staged in integration tests.
5. Use oracle outcomes as fixture-truth checks for the paired-world Atlas; keep the held-out causal/mechanism evaluator separate because structural isomorphism does not imply mechanism transfer.
6. Add scoped induced/subgraph oracles only as separately named validators with separate contracts. Do not silently treat this whole-structure oracle as validating those claims.

## Benchmark implications

The oracle is not a learner baseline for the main leaderboard. It is a correctness oracle and a diagnostic upper bound on tiny worlds. Report:

- exact full-witness ambiguity class count and completeness status;
- candidate witness acceptance versus independent oracle truth;
- false accepted isomorphisms and missed isomorphisms;
- number of node bijections explored and runtime by graph size;
- results on degree/marginal-matched non-isomorphic pairs;
- exact source/target structures and fixture digests.

For larger structures, compare the production candidate against exact graph-isomorphism tools and learned graph matchers under matched observation, time and compute budgets. Do not report this bounded oracle as scalable or as evidence of SOTA capability.
