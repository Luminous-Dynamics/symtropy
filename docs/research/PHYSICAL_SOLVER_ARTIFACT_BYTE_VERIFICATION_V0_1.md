# Physical solver output artifact byte verification v0.1

**Tracking:** Symtropy [PHYS-COUPLING-001 #2241](https://github.com/Luminous-Dynamics/symtropy/issues/2241)  
**Implementation:** stacked on [solver receipt verifier PR #2291](https://github.com/Luminous-Dynamics/symtropy/pull/2291)  
**Status:** local SHA-256 byte verifier; not an execution attestation or physical qualification.

## Why this layer exists

The request/receipt verifier validates the request digest, receipt shape and cross-field consistency. The receipt's output entries also declare `artifact_ref` and SHA-256, but merely validating those strings does not show that the referenced file exists or that its bytes match. This tranche adds a local, offline check without inventing a new signature registry or evidence ledger.

## Invocation

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml -- \
  --request request.json \
  --receipt solver-receipt.json \
  --artifact-root ./output-bundle
```

Without `--artifact-root`, receipt mode remains a consistency check and reports `output_artifact_bytes_verified = false`. Providing the root opts into exact-byte verification for each declared output.

## Verification rules

1. The artifact root must canonicalize to a directory.
2. Each `artifact_ref` must be a normalized relative POSIX path. Reject absolute paths, `.` / `..` components, repeated/empty path components, backslashes, colon-bearing scheme/drive-like references and network URLs. The verifier never fetches references over a network.
3. Each path must resolve to a regular file inside the canonical root. A symlink resolving outside that root is `UNRESOLVED`.
4. Compute SHA-256 over the exact bytes read and compare it with the receipt's declared digest.
5. All declared outputs must resolve and match before `artifact_verification_status = PASS`. A known digest mismatch is `FAIL`; missing, unreadable, malformed or unsafe references are `UNRESOLVED`. An empty output set cannot pass byte verification.
6. Preserve the producer's `status`. A byte-check pass always leaves `solver_execution_authenticated = false` and `physical_result_qualified = false`.

## Hostile controls and benchmark target

The focused tests and workflow cover:

- exact matching fixture bytes → `PASS`;
- changed/tampered bytes → `FAIL` with expected and observed digest;
- path traversal → `UNRESOLVED`;
- symlink escaping the root → `UNRESOLVED` (Unix-targeted unit test);
- unresolved receipt with no outputs → `UNRESOLVED`;
- the workflow repeats a synthetic exact-match run, then mutates an output and tests a path-traversal reference at the exact checked-out commit.

**Target:** 100% correct dispositions over the frozen positive and negative corpus. This is a software verifier target, not a physics accuracy score. The receipt and output fixture are synthetic; no actual solver is invoked.

## Evidence ladder

```text
request/receipt consistency
  -> local output-byte identity
  -> execution/authorship attestation
  -> numerical verification against independent references
  -> experimental validation and uncertainty assessment
  -> engineering-use qualification
```

Do not collapse these stages:

- matching digest ≠ evidence that the solver executed;
- solver execution ≠ numerical correctness;
- numerical agreement ≠ real-world model validity;
- validation in one applicability envelope ≠ universal structural safety or engineering approval.

The artifact tree must remain immutable during verification. This implementation does not protect against concurrent filesystem mutation, a compromised host/kernel, false provenance, or a malicious producer that supplies bytes consistent with a false receipt. Execution/authorship attestation, signer authorization, numerical verification and physical validation remain separate evidence gates owned by existing authorities.
