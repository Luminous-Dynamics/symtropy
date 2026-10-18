# PIRB reference auditor

This standalone Rust crate audits the **synthetic** Peace Infrastructure and Resilience Benchmark (PIRB) v0.1 corpus. It is deliberately separate from the Symtropy workspace while its contract is being reviewed.

## Run

From the repository root:

    cargo test --manifest-path tools/pirb-audit/Cargo.toml
    cargo run --manifest-path tools/pirb-audit/Cargo.toml
    source_commit="$(git rev-parse HEAD)"
    cargo run --manifest-path tools/pirb-audit/Cargo.toml -- --source-commit "$source_commit"

The CLI emits a JSON receipt with the corpus SHA-256 and source commit, plus per-scenario results. A local run without `--source-commit` is explicitly marked `UNPINNED_LOCAL_RUN` and is not an exact-head receipt. The CLI exits non-zero if any audit finding is recorded.

## Scope and claim ceiling

A PASS means only that the known synthetic scenarios pass these semantic checks. It does not validate field evidence, legal or political legitimacy, human consent, privacy in a deployed system, causal effect, or deployment readiness. It is not a conflict prediction tool and must not be used to rank or target people or communities.

The evaluator checks required scenario coverage, unique IDs, hard service floors, required agreement acceptance, stale evidence, conflicting reports, sensitive-field boundaries, look-ahead leakage, explicit refusal, robust cost/worst-case-gap frontiers, harm displacement, and whether warnings have authorized owners, resources, and sufficient response lead time. It distinguishes a dispatched response from an independently verified response and never treats either as proof of prevented harm. Unit tests mutate each critical case to demonstrate fail-closed behavior.

Dependency resolution and Rust execution have to be performed in an environment with a compatible Rust toolchain and access to the declared crates. No test or build pass is claimed by the presence of this source.
