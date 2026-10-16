# Wood Products Corpus Validator

This standalone Rust crate validates the synthetic fixture corpus for the Wood Products Lifecycle Reference Benchmark Suite. It is intentionally outside the production workspace so it can be compiled and tested without pulling in the Symtropy engine, renderer, Holochain, or Symthaea runtime.

Run from the repository root:

    cargo fmt --manifest-path tools/wood-products-bench/Cargo.toml -- --check
    cargo test --manifest-path tools/wood-products-bench/Cargo.toml
    cargo run --quiet --manifest-path tools/wood-products-bench/Cargo.toml -- docs/tech/wood-products-benchmark-corpus-v0.1.json --source-commit "$(git rev-parse HEAD)"

The command writes a deterministic JSON receipt to stdout and diagnostics to stderr. It validates the exact unique scenario set, unit contract, material mass balance, quality and ecological gates, custody replay/correction semantics, and a reference Pareto frontier. It hashes the exact input bytes and binds the caller-supplied source commit. It does not normalize the corpus before hashing.

A passing receipt means only that the synthetic contract corpus is internally consistent and satisfies this verifier's expected assertions. It does not run a real cut optimizer or kiln model and is not evidence of structural lumber certification, sustainable harvesting, lawful sourcing, environmental benefit, mill throughput or commercial savings.

Future candidate planners should consume the same frozen scenarios and report output to an independent evaluator. Do not reuse a candidate's ranking implementation as the verifier's only oracle.
