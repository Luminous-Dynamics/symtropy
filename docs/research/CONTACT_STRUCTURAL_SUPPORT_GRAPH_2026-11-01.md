# Conservative contact graph from Jacobian and inverse-mass support — 2026-11-01

## Purpose

This note specifies the structural graph prerequisite for [#3280 — set-valued Schur elimination](https://github.com/Luminous-Dynamics/symtropy/issues/3280).

A contact graph used to discover separators must never be inferred by thresholding floating Delassus blocks. A small nonzero coupling is still a coupling. A false structural zero can invalidate a block Schur complement and silently omit contact interactions.

The Delassus operator is

\[
W=J M^{-1}J^T.
\]

For contact i, let S_i be the union of generalized-coordinate columns whose entries are structurally supported by its three Jacobian rows. A conservative edge exists between contacts i and j when

\[
\exists a\in S_i,\ b\in S_j:
\operatorname{supp}(M^{-1})_{ab}=1.
\]

The graph is conservative: a present edge may have a numerically cancelling block, but an absent edge is a structural zero under the declared complete support patterns.

## Important distinction: body incidence versus generalized-coordinate coupling

For independent rigid bodies with block-diagonal inverse mass, contacts that share no body have no cross Delassus block. The star fixture in [#3281](https://github.com/Luminous-Dynamics/symtropy/pull/3281) uses this case.

For articulated systems, constraints, reduced coordinates or a supplied dense inverse mass, generalized-coordinate coupling can connect contacts whose geometric body supports are disjoint. The graph therefore must use both J support and M^-1 support. “No shared body” is only sufficient when the inverse-mass support is known to be block diagonal by body.

## Executable tests

The test-only implementation in contact_graph_structure_witness.rs includes:

1. **Body-local star:** four bodies and three contacts; the support graph has separator edges (0,1), (0,2), and a proven omitted leaf-to-leaf edge (1,2).
2. **Articulated coupling:** a declared M^-1 support between one leaf-only generalized coordinate and another creates a leaf-to-leaf edge despite no shared body.
3. **Tiny numeric coupling:** a symmetric 1e-14 inverse-mass coupling yields a nonzero Delassus block and remains an edge. No numeric threshold is consulted by the graph algorithm.
4. **Incomplete support:** the output is a complete conservative graph, carries support_complete=false, and is not eligible for certified separator discovery.
5. **Malformed support:** asymmetric M^-1 patterns, out-of-range coordinates and duplicate contact support indices are rejected.

An included edge carries one generalized-coordinate support witness. An omitted edge carries the number of contact-support coordinate pairs checked under the complete M^-1 pattern.

## Safety contract

- Jacobian and inverse-mass support are explicit Boolean data, not guessed from floating magnitudes.
- M^-1 support must be structurally symmetric.
- Incomplete support over-approximates to a clique and forbids certified separator use.
- A numeric Delassus block can validate the support model, but cannot erase a structural edge.
- Support patterns need provenance, ordering digests, completeness status and a deterministic replay receipt before production use.

## Next steps

- Extend support provenance from Boolean test fixtures to production sparse block metadata.
- Support generalized-coordinate support induced by constraints and articulated inverse-mass factorizations without materializing a dense M^-1.
- Add interval/numeric cross-checks that verify every structurally omitted W_ij block is exactly zero under the declared algebraic model.
- Feed the conservative graph into #3280's separator decomposition and measure false-positive edges, treewidth upper bounds, fill-in, separator width and certificate cost.
- Distinguish structural sparsity from uncertain near-zero coupling: uncertain support must remain present or become an explicit unresolved edge.

## Claim boundary

The current implementation is a fixed-size test oracle for support-pattern logic. It does not assemble production Delassus operators, prove numerical conditioning, or solve contact problems. A conservative structural graph is necessary for safe separator discovery but does not by itself prove the Schur reduction or branch completeness.


## Deterministic min-fill separator analysis

The follow-on branch adds a min-fill elimination analysis for complete symmetric structural graphs. It reports:
- deterministic contact elimination order;
- separator-neighbour count at each elimination;
- induced-width upper bound for this order (not exact treewidth);
- every fill-in edge added during elimination.

Frozen controls:
- the body-generated three-contact star has induced width 1 and no fill edges;
- a four-cycle induces one fill edge (1,3) and width 2;
- a five-contact clique has width 4 and no additional fill edges;
- incomplete or asymmetric graph support is refused for separator analysis.

This is a deterministic heuristic upper bound, not an exact treewidth algorithm. Benchmarking should compare multiple elimination heuristics and report their orders/widths, while never describing an upper bound as the true treewidth.
