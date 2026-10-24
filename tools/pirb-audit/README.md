# PIRB reference auditor

This standalone Rust crate audits the **synthetic** Peace Infrastructure and Resilience Benchmark (PIRB) v0.1 corpus. It is deliberately separate from the Symtropy workspace while its contract is being reviewed.

## Run

From the repository root:

    cargo test --manifest-path tools/pirb-audit/Cargo.toml
    cargo run --manifest-path tools/pirb-audit/Cargo.toml
    source_commit="$(git rev-parse HEAD)"
    cargo run --manifest-path tools/pirb-audit/Cargo.toml -- --source-commit "$source_commit"

The CLI first validates the schema document against JSON Schema Draft 2020-12 and validates the corpus instance against that schema; either error fails closed before semantic auditing. The JSON receipt binds both the corpus and schema SHA-256 values, source commit, and per-scenario semantic results. CI also validates that the receipt's schema digest matches the checked-in schema and archives both inputs with the dependency lock. A local run without `--source-commit` is explicitly marked `UNPINNED_LOCAL_RUN` and is not an exact-head receipt. The CLI exits non-zero if schema validation or any semantic audit fails.

## Scope and claim ceiling

A PASS means only that the known synthetic scenarios pass these semantic checks. It does not validate field evidence, legal or political legitimacy, human consent, privacy in a deployed system, causal effect, or deployment readiness. It is not a conflict prediction tool and must not be used to rank or target people or communities.

The evaluator performs real Draft 2020-12 instance validation against the checked-in JSON Schema, then checks required scenario coverage, unique IDs, hard service floors, required agreement acceptance, stale evidence, conflicting reports, sensitive-field boundaries, look-ahead leakage, explicit refusal, robust cost/worst-case-gap frontiers, harm displacement, distributional group-set claims across connected communities, adaptive stop-rule transitions (including a 128-case bounded state-space cross-check), and whether warnings have authorized owners, resources, sufficient lead time plus a declared safety reserve, and an auditable response-event history. A response marked verified must include the ordered issued/acknowledged/dispatched/arrived/completed/outcome-verified lifecycle, non-negative monotonic offsets, unique event IDs, evidence references, and an independent final outcome attestation. A lower aggregate service gap cannot erase adverse movement in named groups: the four-community spillover fixture improves its aggregate gap from 36 to 34 while two groups worsen, and requires the claim `qualified_with_displacement`. It never treats a warning, dispatch, or verified response as proof of prevented harm. The reserve is a synthetic input, not a universal timing policy. Stop-rule cases compute `BLOCKED`, `REVIEW_REQUIRED`, or `CONTINUE` from explicit triggers, evidence state, accountable authority, and scope assessment; unknown inputs and unregistered triggers fail closed. This is state-machine conformance only, not live intervention control. Unit tests mutate critical cases to demonstrate fail-closed behavior.

Dependency resolution and Rust execution have to be performed in an environment with a compatible Rust toolchain and access to the declared crates. No test or build pass is claimed by the presence of this source.
