# plasma-pressure-tensor-audit

Standalone Rust reference for **supplied** three-dimensional pressure tensors and velocity gradients. It audits symmetry, positive-semidefinite realizability, and the pressure-work / pressure-strain decomposition used in plasma energy-channel analysis.

## Run

    cargo fmt --manifest-path crates/tools/plasma-pressure-tensor-audit/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-pressure-tensor-audit/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-pressure-tensor-audit/Cargo.toml --all-targets -- -D warnings
    cargo run --locked --manifest-path crates/tools/plasma-pressure-tensor-audit/Cargo.toml -- benchmarks/plasma/pressure-tensor-audit-v0.json

## Physics contract

- Pressure tensor: (P_{ij}=m\int(v_i-V_i)(v_j-V_j)f\,d^3v).
- Pressure scalar: (p=\mathrm{tr}(P)/3).
- Deviatoric pressure: (\Pi=P-pI).
- Velocity gradient convention: (G_{ij}=\partial_j V_i).
- Divergence: (\theta=\mathrm{tr}(G)).
- Deviatoric strain: (D=(G+G^T)/2-\theta I/3).

The pressure work identity is

    -P:G = -p theta - Pi:D

The report keeps total pressure work, isotropic pressure-dilation work, and deviatoric pressure-strain work separate and returns the identity residual. The CLI prints the complete report but exits 1 when the disposition is `Rejected`, so shell pipelines cannot mistake an inadmissible tensor for a successful audit. The workflow includes an indefinite-tensor negative control that must emit a rejected report and a nonzero exit.

## Realizability

For symmetric 3×3 matrices, positive semidefiniteness is equivalent to nonnegative principal minors. The crate computes scale-normalized 1×1, 2×2 and 3×3 principal minors to make the tolerance independent of pressure units. A rank-deficient pressure covariance is permitted. Material asymmetry, negative principal minors beyond tolerance, and non-finite inputs are rejected or marked Rejected.

Inputs include explicit pressure units, gradient units, and coordinate-frame identity. The crate does not transform frames or infer units. If asymmetry is within the declared tolerance, the report retains the residual and uses the average of (P) and (P^T) for diagnostic contractions only.

## Scope

This is a numerical diagnostic over supplied values. It does not read PIC files, validate the kinetic source, solve fluid equations, prove energy conservation of a simulation, discover a closure, or authorise experiments or actuators. Use Symtropy #1685 for solver-run evidence and Symthaea #12923 for the magnetosheath closure research protocol.
