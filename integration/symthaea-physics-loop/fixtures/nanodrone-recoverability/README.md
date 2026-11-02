# Synthetic recoverability frontier fixture

This fixture is a deterministic contract test, not a simulation result or flight dataset.

It registers two independent placeholder families, two training seeds, three checkpoints per flight, two interventions, and four branch seeds: 2 × 2 × 3 × 2 × 4 = 96 rows. Checkpoints at 1, 2, and 3 seconds have remaining budgets of 3, 2, and 1 seconds respectively, so all checkpoints preserve the same terminal deadline of 4 seconds. Each synthetic branch's elapsed time is 0.5 seconds and stays inside that remaining budget. The continue arm fails at the final checkpoint. The fallback arm succeeds at that checkpoint for family-a, while one of four branches fails for family-b.

## Content-addressed branch artifacts

The adjacent `branch-artifacts-v0.1/` bundle now contains exact bytes for the 64-character identifiers referenced by `branches.csv`:

- 12 state snapshots, one per family × flight × start × regime × checkpoint × training seed.
- 48 exogenous disturbance tapes, one per snapshot × branch seed.
- The same disturbance tape artifact is reused by both intervention arms for a paired branch seed; different branch seeds have distinct tape artifacts.

The CLI's `--artifact-root` verifier recomputes SHA-256 from the exact file bytes, checks parsed artifact identity against each CSV row, validates the exact referenced inventory, and rejects missing, orphaned, symlinked or non-regular files. The smoke asserts 12 verified snapshots, 48 verified tapes, and 96 bound branch rows.

**Evidence ceiling:** the payloads are explicitly synthetic strings. Their SHA-256 values prove that the referenced fixture bytes match the ledger and that the pairing identities are internally consistent. They are not hashes of captured simulator state, restored physics snapshots, or actual RNG tapes. A real runner must serialize and hash those exact runtime bytes and independently attest capture, restore fidelity and RNG source/state.

Expected synthetic oracle behavior: the fallback arm's mean latest recoverability checkpoint exceeds the continue arm's mean, with no claim about real dynamics or control benefit.


## Schema and focused verification

The closed artifact schemas are:

- `integration/symthaea-physics-loop/schemas/nanodrone-state-snapshot-v0.1.schema.json`
- `integration/symthaea-physics-loop/schemas/nanodrone-disturbance-tape-v0.1.schema.json`
- `integration/symthaea-physics-loop/schemas/nanodrone-branch-artifact-verification-v0.2.schema.json`

Unknown fields are rejected by the Rust serde types. The verifier also checks exact schema version, evidence class, payload prefix, claim-ceiling literal, identity tuple, exact SHA-256 filename and a closed directory inventory. The CLI rejects simulation or real-flight evidence when using these synthetic-only v0.1 artifact schemas; real evidence requires a separate producer-attested artifact format.

The recoverability report schema is now v0.2 because it includes the `branch_artifact_verification` receipt. A dedicated workflow `.github/workflows/nanodrone-branch-artifact-verifier.yml` checks formatting, tests/builds the standalone integration example against a pinned Symthaea commit, runs the full 96-row fixture and retains the generated JSON receipt.
