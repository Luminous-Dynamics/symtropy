# QUAL-001B authority dispatch envelope v1

`repository_dispatch` is a transport trigger, not an authority credential. Its client payload is caller-supplied and therefore must be validated before it reaches contract checkout.

The closed v1 payload contains only:
- schema identity/version;
- exact frozen contract commit;
- repository-relative contract path;
- exact contract SHA-256.

It contains no verifier identity, repository, ref, tree, profile, suite, toolchain, command, shell, or workflow selector.

The release record remains the sole source of verifier identity. The validated dispatch digest is evidence about the request, not proof of qualification.

The validator is fail-closed and rejects duplicate JSON keys, unknown fields, malformed Git/SHA-256 identities, traversal/absolute paths, BOM/non-finite JSON, and oversized payloads.
