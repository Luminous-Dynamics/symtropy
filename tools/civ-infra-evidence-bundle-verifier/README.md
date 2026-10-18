# CIV-INFRA Evidence Bundle Verifier v0.1

A small, standalone Rust verifier for checking whether local evidence artifact bytes match a declared bundle manifest.

## Run

```sh
cargo fmt --manifest-path tools/civ-infra-evidence-bundle-verifier/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civ-infra-evidence-bundle-verifier/Cargo.toml
cargo run --locked --manifest-path tools/civ-infra-evidence-bundle-verifier/Cargo.toml -- \
  tools/civ-infra-evidence-bundle-verifier/fixtures/evidence-bundle-v0.1.json \
  tools/civ-infra-evidence-bundle-verifier/fixtures/bundle-root
```

The command emits a deterministic report. It returns a nonzero exit code when any artifact is missing, has unsafe path resolution, is a symlink, is not a regular file, exceeds resource caps, or fails declared byte-length or SHA-256 verification.

## Contract

- Paths must be normalized, relative, slash-separated paths. Parent-directory traversal, absolute paths, Windows-style paths and symlinked components are rejected.
- SHA-256 and byte length are checked against the manifest for the bytes read.
- Duplicate artifact identifiers or paths fail before resolution.
- Reports are sorted by artifact ID independent of manifest record order. The manifest receipt still hashes the exact input bytes.
- Limits: 4 MiB manifest, 10,000 entries, 64 MiB per artifact and 512 MiB total declared size.
- A passing result verifies integrity relative to this manifest only. It does not authenticate who supplied the manifest, establish chain of custody, validate a solver, or prove physical correctness.
