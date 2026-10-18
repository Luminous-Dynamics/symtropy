# EnergyPlus Adapter Qualifier — A0 envelope

This standalone Rust tool establishes the pinned package and receipt contract for [ENERGYPLUS-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1900). **It does not invoke EnergyPlus, generate a building model, or validate physical predictions.**

## Current pin

- Release: [EnergyPlus 26.2.0](https://github.com/NatLabRockies/EnergyPlus/releases/tag/v26.2.0), tag commit `4bd7a1f26fb749a83257855aba6760e26c8a3c8f`.
- Supported package for this A0: Ubuntu 24.04 x86_64.
- Installer: `EnergyPlus-26.2.0-4bd7a1f26f-Linux-Ubuntu24.04-x86_64.run`.
- Published asset SHA-256: `e602dbe95235e853f0f51d8e99702da4a951fdb4473ebe7f463bedd4f70532e8`.
- Exact size: `198937282` bytes.

The installer has **not** been downloaded or executed by this change. The epJSON schema revision is pinned to EnergyPlus 26.2.0; its exact file digest must be captured from the installed package into each run receipt. A0 does not claim a captured schema digest or a completed solver run.

## Commands

From the repository root:

```sh
cargo test --manifest-path tools/energyplus-qualifier/Cargo.toml
cargo run --manifest-path tools/energyplus-qualifier/Cargo.toml -- --check-spec --json
cargo run --manifest-path tools/energyplus-qualifier/Cargo.toml -- --check-fixtures --json
cargo run --manifest-path tools/energyplus-qualifier/Cargo.toml -- --verify-package /path/to/EnergyPlus-26.2.0-4bd7a1f26f-Linux-Ubuntu24.04-x86_64.run --json
cargo run --manifest-path tools/energyplus-qualifier/Cargo.toml -- --validate-receipt run-receipt.json evidence-root --json
```

- `--check-spec` checks the release pin and static adapter envelope.
- `--check-fixtures` runs ten synthetic known-answer receipt-gate cases; it does not run EnergyPlus.
- `--verify-package` streams the installer through SHA-256 and checks both its exact digest and byte size. It only verifies package identity.
- `--validate-receipt` validates the run envelope and recomputes the hashes of every referenced schema/model/geometry/weather/schedule/configuration/log/output artifact under the supplied evidence root. Paths must be relative, stay inside that root, resolve to files and match the reported hashes. Only a fully consistent receipt is eligible for *reference comparison*.

Exit status is zero only for a passing spec/fixture check, or an externally supplied receipt whose envelope is eligible for reference comparison. Blocked/unknown receipts return nonzero. The JSON receipt is designed for automation.

## Gate semantics

- `eligible_for_reference_comparison`: package/input provenance and artifact digests agree, execution completed with exit code zero, the run converged, there are no severe/fatal errors or partial outputs, and all required output series/meters are present with the contract's exact units and reporting frequencies. This is **not** a model validation result.
- `blocked`: process failure, non-convergence, severe/fatal solver messages or partial outputs.
- `unknown`: mismatched/unverifiable pin, missing provenance, schema revision mismatch, absent/invalid required output, unsafe/unavailable artifact or an inconsistent self-declared disposition.

A zero-valued output is never substituted for a missing output. A process exit code of zero cannot override a failed convergence state, missing result file or mismatched digest.

## What is intentionally not in A0

No installer download is part of CI; no thermal simulation is performed; no EnergyPlus result is claimed; no local code, accessibility or safety verdict is generated. The initial output contract is a small scope—zone air temperature at timestep frequency and the `Electricity:Facility` meter at hourly frequency—chosen as explicit examples of a variable and a meter in the official [EnergyPlus 26.2.0 output reference](https://energyplus.readthedocs.io/en/v26.2.0/guides/input-output-reference/index.html). The exact input schema, model package, weather, schedules, configuration, output series and logs remain receipt-bound artifacts.

Next gates are separately versioned:
- A1: choose rights-checked applicable ASHRAE Standard 140 tests and input-negative controls.
- A2: run one single-zone reference model and compare time trajectories against a separate synthetic trajectory oracle.
- A3: compare against suitable independent reference/measured data, with calibration and held-out sets kept separate, and choose tolerances before evaluation.
