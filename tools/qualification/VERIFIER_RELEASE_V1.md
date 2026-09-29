# QUAL-001B verifier-release identity v1

This is a verifier-owned identity boundary, not a qualification result.

It binds the exact verifier release to its repository, commit, tree, verifier profile identity and digest, qualification-contract schema/version, suite revision, and toolchain.

The record is closed and declarative. It contains no commands, shell fragments, workflow selectors, branch/tag names, Cargo arguments, or executable policy.

## Authority rule

The approved release record must live on the authoritative default-branch verifier lineage. Candidate PRs may validate candidate records in preflight, but a candidate record is never an approved release.

An authoritative dispatch must load the release record from its default-branch authority, validate its exact bytes/digest, resolve the recorded verifier commit, check out that exact commit, recompute HEAD and the tree, and compare both against the record before execution.

The record cannot emit TheoremExecutedPass/Fail or QualifiedPass/Fail.

## Release digest

release_sha256 is the SHA-256 digest of the exact UTF-8 record bytes. It is computed by verifier_release_v1.py and is not a field in the record, avoiding self-reference.

This separates record-content identity from Git commit/tree identity.

## Relationship to PKG

The PKG may reference the release identity and release_sha256, but it cannot manufacture or approve the release. The authority verifier independently reads the approved default-branch record.

## Git identity domain

QUAL-001B v1 deliberately remains SHA-1-only for its frozen contract/release identity because its existing v1 contract is frozen around 40-hex Git object IDs. The broader PKG can support both 40-hex SHA-1 and 64-hex SHA-256 Git object IDs without silently changing QUAL-001B v1.

## Bootstrap

The checked-in verifier-release-v1.example.json is a parser/test fixture only. It is not an approved release record and must never be consumed by the authoritative workflow.
