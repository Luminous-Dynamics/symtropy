# External solver runner protocol v0.1

**Status:** implemented as a Rust library contract; exact-head CI and real-SU2 execution remain pending.  
**Parent:** CFD-REF-001 (#2894).  
**Initial bounded case:** AERO-QUAL-001 (#2960), NACA 0012 incompressible SA-RANS.

## Purpose

The runner executes one explicitly selected external analysis executable and creates a byte-bound process receipt. It is the first execution layer between Symtropy's Rust-owned engineering ledger and independent discipline solvers such as SU2.

This is not a CFD solver, a convergence validator, a digital signature, a trusted-host attestation, or experimental validation. The receipt's evidence ceiling is:

`process_receipt_only_caller_version_claim_unverified_no_convergence_or_physical_validation`

A zero exit code is necessary but not sufficient for numerical or physical admission.

## Request contract

`ExternalSolverRequest` fixes:

- a single-line run ID and solver identity/version label;
- an explicit executable path; the executable is resolved and SHA-256 hashed before launch;
- an entirely new run directory (existing directories are rejected);
- an argv vector, never a shell-interpolated command string;
- a cleared process environment populated only with explicitly supplied key/value pairs;
- source inputs with expected byte lengths, SHA-256 values and source references;
- relative expected-output paths;
- a wall-clock timeout and per-stream stdout/stderr byte cap.

Each input is checked before staging and the staged copy is re-hashed. Input/output paths must be relative and may not traverse outside the fresh run root. Reserved runner filenames cannot be replaced by a staged input or expected solver output.

## Persisted run bundle

A run directory contains:

- `environment_manifest.txt`: variable names and hashes of values; secret values are not copied into the manifest;
- `invocation_manifest.txt`: hex-encoded run identity, executable path, argument vector, expected outputs and staged-input identities, plus platform and resource limits;
- `stdout.log` and `stderr.log`: bounded raw process streams;
- caller-declared solver outputs in their run-relative paths;
- `expected_outputs.csv`: present/missing state, byte length and SHA-256 for every expected output;
- `runner_receipt.csv`: process outcome, executable/invocation/environment/log/output digests, elapsed time and receipt digest.

The runner preserves spawn failures, non-zero exits, timeouts, output-cap termination and missing expected outputs as distinct receipt states. It does not translate missing numeric data to zero or discard failed runs.

## Independent replay

`verify_external_solver_receipt` re-hashes the invocation and environment manifests, staged inputs, stdout/stderr logs, declared outputs and output manifest; it then recomputes the receipt digest and compares the persisted receipt representation.

This is an integrity check against a supplied receipt. Without a signature or trusted attestor, it does not prove who created the receipt or whether the declared version/environment is truthful.

## v0.1 limits (must remain visible)

- The solver version label is caller-supplied. The executable bytes are hashed, but source commit, compiler, build flags, linked libraries and Nix derivation closure are not independently attested.
- Environment values are hashed and redacted, so the manifest supports comparison, not reconstruction of secret-bearing environments.
- OS and architecture are recorded; CPU/GPU model, kernel, driver, compiler and peak memory are not yet captured.
- Timeout termination kills the direct child process. Process-group/descendant termination is not yet guaranteed; this must be addressed before using the runner for wrappers that spawn long-lived worker processes.
- The output cap is applied per stream. It bounds persisted logs, but it is not a general CPU, memory, disk, filesystem or network sandbox.
- A completed process receipt says nothing about residual convergence, force-history stability, conservation, coefficient definitions, GCI, uncertainty, reference-data suitability or physical validity.
- A caller can declare an incomplete expected-output set. A case-specific adapter must freeze required outputs and parse them with strict schemas before numerical admission.

## Required next gates

1. Exercise the fake-executable negative controls: literal argv preservation, pinned input staging, stale-directory rejection, malformed input digests, non-zero exit, missing outputs, timeout, output cap, and receipt replay.
2. Add process-group/resource isolation and independently verify executable/build provenance.
3. Implement a case-specific SU2 adapter for the frozen NACA 0012 0° and 10° instances. It must create derived config files from pinned inputs, capture the complete required output set, and distinguish process status from convergence status.
4. Only after the runner and parser pass their negative controls, run the exact pinned SU2 fixture. Do not treat the 897×257 tutorial mesh as a grid-convergence study; the frozen case requires at least three nested/refined TMR meshes for convergence claims.
5. Independently replay the output artifacts and compare only the observable and reference datasets admitted for that case. TMR's Cp data, CFL3D Cf profile and compatible tripped force data remain separate evidence classes.

## Security boundary

Case configuration is data. The runner accepts an explicit executable path and argv vector; it does not execute a command embedded in a case file, invoke a shell, download a solver, or silently substitute versions. Callers should not pass secrets as command-line arguments because the invocation manifest records argv values in reversible hex form.
