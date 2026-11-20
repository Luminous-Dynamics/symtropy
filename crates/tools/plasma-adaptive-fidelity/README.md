# plasma-adaptive-fidelity

A dependency-free Rust reference for choosing among already-qualified plasma model profiles: different-order moment models and kinetic patches.

## Run

    cargo fmt --manifest-path crates/tools/plasma-adaptive-fidelity/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-adaptive-fidelity/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-adaptive-fidelity/Cargo.toml --all-targets -- -D warnings

## Policy

The evaluator is a pure function. It does not train, run or mutate a solver, select its own physical truth, or mint qualification receipts.

A candidate must match the exact physical-system/interface compatibility key, including source revision, solver build, species, collision model, geometry, boundary model, conserved-component layout and interface schema. Its calibration/error and well-posedness evidence must bind the exact candidate profile, model artifact and compatibility contract. All estimated costs must use the same declared accounting unit.

A candidate must carry a calibrated error-bound declaration within the requested tolerance, and a model-kind-specific well-posedness receipt:
- moment models require hyperbolicity-envelope evidence with a declared margin and an eigenbasis condition number below the request threshold;
- kinetic patches require a separately declared kinetic admissibility / positivity receipt.

When a current model is supplied, it must also exist in the request's candidate inventory. Any switch requires a transition receipt bound to the exact source and destination profiles and model artifacts, common interface schema, projection, reconstruction and conservation evidence. Refinement required because the current model is invalid or exceeds its error envelope can bypass the minimum-dwell restriction; coarsening cannot. A current model inside the tighter coarsening band is retained, preventing needless order churn.

Among eligible candidates, the evaluator chooses the lowest declared cost, breaking ties by profile digest. fidelity_rank is an explicit ordinal within the admitted physical-system hierarchy; it is not inferred from moment count and must not be compared across unrelated governing systems. The evaluator never assumes that higher order is better.

## Evidence / claim boundary

All evidence digests, qualification states, cost estimates and error bounds are caller-supplied declarations. A SHA-256-shaped string is not authentication; a calibrated error declaration is not a mathematical proof unless its estimator and coverage have been independently validated for the exact governing model. A finite hyperbolicity scan is not a global hyperbolicity theorem.

The current implementation provides typed admission and deterministic policy semantics only. It does not implement the Verbiest rarefied-gas estimator, the Hermite–Laguerre matrix analysis in Symtropy #6313, kinetic-to-moment projection, buffer cells, DG coupling, or a production solver adapter. The 2026 40% speedup reported for rarefied-gas HME is a cross-domain comparator, not a plasma result.

Related research:
- Symtropy #6313: source-faithful Hermite–Laguerre hyperbolicity envelope and regularization.
- Symtropy #6318: adaptive variable-order moments / kinetic-patch interface programme.
- Symtropy #6288 and PR #6294: conservative shared-face flux and domain-balance receipts.
- Symthaea #12918: closure profile eligibility and regime selection.

All tests are authored; none are represented as passing until an exact-head Rust toolchain run completes.
