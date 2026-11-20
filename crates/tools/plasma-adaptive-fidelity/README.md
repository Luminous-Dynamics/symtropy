# plasma-adaptive-fidelity

A dependency-free Rust policy evaluator for choosing among **already-qualified** plasma moment-order profiles or requesting a separately qualified kinetic patch.

## Run

    cargo fmt --manifest-path crates/tools/plasma-adaptive-fidelity/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-adaptive-fidelity/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-adaptive-fidelity/Cargo.toml --all-targets -- -D warnings

## Policy

For one local state, the evaluator considers candidate moment-order profiles. A moment candidate is eligible only when:

- its governing-equation / species / basis / normalization / geometry / mesh / boundary / solver / tableau compatibility exactly matches the request;
- its calibrated local error upper bound is no greater than the caller's requested local tolerance;
- its hyperbolicity/eigenbasis, realizability and conservation evidence is explicitly Pass and bound to the exact local-state digest;
- its eigenbasis-condition upper bound stays within the separately declared admissible limit;
- its profile/model/calibration/evidence digests are well-formed and cost units match.

Among eligible candidates it selects the lowest declared cost, with deterministic order-complexity and profile-ID tie-breaks. Unknown and rejected candidates remain visible as reasons; they are never treated as passing.

If no moment order is eligible, the policy may request a kinetic patch only when that patch has a separate, state-bound qualification receipt and exact compatibility. The result is a **request**, not execution authority.

## Ownership and limits

- Symthaea #12918 remains the owner of general closure-profile eligibility and physical regime selection.
- Symtropy #6313 owns source-faithful Hermite–Laguerre hyperbolicity / regularization derivation.
- Symtropy #6288 / PR #6294 own conservative interface composition and domain-balance receipts.
- This crate owns only local selection among moment orders and the request boundary for an independently qualified kinetic patch.
- It does not estimate kinetic error, compute eigenvalues, derive a hyperbolicity envelope, authenticate receipts, project distributions, run a solver, or authorize physical actuation.
- A digest-shaped string and caller-supplied Pass are declarations, not cryptographic or scientific verification.

The source discrepancy in Koellermeier et al. (2026) Eqs. (64)–(67) is tracked in Symtropy #6313. This policy crate does not encode their P/Q=4 coefficients or presume the journal equations have been reconciled.

See [Symtropy #6318](https://github.com/Luminous-Dynamics/symtropy/issues/6318) for the adaptive fidelity research protocol.
