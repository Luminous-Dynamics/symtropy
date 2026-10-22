# HMR-0 Artifact Byte-Binding Contract v0.1

**Status:** artifact identity and manifest screening only. Not CAD verification, geometry validation, simulation validation, manufacturability, functional safety, clinical conformity or authorization to fabricate or operate.

## Why this tranche exists

A SHA-256-shaped string is not proof that an artifact exists or that its bytes match the declared digest. The tool-manifest gate intentionally uses synthetic digest strings; those are useful schema fixtures, but they must not be confused with evidence about an actual generated STL, CAD source, or simulation model.

This separate gate verifies that every artifact named in an HMR-0 concept-preview manifest is present as a regular file beneath the approved artifact root, has no symlink path component, and matches both its declared SHA-256 and byte length.

## Manifest contract

A manifest must declare:
- schema, tool ID and revision;
- evidence class exactly `concept_preview_only`;
- physical-actuation, patient-contact and clinical-function authorization flags, all false;
- one or more artifacts with stable ID, relative path, media type, byte length, SHA-256 and declared role;
- a claim ceiling explaining what the artifact does not prove.

Unknown fields, duplicate IDs/paths, absolute paths, parent traversal, backslash-containing paths, symlink components, missing files, directories instead of files, digest mismatches, and byte-length mismatches fail closed. The evaluator never follows a symlink to obtain a “matching” file outside the artifact root.

## Evidence boundary

A successful receipt establishes only byte identity against a submitted manifest and the exact tested revision of this verifier. It does not establish:
- that a mesh represents the intended shape;
- that a simulation model accurately captures the physical world;
- that force, material, wear, tolerance, or cleaning claims are correct;
- that the design can or should be manufactured;
- that a tool is safe around people or suitable for healthcare;
- any clinical function, medical-device conformity or permission to deploy.

The current tests use temporary synthetic files on purpose. The eventual bridge from Symthaea's generated preview artifacts must provide an artifact manifest with digests calculated from the actual outputs; this verifier does not assume or invent those digests.

## Verification cases

The standalone Rust test suite covers exact bytes, altered bytes, missing files, parent-directory traversal, absolute paths, symlink artifacts, unknown manifest fields, duplicate artifact IDs/paths, and attempts to set authorization flags true.

The CI workflow is pinned to Rust 1.96.0, checks formatting, unit/adversarial tests and Clippy, and identifies the exact PR head. A passing run remains an artifact-byte-integrity result only.
