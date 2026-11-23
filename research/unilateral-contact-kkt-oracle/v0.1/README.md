# Unilateral Contact KKT Oracle v0.1

A dependency-free Rust research oracle for active-set enumeration and omitted-contact KKT verification in a restricted frictionless unilateral impulse problem.

## Declared convex problem

The input is:

    minimize 0.5 * lambda^T A lambda + b^T lambda
    subject to lambda >= 0

where:
- lambda is the vector of candidate normal impulses;
- A is a symmetric positive-semidefinite Delassus/contact-response matrix, optionally including a declared positive-semidefinite compliance term;
- b is free normal relative velocity plus a fixed declared bias/restitution term.

For a full-size impulse vector, define g = A * lambda + b. KKT requires:
- lambda >= 0 (primal feasibility);
- g >= 0 (dual feasibility);
- lambda_i * g_i = 0 for each contact (complementarity).

When A is PSD, these conditions are sufficient for global optimality of this convex QP. If a reduced candidate subset is solved and all omitted contacts have non-negative gradient, the padded reduced solution is already globally optimal for the full declared QP. A materially negative omitted gradient is a valid separation witness.

## What the oracle does

- Validates stable unique contact IDs, matrix dimensions, finiteness and symmetry.
- Uses deterministic LDL^T checks to reject matrices that are not positive semidefinite, while accepting common rank-deficient PSD matrices.
- Enumerates active sets for a restricted subset under a hard search budget.
- Checks restricted KKT conditions and returns a deterministic minimum-objective solution.
- Pads omitted contacts with zero impulse and verifies primal, dual and complementarity residuals across the entire candidate set.
- Adds the omitted contact with the most negative gradient, with stable-ID tie-break, and repeats until full-patch KKT passes or a declared refinement budget is exhausted.
- Returns typed errors and a typed RefinementBudgetExhausted status; no unresolved case is reported as globally optimal.

## Fairness and numerical boundary

All tolerances are explicit and part of the input. Symmetry is checked against a relative tolerance; the checked matrix is symmetrized by averaging corresponding entries before PSD validation and optimization. The report preserves maximum input asymmetry and LDL pivot/tolerance information. The KKT receipt separates (1) whether the full impulse vector is a KKT optimum and (2) whether the selected retained subset alone represents it, requiring omitted impulses to be zero within tolerance. Intermediate non-finite gradients, pivots, solves and objectives are typed failures.

The implementation uses binary64 arithmetic and user-declared tolerances. It is an executable numerical KKT oracle, not an interval-arithmetic formal proof. Globally optimal means globally optimal within the declared convex QP under the validated floating-point/tolerance contract; it does not establish that the QP itself captures production contact physics.

Active-set enumeration is exponential in the number of retained contacts and is intentionally a small-patch reference. The subset count is checked before enumeration. This is not a scalable general-purpose solver.

## Run

    cargo fmt --manifest-path research/unilateral-contact-kkt-oracle/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/unilateral-contact-kkt-oracle/v0.1/Cargo.toml --locked
    cargo run --manifest-path research/unilateral-contact-kkt-oracle/v0.1/Cargo.toml --example adaptive_contact_tournament --locked
    cargo clippy --manifest-path research/unilateral-contact-kkt-oracle/v0.1/Cargo.toml --locked --all-targets -- -D warnings

## Output and proposed benchmark targets

Report per refinement round:
- retained contact IDs and the added omitted-contact witness;
- restricted objective;
- primal, dual and complementarity residuals;
- minimum omitted gradient;
- active-set subsets evaluated and cumulative work.

Targets, not achieved:
- zero false KKT passes against independent exact small-set enumeration;
- 100% PSD/KKT pass or typed-failure coverage;
- on sparse-active patches, at least 25% fewer retained constraints and 20% lower total work than full-patch QP solving at equivalent KKT residual;
- on dense-active patches, expand toward full set or abstain.

## Claim ceiling

The proof applies only to the declared frictionless unilateral convex QP with validated PSD A and full-patch KKT conditions. It does not apply automatically to frictional Coulomb contact, TGS Soft, nonlinear contact evolution, nonconvex complementarity, rolling/spin friction or physical truth.

Related programme: #7203, #7199, #7130, #7127, #7155, #7195, #7197, #3775, #4366 and #460.
