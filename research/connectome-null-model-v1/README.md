# Connectome Null Model V1

A dependency-free Rust implementation of a deterministic, directed, cell-type-constrained edge-swap null model.

This crate implements the negative-control family required by the Locomotion Mechanism Identifiability Contract in Symtropy #3450: the observed connectome must compete against rewired alternatives that preserve declared low-order structure.

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

The switch is eligible only when both edges have the same source cell type, target cell type and edge class. Self-loops and duplicate directed edges are rejected.

The implementation verifies, before returning a sample:

- the same node IDs, cell-type assignments and edge count;
- exact per-node in-degree and out-degree sequences;
- exact directed cell-type mixing and edge-class block counts;
- the same edge-weight multiset inside every cell-type/edge-class block;
- exact per-source outgoing strength and per-source edge-weight multiset;
- no self-loops and no duplicate directed connections.

Weights stay attached to their source-edge slots. Consequently, source-side weighted strength is preserved exactly; target-side incoming strength is **not** preserved. This is intentional and must not be described as a full in-/out-strength-preserving null.

## Randomization and reproducibility

- A local SplitMix64 generator makes a sample reproducible from the graph, configuration and seed.
- Candidate edge pairs are sampled uniformly over all pairs within eligible cell-type/edge-class blocks.
- The proposal is reversible for the labelled edge slots and declared block constraints; illegal proposals are holds.
- The receipt reports proposal count, accepted swaps, rejection reasons and the number of changed connections.

The crate does not certify that a finite chain has mixed, that distinct seeds yield independent samples, or that an unlabeled graph topology is uniformly sampled. It does not establish irreducibility for every degree/block sequence. A publishable null ensemble still requires multiple chains, burn-in/thinning decisions, autocorrelation or topology-statistic convergence checks, and a preregistered ensemble size. Do not use a single sample or raw acceptance rate as a significance test.

## Explicitly not preserved in V1

- target-node incoming weighted strength;
- Euclidean edge length or spatial embedding;
- synapse compartment, polyadic structure or subcellular targeting;
- developmental lineage or unrecorded biological annotations;
- higher-order motifs, communities or path structure (these are outcomes to measure, not constraints);
- any property absent from the source adapter's explicit node/edge schema.

A stricter weighted null preserving both in- and out-strength, spatial constraints, or additional anatomical metadata is a separate model family. This matters: null models that preserve more structure test a narrower and more conservative hypothesis. The literature warns that degree-only nulls can confound topology and weights, and that directed rewiring requires care about sampling bias. V1 therefore reports its exact preservation contract rather than calling every randomized graph an interchangeable "rewired connectome".

## Run

    cargo fmt --manifest-path research/connectome-null-model-v1/Cargo.toml -- --check
    cargo test --manifest-path research/connectome-null-model-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/connectome-null-model-v1/Cargo.toml --locked --all-targets -- -D warnings

## Scientific claim ceiling

A valid generated graph qualifies only the null-generator software and the declared graph invariants. It does not prove that the real connectome has a functionally meaningful advantage, explain locomotor behaviour, establish a causal neural mechanism, or support a biological claim. Such claims require matched controller/physics/sensor budgets, source-grounded model execution, an ensemble of qualified nulls, held-out interventions and independent replication.
