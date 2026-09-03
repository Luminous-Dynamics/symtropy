# CUF v0.11 Executable Cargo.lock Semantic Delta v0.1 — 2026-09-03

## Purpose

Extend PR #120's exact Cargo-generated lock-repair proof with a deterministic semantic report of what changed inside `Cargo.lock`.

This tranche is descriptive, not approval policy. It does not hard-code Stage A or Stage B package allowlists before the real resolver output is observed.

## Analyzer

`scripts/analyze-cuf-v0.11-cargo-lock-delta.py` uses Python's standard-library `tomllib` and compares Cargo package records by exact identity:

```text
(name, version, source)
```

Workspace packages use the explicit source marker `<workspace>`.

The analyzer reports:

- lockfile format version before/after;
- package count before/after;
- complete package records added;
- complete package records removed;
- per-package-name version/source identity-set changes;
- checksum changes for otherwise identical package identities;
- dependency-list changes for otherwise identical package identities, including added/removed dependency strings.

All lists and output keys are deterministically ordered. JSON output is canonical for review/recomputation purposes within this v1 schema; a deterministic text view provides a compact human audit surface.

## Frozen regression suite

`scripts/test-cuf-v0.11-cargo-lock-delta.py` is part of the semantic evidence boundary. It covers:

- registry package version transition;
- new workspace-package addition;
- dependency-list rewrite;
- checksum change for a stable package identity;
- deterministic repeat analysis/text output;
- rejection of duplicate exact package identities.

Both evidence production and later semantic verification extract the analyzer and this regression suite from the exact recorded repair base and require the suite to exit successfully.

The evidence capsule binds:

- analyzer Git blob;
- analyzer-test Git blob;
- analyzer-test exit status;
- analyzer-test stdout/stderr;
- Python version used during evidence production.

Test output text is checksum-preserved as evidence, but later verification requires deterministic test PASS rather than byte-identical unittest timing/log output.

## Evidence production

Use the semantic wrapper instead of the base #120 generator when semantic evidence is desired:

```bash
bash scripts/prepare-cuf-v0.11-cargo-lock-repair-semantic.sh \
  parent \
  /tmp/cuf-v0.11-parent-lock-repair
```

or Stage B:

```bash
bash scripts/prepare-cuf-v0.11-cargo-lock-repair-semantic.sh \
  v4.8 \
  /tmp/cuf-v0.11-v4.8-lock-repair
```

The wrapper delegates all mutation/failure classification to #120's proven generator. Only after a successful `GENERATED_UNCOMMITTED` result does it:

1. read the exact repair base HEAD from evidence;
2. extract analyzer + regression suite committed in that exact base;
3. prove the generated `Cargo.lock` still has #120's recorded post-repair SHA-256 and is the only working-tree delta;
4. run the exact-base regression suite and require PASS;
5. reconstruct the pre-repair `Cargo.lock` from that base commit;
6. compare it to Cargo's generated working-tree lockfile;
7. emit `CARGO_LOCK_SEMANTIC_DELTA.json` and `.txt`;
8. re-prove the generated lock hash and repository drift constraints after semantic analysis;
9. bind analyzer/test Git blobs and the v1 schema in lineage evidence;
10. mark policy as `DESCRIPTIVE_REVIEW_REQUIRED`;
11. regenerate the evidence checksum manifest.

The analyzer is therefore not taken from an arbitrary later working tree, and semantic enrichment cannot silently operate on a lockfile that drifted after #120 generated it.

## Verification

After the one-file Cargo.lock repair commit is reviewed and created, use:

```bash
bash scripts/verify-cuf-v0.11-cargo-lock-repair-semantic.sh \
  /tmp/cuf-v0.11-parent-lock-repair \
  HEAD
```

The semantic verifier first validates the capsule checksum, reads the recorded repair base, extracts **#120's exact repair verifier from that base**, and executes that historical verifier rather than trusting the later working-tree copy.

It then:

1. proves analyzer/test blobs match the repair base;
2. requires evidence-time analyzer tests to have passed;
3. reruns the exact-base regression suite and requires PASS;
4. reconstructs the repair-base and committed child lockfiles;
5. recomputes both deterministic semantic reports with the exact-base analyzer;
6. requires those reports to match evidence byte-for-byte.

A semantic verifier PASS proves:

```text
exact Cargo-generated lock repair identity
+
exact repair-base verifier provenance
+
exact analyzer/test provenance
+
regression PASS
+
exact reproducible semantic description of that repair
```

It does not prove that the semantic delta is acceptable. Human review remains required for v0.1.

## Review targets

For each real Stage A/B repair, inspect at minimum:

- newly added workspace/registry/git packages;
- removed packages;
- package-name version/source set transitions;
- checksum changes;
- dependency-list additions/removals;
- unexpected transitive churn unrelated to the manifest delta under repair.

Expected changes are hypotheses until Cargo generates the actual delta.

## Promotion to policy

After real Stage A and Stage B outputs are observed and reviewed, stable expectations may be promoted into a later fail-closed policy layer. That later policy should be derived from evidence, not guessed in advance.

## Non-goals

This tranche does not:

- mutate Cargo.lock itself;
- replace #120's exact repair verifier;
- auto-approve dependency churn;
- declare expected package names as a hard allowlist;
- claim Q1 or Q2 qualification;
- modify simulation semantics.
