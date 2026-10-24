# CIV-INFRA Evidence Bundle Verification v0.1

**Status:** bounded standalone Rust implementation and synthetic contract.  
**Parent work:** [held-out calibration benchmark PR #1957](https://github.com/Luminous-Dynamics/symtropy/pull/1957), [CAL-001 #1812](https://github.com/Luminous-Dynamics/symtropy/issues/1812).

## Paradigm shift

A well-formed SHA-256 string is not evidence that source bytes were retrieved. Calibration must receive artifacts that can actually be resolved, bounded, read and compared with the manifest. This verifier introduces that missing step for local evidence bundles.

A successful result establishes only that the bytes read for each path match the manifest's declared length and digest. The manifest itself remains a trust input. Authenticity, permissions, chain of custody, scientific validity and physical truth require separate qualification.

## Manifest and report

Each manifest record declares a unique artifact ID, role, normalized relative path, expected byte length, expected SHA-256, producer label, version and applicability-domain label. The verifier reports actual byte length and digest plus all integrity issues for each artifact. Output artifact rows are sorted by artifact ID, while the raw manifest digest binds the exact manifest bytes.

Missing artifacts and digest/length mismatches appear in the report; the CLI writes the report but exits nonzero when qualification fails. Invalid manifests—duplicate IDs or paths, unsupported fields, malformed hashes, traversal attempts or excessive declared sizes—fail before verification.

## Security and resource bounds

- Maximum manifest size: 4 MiB.
- Maximum declared artifacts: 10,000.
- Maximum artifact size: 64 MiB.
- Maximum total declared size: 512 MiB.
- Maximum aggregate reserved read volume: 512 MiB, including one-byte growth probes; files whose size changes during reading are unqualified.
- Paths reject absolute form, dot/parent components, backslashes, colons and symlink components.
- Each artifact is read through a bounded reader; it must be a regular file beneath the canonical bundle root.

These limits bound the prototype's declared work. They do not claim protection against every local filesystem race or establish tamper-proof storage. A production resolver should use immutable content-addressed storage or platform-specific safe-open primitives, signed attestations, and access-controlled provenance roots.

## Interoperability and trust path for a future version

Do not evolve this prototype into an isolated signature scheme. W3C PROV models provenance through entities, activities and agents, which is a useful neutral vocabulary for describing source artifacts, solver runs and the agents responsible for producing them: https://www.w3.org/TR/prov-overview/

For authenticated build/analysis steps, evaluate in-toto attestations and their policy verification model rather than trusting manifest producer labels. The in-toto verification workflow can check signed layouts and link metadata, authorization/threshold requirements, expiry and material/product rules: https://in-toto.readthedocs.io/en/stable/command-line-tools/in-toto-verify.html and https://github.com/in-toto/attestation

A future adapter should map bundle artifacts to provenance entities, compute runs to activities, and producers/reviewers to agents; then verify signatures against an explicit trust policy. That still would not establish physical validity: independent model qualification, uncertainty treatment, applicability-domain evidence and comparison against reference measurements remain separate gates.

## Benchmark gates

- Exact match to the three frozen fixture artifact lengths and SHA-256 digests.
- 100% detection of seeded byte-length and digest mismatches.
- 100% rejection of parent traversal, absolute/Windows-style paths, symlink artifacts, duplicate IDs and duplicate paths.
- Aggregate declared-size overflow is rejected before any artifact file is read.
- Missing files remain visible in reports and force nonzero exit status.
- Deterministic output artifact ordering under manifest reordering; the raw-manifest hash must still change.
- Resource caps enforced before unbounded reads.
- No claim of artifact authenticity or scientific/physical validity from a successful integrity check.

## Composition

This component is intended to run before the held-out calibration and measurement-compatibility evaluators. A future integration should pass only verified artifact IDs and digests onward, preserving the distinction between *declared*, *retrieved*, *byte-verified*, *method-qualified* and *physically validated* evidence states.

The fixtures are artificial JSON files with synthetic values. They are not structural solver results, surveyed observations, or a calibration dataset.
