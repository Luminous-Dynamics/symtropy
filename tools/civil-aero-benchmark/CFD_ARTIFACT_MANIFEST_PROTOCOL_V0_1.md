# CFD artifact manifest protocol v0.1

## Purpose and evidence ceiling

This protocol binds a CFD run summary to exact files on disk using byte lengths and SHA-256 digests. It prevents a summary from referring to missing, mutated or misidentified build/configuration/geometry/mesh/log/history/output files.

A successful audit means actual file bytes under the selected artifact root match digests and lengths in the supplied manifest. It does not establish:
- who authored or supplied the manifest;
- whether the executable was the binary that produced the output;
- whether the configuration was the one loaded at run time;
- whether the log is complete or truthful;
- whether the solver is numerically correct or experimentally validated;
- airworthiness, structural adequacy or manufacturing release.

The current canonical manifest digest is unsigned. It is SHA-256 over a normalized representation generated after verification, not a hash of an external serialized manifest file. A signature, trusted timestamp, CI/OIDC attestation, hardware identity and independent chain-of-custody review remain future work.

## Required artifacts per run

Exactly one file for each of eight roles is required:

| Role | Expected content |
|---|---|
| solver_build | Executable or immutable solver package/build artifact |
| physics_configuration | Complete numerical/physical configuration, boundary conditions, turbulence/transition and discretization settings |
| geometry | Exact CAD/geometry source or neutral geometry artifact used to produce the mesh |
| mesh | Exact mesh file or immutable mesh package |
| run_log | Solver stdout/stderr and process/run metadata |
| residual_history | Per-equation/per-variable residual history and stopping criteria |
| force_history | Per-iteration or per-time-step force/moment monitor history |
| field_output | Raw field/surface output required for independent recomputation and comparison |

The summary receipt's caller-supplied solver-build, configuration, geometry and mesh digests must equal the actual corresponding artifact digests. Other artifact hashes are retained in the canonical per-run manifest digest.

## Manifest CSV

The CLI accepts a header-exact CSV with five comma-free columns:

    run_id,artifact_role,relative_path,byte_length,sha256

- run_id groups eight artifact rows for one CFD grid run.
- artifact_role must be one of the exact strings above.
- relative_path must be a safe path relative to the selected artifact root.
- byte_length must be the exact file size in bytes.
- sha256 must be a 64-character hexadecimal SHA-256 digest.

Quoted fields, unknown columns, empty values and malformed rows are rejected. Paths containing separators, traversal components, absolute paths, delimiter/control characters or symlinks escaping the canonical artifact root are rejected. Repeated roles or repeated paths are rejected. Every manifest must contain all eight roles exactly once.

The verifier opens the file, reads metadata from that file handle, streams the same opened handle to the hash utility, checks the number of bytes actually read, then compares the digest. This avoids trusting a precomputed length or digest string alone. A file modified in-place during hashing may still present a filesystem-specific consistency risk; for high-assurance archival use, stage immutable snapshots or read-only content-addressed objects before verification.

## Canonical manifest digest

After all eight actual files verify, records are sorted by role and the following UTF-8 representation is formed:

- schema=<schema>
- run_id=<run id>
- then one line per role: <role>|<relative path>|<byte length>|<lowercase sha256>

The SHA-256 of those normalized bytes is emitted as canonical_manifest_sha256. It provides stable identity for the manifest content as normalized by this implementation; it is not a signature and does not authenticate its source.

## Study-level binding

The Rust API verify_cfd_study_artifacts requires one manifest per CfdGridRun, rejects missing/duplicate/unmatched run IDs, verifies every required artifact and binds solver build, physics configuration, geometry and mesh hashes to the numeric summary. The returned status intentionally says manifest_unsigned_not_source_authenticated_not_experimentally_validated.

The command-line tool verifies individual run bundles. Study-level binding across summary structs is currently exposed through the Rust API, not the CSV CLI.

## Verification tests and limits

Unit tests include:
- successful eight-role verification for each synthetic run;
- same-length single-byte mutation rejection;
- missing-role rejection;
- path traversal rejection;
- symlink escape rejection (Unix);
- manifest/run identity mismatch;
- validly hashed mesh bytes that conflict with the mesh hash in the numerical run summary.

The independent Python CI oracle creates its own synthetic files and manifest, then verifies the Rust CLI output, canonical digest, role count, byte lengths and digests. It also checks negative controls for same-length corruption, path traversal and a missing required role.

These synthetic tests prove implementation behaviour only. Before any real DPW8 or solver bundle is admitted, preserve raw artifacts and original provenance, validate the exact run configuration and toolchain, retain per-equation residual histories and establish signed/attested custody where possible. The V5 participant file remains subject to unresolved upstream licensing; this protocol does not change that terms gate.

## Next integrity requirements

1. Add a signed canonical manifest with a pinned signing key/identity and verifiable timestamp.
2. Capture the executable and every runtime dependency (or a reproducible container/Nix closure) rather than a single binary when required.
3. Bind invocation arguments, environment, host/accelerator, core/GPU allocation, timestamps and job scheduler receipt.
4. Parse and independently verify residual, force and field artifacts against the summary rather than merely hashing them.
5. Add independent replay of a small golden case and compare numerical outputs within declared tolerances.
6. Keep byte integrity, numerical verification, experimental validation and airworthiness as separate evidence levels.

## References

- NASA CFD verification assessment: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- NASA grid convergence and spatial convergence: https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html
- NIST IR 8298, Industrial CFD VVUQ: https://www.nist.gov/publications/summary-industrial-verification-validation-and-uncertainty-quantification-procedures


## Composite numerical-admission receipt

The library helper \`create_cfd_evidence_chain_receipt\` reruns byte-level artifact verification for every grid, recomputes numerical admission from the supplied CfdEvidenceStudy, checks run identity and principal hashes across both results, and returns:
- an artifact-bundle SHA-256 over the ordered canonical per-run manifest digests;
- a numerical-admission SHA-256 over the declared criteria, per-grid numerical/resource audit metrics and every GCI triplet;
- a composite receipt SHA-256 over both component digests and the case/quantity identity.

The composite receipt digest can be supplied as the policy benchmark's admission_receipt_sha256. The policy scorecard still does not authenticate who created it, and the receipt is not signed. It only binds the actual bytes verified at call time to the numerical admission recomputed from the same study structure. A trusted runner identity, signed provenance/attestation and experimental validation remain separate requirements.

A negative-control test changes one admitted numerical grid value while retaining the artifact bundle; the numerical-admission and composite receipt digests must both change. Another control rejects mismatched manifest/run IDs. The receipt's evidence label always states that it is unsigned, not source-authenticated and not experimentally validated.
