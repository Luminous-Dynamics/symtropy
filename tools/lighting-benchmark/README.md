# Lighting benchmark evaluator

This isolated, dependency-free Rust crate implements a conservative reference evaluator for lighting-service observations. It is an early contract kernel for the Symtropy lighting research protocol, not a photometric solver or optimizer.

## What it does

- Validates numeric inputs and duplicate/missing gate evidence.
- Accepts raw grid samples and derives average, minimum, maximum and minimum/average uniformity itself; it does not trust candidate-supplied illuminance aggregates.
- Supports a separate robust-evaluation path that uses pointwise illuminance intervals, lower-bound service metrics, conservative uniformity and upper-bound energy/power. The interval semantics must be recorded; this is not an automatic guarantee of simultaneous statistical coverage. The robust evaluator also requires an explicit interval basis and source ID; measurement-uncertainty and calibrated-prediction intervals require a non-empty coverage statement.
- Rejects empty, incomplete, negative, non-finite, inverted-interval or overflowed grids before ranking.
- Enforces case-defined illuminance, uniformity and electrical-power limits.
- Keeps pass, fail, unknown, unsupported, not-evaluated, timeout and invalid-output states distinct.
- Makes candidates rankable only when all numeric constraints and every required evidence gate pass.
- Emits energy per compliant task-hour only for a compliant, positive-duration observation.
- Provides a small Pareto-dominance helper for already normalized minimization vectors.
- Includes a deterministic experiment selector that ranks externally supplied expected decision-loss reduction against cost, while rejecting malformed estimates and excluding unsafe or over-budget options. It does not infer or calibrate value-of-information estimates itself.
- Includes a deterministic synthetic-only evaluator CLI, a 30-case synthetic stress matrix spanning task-plane optics, adaptive control and off-grid resilience, family-level development/held-out isolation checks, and a 30-test suite covering the evaluator, robust interval logic and evidence provenance, selector, and stress-corpus contract.

## Receipt audit

The shell helper `tools/lighting-benchmark/verify-receipt.sh` checks a complete-grid receipt against the exact case, design, scene, property, adapter-config, executable, environment, settings, normalized-sample, stdout and stderr artifacts supplied to it. It verifies SHA-256 digests, exact equality between the normalized sample JSON and receipt samples, fixture grid order/coordinates, independently recomputed illuminance summaries, interval ordering and the case's service/power gates before accepting a compliant status.

Run its synthetic positive case and negative controls with:

    bash tools/lighting-benchmark/tests/receipt-audit-smoke.sh

Important boundary: this is a receipt-consistency verifier, not an independent Radiance parser or photometric validation. It does not prove that the adapter correctly converts native solver output into the normalized lux grid; that parser must be pinned and independently tested in the Radiance adapter workstream. It also audits complete-grid receipts only; a malformed, partial, timed-out or absent-solver run remains unrankable and needs a separate failure-receipt contract. Passing this helper does not establish product compliance, optical accuracy, flicker safety or real-world energy savings.

## Run

From the repository root:

    cargo test --manifest-path tools/lighting-benchmark/Cargo.toml --locked
    cargo run --manifest-path tools/lighting-benchmark/Cargo.toml --locked --bin lighting-benchmark-demo
    cargo run --manifest-path tools/lighting-benchmark/Cargo.toml --locked --bin lighting-stress-matrix

## Evidence boundary

The example data are hand-authored synthetic fixtures. Passing tests establish only the evaluator's local contract. They do not validate Radiance outputs, a physical fixture, spectra, glare, flicker, temperature, product safety, annual energy, or superiority over industry products.

The stress matrix is a fixed set of synthetic interventions, not a calibrated distribution or a claim about the frequency/severity of real faults. It deliberately keeps held-out scenario families separate from development families within each tier. The JSON schema at docs/research/lighting-run-receipt.schema.json defines the planned evidence-carrying run receipt, including source/input hashes, toolchain identity, raw grid coordinates/samples, derived metric provenance, interval basis, gate results and evidence tier. It structurally rejects a compliant receipt with a nonzero solver exit or a non-passing gate, and requires measurement records for calibrated/physical evidence tiers. Cross-field checks such as sample ordering, interval lower ≤ upper, digest correctness and summary-to-grid equality still belong in the independent verifier. A future adapter must parse and validate the versioned case JSON, pin solver/tool identities, preserve raw results, and pass candidates through an independent evaluator. No missing or unsupported evidence may be imputed as a pass.
