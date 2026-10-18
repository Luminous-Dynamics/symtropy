# PIRB reference auditor

This standalone Rust crate audits the **synthetic** Peace Infrastructure and Resilience Benchmark (PIRB) v0.1 corpus. It is deliberately separate from the Symtropy workspace while its contract is being reviewed.

## Run

From the repository root:

\`\`\`sh
cargo test --manifest-path tools/pirb-audit/Cargo.toml
cargo run --manifest-path tools/pirb-audit/Cargo.toml
cargo run --manifest-path tools/pirb-audit/Cargo.toml -- path/to/other-corpus.json
\`\`\`

The CLI emits a JSON report and exits non-zero if any finding is recorded.

## Scope and claim ceiling

A PASS means only that the known synthetic scenarios pass these semantic checks. It does not validate field evidence, legal or political legitimacy, human consent, privacy in a deployed system, causal effect, or deployment readiness. It is not a conflict prediction tool and must not be used to rank or target people or communities.

The evaluator checks required scenario coverage, unique IDs, hard service floors, required agreement acceptance, stale evidence, conflicting reports, sensitive-field boundaries, look-ahead leakage, and explicit refusal. Unit tests mutate each critical case to demonstrate fail-closed behavior.

Dependency resolution and Rust execution have to be performed in an environment with a compatible Rust toolchain and access to the declared crates. No test or build pass is claimed by the presence of this source.
