# Connectome Null Model V1

A dependency-free Rust implementation of three deterministic, directed connectome edge-swap null families: degree-only (N0), cell-type/edge-class-constrained (N1), and an exact-strength equal-weight switch (N2-EQ).

This crate implements N0/N1 and a restricted N2-EQ negative-control ladder required by the Locomotion Mechanism Identifiability Contract in Symtropy #3450. The observed connectome must compete against rewired alternatives that preserve declared low-order structure.

## V1 graph contract

Input is an aggregated directed simple graph:

- one node record per neuron or declared model unit;
- one directed edge per ordered source-target pair;
- positive integer weight units, already quantized by the source-data adapter;
- a declared source cell type, target cell type and edge class for each connection.

V1 does not read connectome file formats, resolve neuron identities, or aggregate synapses. Those steps must be performed by a source-pinned adapter and recorded outside this crate.

## Exact invariants

Every accepted switch transforms two edges:

    source_1 -> target_1
    source_2 -> target_2

into:

    source_1 -> target_2
    source_2 -> target_1

Self-loops and duplicate directed edges are rejected.

### N0 — directed degree-only baseline

Any two edges may be proposed, regardless of source/target cell type or edge class. This preserves:
- node IDs and cell-type assignments;
- exact per-node in-degree and out-degree;
- exact per-source outgoing strength and per-source edge-weight multiset;
- total edge count and global edge-class counts;
- no self-loops and no duplicate directed connections.

Cell-type mixing and type/edge-class block weight distributions are allowed to change. This is the baseline that tests whether a connectome result exceeds degree and source-side weight constraints alone.

### N1 — cell-type and edge-class constrained

The switch is eligible only when both edges have the same source cell type, target cell type and edge class. N1 additionally preserves:
- exact directed source-type × target-type × edge-class block counts;
- the edge-weight multiset inside every such block.

### N2-EQ — exact-strength equal-weight switch

The switch is eligible only when the two selected directed edges have exactly the same positive integer weight. The targets are exchanged while weights remain attached to their source-edge slots.

Because the two weights are equal, N2-EQ preserves:
- every node's exact directed in-degree and out-degree;
- every node's exact incoming and outgoing weighted strength;
- the global edge-weight multiset;
- the per-source edge-weight multiset and outgoing strength;
- simple directed graph constraints.

N2-EQ allows cell-type/edge-class mixing to change. It is a deliberately restricted microcanonical control: if no equal-weight edge pair exists, it returns `NoEligibleEdgePairs`. It does not split weights, interpolate, round, or silently relax strength constraints. If integer weights are highly heterogeneous, the reachable state space may be very small.

The receipt records which constraints were required and which were preserved. N0/N1 keep weights attached to source-edge slots, preserving source-side strength but not target-side incoming strength. N2-EQ preserves exact incoming and outgoing strength sequences as well as the global weight multiset.

## Randomization and reproducibility

- A local SplitMix64 generator makes a sample reproducible from the graph, configuration and seed.
- Candidate edge pairs are sampled by a fully specified, seed-reproducible rule over eligible cell-type/edge-class blocks; illegal proposals are holds.
- V1 makes no stationary-distribution, irreducibility, finite-run mixing or uniform-unlabeled-graph claim. A fixed proposal rule is not by itself evidence that a finite ensemble is an unbiased null distribution.
- The receipt reports proposal count, accepted swaps, rejection reasons and the number of changed connections.

The crate does not certify that a finite chain has mixed, that distinct seeds yield independent samples, or that an unlabeled graph topology is uniformly sampled. It does not establish irreducibility for every degree/block sequence. A publishable null ensemble still requires multiple chains, burn-in/thinning decisions, autocorrelation or topology-statistic convergence checks, and a preregistered ensemble size. Do not use a single sample or raw acceptance rate as a significance test.

## Explicitly not preserved in V1

- target-node incoming weighted strength for N0/N1 (N2-EQ preserves it);
- Euclidean edge length or spatial embedding;
- synapse compartment, polyadic structure or subcellular targeting;
- developmental lineage or unrecorded biological annotations;
- higher-order motifs, communities or path structure (these are outcomes to measure, not constraints);
- N2-EQ does not explore all feasible weighted graphs: weights remain attached to source-edge slots and only equal-weight target switches are proposed;
- any property absent from the source adapter's explicit node/edge schema.

A broader microcanonical weighted null that explores more of the feasible in/out-strength-preserving graph space, as well as spatial constraints or additional anatomical metadata, remains a separate model family. This matters: null models that preserve more structure test a narrower and more conservative hypothesis. The literature warns that degree-only nulls can confound topology and weights, and that directed rewiring requires care about sampling bias. V1 therefore reports its exact preservation contract rather than calling every randomized graph an interchangeable "rewired connectome".

## Run

    cargo fmt --manifest-path research/connectome-null-model-v1/Cargo.toml -- --check
    cargo test --manifest-path research/connectome-null-model-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/connectome-null-model-v1/Cargo.toml --locked --all-targets -- -D warnings

## Scientific claim ceiling

A valid generated graph qualifies only the null-generator software and the declared graph invariants. It does not prove that the real connectome has a functionally meaningful advantage, explain locomotor behaviour, establish a causal neural mechanism, or support a biological claim. Such claims require matched controller/physics/sensor budgets, source-grounded model execution, an ensemble of qualified nulls, held-out interventions and independent replication.
