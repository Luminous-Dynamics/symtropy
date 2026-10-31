# Adversarial Scenario Identity Qualification v0.1

Status: design contract for issue #3104. This document does not claim that the adversarial corpus or duplicate-key guard has been implemented.

## 1. Protocol, not serializer

The portable identity profile is a protocol boundary. It needs consistent typed interpretation, canonical bytes and explicit rejection of ambiguous input before digesting. Keep three identity levels distinct:

1. Raw source identity: exact source bytes.
2. Semantic identity: profile/version/compiler/parser envelope plus typed Scenario IR.
3. Compilation-subject identity: source identity, semantic identity, verified artifact manifest, disposition and complete compile obligations under the existing domain separator.

Do not repurpose an existing digest field. Any incompatible semantic or encoding change requires a new profile ID and golden vectors.

## 2. Parse-before-canonicalize boundary

RFC 8785 JCS is defined over an I-JSON-compatible data model. Duplicate property names must not be accepted as ambiguous input; number values must fit the IEEE-754 binary64 data model; invalid non-finite values are rejected; string data is preserved without Unicode normalization. Property sorting uses UTF-16 code-unit ordering and array order is preserved.

A typed serializer cannot detect every ambiguity in raw bytes after a lossy parse. Therefore the untrusted-input path must reject duplicate object member names before conversion into a map or typed struct. This is a parser security requirement, not a JCS formatting option.

Define one explicit boundary for invalid UTF-8 and escaped lone-surrogate behavior. Rejection must occur before canonical output or a hash is emitted.

## 3. Frozen corpus classes

Valid cases include RFC 8785 shared vectors, current checked-in vectors, reordered object properties, recursively nested objects in arrays, BMP and supplementary-plane property names, composed and decomposed Unicode strings as distinct content, escaped and literal spellings that decode to the same string, zero/negative zero, subnormal values, exponent boundaries, maximum finite binary64, bounded integer values and valid typed Scenario IR envelopes.

Invalid or ambiguous cases include duplicate property names at top-level and nested levels, invalid UTF-8, lone-surrogate escape attempts, NaN/infinities/exponent overflow, unsupported raw numeric values, unknown profile, duplicate artifact role, malformed digest, missing required envelope field and inconsistent source/semantic/artifact/subject digest associations.

The corpus manifest must classify each input as valid or expected-reject and name the rejection boundary. Parser errors and typed-schema errors are different outcomes.

## 4. Metamorphic identity matrix

| Mutation | Source identity | Semantic identity | Compilation-subject identity |
|---|---|---|---|
| TOML whitespace/comment only | changes | stable | changes (source-bound) |
| JSON object property order only | n/a to TOML source | stable | stable if canonical subject fields unchanged |
| Verified artifact list order only | n/a | stable | stable after role sorting |
| Artifact bytes or role | n/a | stable | changes |
| Compile obligation | n/a | stable if not part of IR semantics | changes |
| Typed semantic field | n/a | changes | changes |
| Compiler/schema/parser/profile version | n/a | changes | changes |
| Compile disposition | n/a | stable if not in semantic IR | changes |

Tests must verify the actual data flow for each row; do not infer an identity level from a field name.

## 5. Cross-language verifier

For every valid case, Rust and the pinned independent Go canonicalizer must emit byte-for-byte identical canonical bytes. For every invalid case, each implementation's documented preflight/canonicalization boundary must reject it. Compare failure classes where intended to be common, but do not require identical error text.

Run the verifier on actual typed Scenario IR envelopes, not only generic JSON samples. Record corpus revision/hash, Rust toolchain and serde_jcs version, Go toolchain and upstream canonicalizer commit, per-case input/output digests, valid/rejected/mismatch counts, bounded mismatch samples and claim ceiling.

## 6. Property-based and mutation testing

Generate deterministic cases with recorded seeds and cap every generator. Include field insertion/removal/reordering, profile/version substitution, digest corruption, artifact-role duplication, obligation mutation and numeric boundary mutations. Every mutation states which identity level must change or which parser stage must reject it.

A digest mismatch is not a semantic mismatch; report them separately. A false accept of ambiguous raw input is a security failure even when two implementations agree on the same wrong interpretation.

## 7. Performance protocol

Separate raw input read, strict duplicate-aware JSON parse, typed deserialization/schema validation, JCS canonicalization, SHA-256, artifact manifest construction and full compilation-subject construction.

Report p50/p95/p99, throughput, allocation count and peak memory by payload size. Use 50 warmups and at least 500 samples for small deterministic local tests; increase sample size and report confidence/noise for public comparisons. Preserve the prior whole-compiler benchmark as a separate end-to-end metric.

Do not claim superiority over general-purpose JSON Schema or CUE validators until the corpus contains equivalent semantics, accepted/rejected rules, hardware, warm/cold behavior and output requirements.

## 8. Acceptance and claim ceiling

Acceptance requires byte equality on all valid frozen cases; rejection of every invalid/ambiguous frozen case before digest generation; positive/negative metamorphic checks for all three identity levels; no change to legacy digest definitions; reproducible artifacts with implementation versions and exact corpus hash.

A pass establishes cross-implementation identity agreement and rejection behavior for the frozen corpus and declared profile. It does not prove the compiler's engineering model is physically valid, authenticate upstream artifacts, or establish solver correctness.
