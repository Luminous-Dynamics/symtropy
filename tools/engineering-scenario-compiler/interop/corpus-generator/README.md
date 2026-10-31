# Engineering Scenario Generated JCS Corpus v0.1

A standalone Rust generator for a deterministic raw JSON valid/reject corpus
used to qualify the portable engineering-scenario JCS identity profile.

The generator depends on the compiler crate's strict raw-input parser rather
than maintaining a second Rust parser. Valid bytes are canonicalized with the
pinned serde_jcs 0.2.0 implementation. The Go verifier in the sibling
go-verifier directory independently preflights the same raw bytes and
canonicalizes valid cases using the pinned cyberphone implementation.

## Run

From the repository root:

    cargo generate-lockfile --manifest-path tools/engineering-scenario-compiler/interop/corpus-generator/Cargo.toml
    cargo fmt --manifest-path tools/engineering-scenario-compiler/interop/corpus-generator/Cargo.toml --all -- --check
    cargo test --manifest-path tools/engineering-scenario-compiler/interop/corpus-generator/Cargo.toml --locked
    cargo clippy --manifest-path tools/engineering-scenario-compiler/interop/corpus-generator/Cargo.toml --all-targets --locked -- -D warnings
    cargo run --release --manifest-path tools/engineering-scenario-compiler/interop/corpus-generator/Cargo.toml --locked -- target/jcs-adversarial-corpus

The output directory must be empty or absent. Generation is deterministic for
the frozen generator version, profile and seed.

## Frozen panel

The current generator emits:

- 268 valid cases, including twelve fixed edge cases and 256 deterministic
  nested cases from an LCG seed.
- 14 expected-reject cases.
- Two metamorphic equivalence groups: object property-order changes and
  escaped/literal Unicode spellings that decode to the same strings.

Valid cases cover nested arrays/objects, UTF-16 key sorting, BMP and
supplementary-plane names, composed/decomposed Unicode values and property
names, escaped/literal equivalence, negative zero, subnormal and maximum
finite binary64 values, decimal/exponent boundaries and safe-integer edges.
Generated cases rotate deterministic payload lengths of 64, 512, 4,096 and
16,384 bytes.

Reject cases cover duplicate object member names at the root, nested in an
object, nested in an array, escaped-equivalent duplicates, Unicode-equivalent
duplicates, invalid UTF-8, unpaired high/low surrogate escapes, non-finite
binary64 overflow, unsafe positive/negative integer literals, trailing JSON,
malformed tokens and unescaped control characters.

## Output structure

    manifest.json
    manifest.sha256
    valid/<case-id>.json
    expected/<case-id>.jcs
    reject/<case-id>.bin

The manifest binds case IDs, relative file paths, raw SHA-256 values, expected
canonical-byte SHA-256 values and lengths, equivalence groups, seed, generator
version, canonicalization profile and claim ceiling. The Rust generator
strict-parses every valid input and rejects every expected-reject input before
publishing the manifest. It verifies the two metamorphic canonical-output
equivalence groups before writing.

## Independent Go verification

From the pinned GOPATH workspace:

    GO111MODULE=off GOPATH=/path/to/gopath go test .
    GO111MODULE=off GOPATH=/path/to/gopath go run . --corpus \
      /path/to/jcs-adversarial-corpus \
      /path/to/go-results.jsonl \
      /path/to/go-summary.json

The Go corpus mode checks the manifest schema/profile/counts, safe relative
paths and exact input SHA-256. It then:
- strict-preflights every valid input, canonicalizes it with the independent
  Go JCS implementation, and requires byte-for-byte equality with the Rust
  expected canonical file and digest;
- strict-preflights every expected-reject input and records the rejection
  reason without invoking the JCS canonicalizer.

Results are written per case. The summary separately counts valid matches,
valid inputs rejected by Go, canonical mismatches, expected-reject inputs
incorrectly accepted, and correctly rejected inputs. Any mismatch or false
accept returns a nonzero exit status.

## Performance

The workflow benchmarks strict preflight, JCS canonicalization and the
combined path on generated 64-byte, 512-byte, 4-KiB and 16-KiB payload cases.
It reports Go's benchmark allocations and timing distributions over five
repetitions. The existing engineering-scenario compiler workflow separately
benchmarks the actual typed Scenario IR envelope; generated arbitrary JSON
benchmarks are not substituted for that application payload.

## Claim ceiling

A pass establishes exact byte agreement and rejection behavior across this
frozen generated corpus and the declared RFC 8785 profile. It does not prove
engineering-model correctness, input authenticity, solver correctness,
physical truth or operational safety. Future schema/profile changes require
new explicit corpus versions and golden expectations rather than silently
mutating this v0.1 panel.
