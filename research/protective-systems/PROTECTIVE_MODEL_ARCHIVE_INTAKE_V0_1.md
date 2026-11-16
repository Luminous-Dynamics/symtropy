# Protective Model Archive Intake v0.1

**Snapshot:** 16 November 2026  
**Owner:** PROTECT-MODEL-ARCHIVE-INTAKE-001 #5503  
**Parent:** PROTECT-MODEL-EVIDENCE-001 #5474  
**Status:** metadata-only ZIP preflight research and synthetic conformance implementation; no extraction or live provider access

## 1. Purpose and paradigm

A specialist-model adapter should not equate downloaded successfully with safe to unpack or valid research evidence.

The default should be **preserve → inspect → bound → attest locally → quarantine**, not extract first and inspect later.

For v0.1, preserve the original compressed bytes as an immutable artifact, compute a local SHA-256, inspect archive directory/local-header metadata, and produce a manifest of accepted/rejected members. Do not write archive members to a filesystem, invoke programs, import modules, or decompress payloads in this tranche.

This is deliberately narrower than a general archive tool. It creates an evidence boundary around external model datasets.

## 2. Verified IN-CORE/pyIncore integration boundary

At public pyIncore commit `540b3cc568643ff788479ffd4004853d059d7067`, DataService includes:
- `get_dataset_metadata`;
- `get_dataset_files_metadata`;
- `get_dataset_file_metadata`;
- `get_dataset_blob`;
- `unzip_dataset`.

The inspected `unzip_dataset` creates a destination directory and calls `zipfile.ZipFile(...).extractall(foldername)`; if a cache directory already exists, it returns that directory rather than re-inspecting the archive. A second helper downloads a file blob and calls `extractall(dirname)`.

This is a concrete interoperability boundary. It is **not** proof that every upstream extraction is exploitable or that every ZIP library is vulnerable.

Pinned source:
- https://github.com/IN-CORE/pyincore/blob/540b3cc568643ff788479ffd4004853d059d7067/pyincore/dataservice.py

## 3. Why metadata-only preflight

Three independent public references illustrate why an archive boundary deserves explicit treatment:

1. Python's current `zipfile` documentation says its extraction helpers attempt to sanitize absolute and parent-traversal names, but still warns that untrusted archives require prior inspection: https://docs.python.org/3/library/zipfile.html
2. OWASP's Web Security Testing Guide documents archive directory traversal and symlink-related archive attacks as separate test cases: https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/10-Business_Logic_Testing/09-Test_Upload_of_Malicious_Files/
3. On 9 October 2026, the Rust `zip` crate published GHSA-4v9h-p7m9-rh26. The advisory describes an extraction path check that validated the entry name but failed to constrain attacker-controlled symlink target bytes; it records the affected pre-release range and patched version: https://github.com/zip-rs/zip2/security/advisories/GHSA-4v9h-p7m9-rh26

The lesson is not never use ZIP. It is that the exact library version and extraction API are part of the security boundary. An archive-member path check alone is not enough when symlinks can be materialized.

## 4. V0.1 contract

### 4.1 Accepted work

- preserve exact archive bytes;
- compute local SHA-256 and byte length;
- locate and validate End Of Central Directory (EOCD);
- validate central-directory bounds and declared entry count;
- inspect entry names, flags, compression method, timestamps, CRC fields, sizes, external attributes and local-header offsets;
- verify that local and central headers agree on fields v0.1 supports;
- enforce a versioned `ArchiveIntakePolicy`;
- produce deterministic member metadata and explicit abstentions/rejections.

### 4.2 Not performed

- no filesystem extraction;
- no decompression;
- no CRC-32 validation against decompressed member data;
- no malware scanning;
- no executable-content analysis;
- no model or scientific validation;
- no claim about archive-provider authenticity.

The report must say `METADATA_ONLY_CONTENT_NOT_VERIFIED`. A parseable central directory is not proof that the archive is safe to extract or that any member data is intact.

### 4.3 Supported ZIP profile

For this first implementation:
- single-disk archives only;
- no ZIP64;
- no encryption;
- no data-descriptor flag;
- no symlinks, hard links or special file types;
- stored (0) and deflate (8) methods are represented in metadata, but neither is decompressed;
- ASCII relative paths only; non-ASCII names are explicitly unsupported rather than silently recoded or normalized;
- exact path and case-fold duplicate checks;
- central directory and local-header metadata must agree;
- no trailing junk after EOCD.

Anything outside this deliberately narrow profile abstains/rejects. Do not silently coerce unsupported archives into acceptance.

### 4.4 Path rules

Reject:
- absolute paths, drive prefixes and UNC paths;
- backslashes, colon/alternate-data-stream syntax, NUL and control characters;
- empty, `.` or `..` path components;
- trailing spaces or dots;
- Windows device-name components such as CON, PRN, AUX, NUL, COM1–COM9 and LPT1–LPT9;
- case-insensitive path collisions;
- file/directory ancestor conflicts;
- symlink or special-file external attributes.

Preserve the exact original filename bytes and the exact archive bytes. The parser must never silently normalize a provider name and then treat the normalized result as the provider's original identity.

### 4.5 Resource limits

Every evaluation must carry an explicit, versioned policy:
- maximum compressed archive bytes;
- maximum central-directory bytes;
- maximum member count;
- maximum filename length;
- maximum compressed bytes per member;
- maximum uncompressed bytes per member;
- maximum aggregate declared uncompressed bytes;
- maximum declared compression ratio.

These are configured limits, not universal constants. The scanner checks arithmetic overflow and rejects when declared metadata exceeds the frozen policy. Since v0.1 does not decompress, actual expanded bytes and CRC remain unverified.

## 5. Compose E1 evidence envelopes

E2 should produce receipts that can attach to the E1 model evidence envelope:
- original archive SHA-256 and byte length;
- exact source dataset/file ID and provider metadata revision;
- archive format/profile and scanner version;
- frozen policy ID and digest;
- ordered member manifest;
- per-member name, flags, method, CRC field, compressed/uncompressed declared sizes and external attributes;
- accepted/rejected disposition and reason per member;
- explicit content-validation status.

A dataset ID, filename, content-disposition header or provider MD5 is not a local content digest or a provider-authentication proof.

## 6. Synthetic conformance and negative controls

The first corpus should include:
- a minimal valid stored ZIP;
- a deflate-method metadata case (without content validation);
- absolute and parent-traversal names;
- Windows drive/UNC and backslash names;
- duplicate and case-colliding names;
- file/directory ancestor conflicts;
- symlink and special-file attributes;
- encrypted and unsupported compression flags/methods;
- ZIP64 and multi-disk markers;
- forged size/ratio and member-count limit breaches;
- local-header/central-directory disagreement;
- truncated EOCD/central directory;
- trailing bytes after EOCD.

A benign archive must not be rejected merely because it is compressed. An unsafe/unsupported archive must not be accepted because its central directory parses. Both false acceptance and unnecessary rejection are measured.

## 7. Benchmarks

| Metric | Prospective target |
|---|---:|
| Correct known-answer metadata classifications | 100% |
| Accepted path escapes | 0 |
| Accepted symlink/special-file entries | 0 |
| Policy-limit overruns accepted | 0 |
| Silent filename normalization | 0 |
| Original archive byte/digest preservation | 100% |
| Deterministic manifest replay | 100% |
| Unsupported features silently accepted | 0 |
| Content incorrectly claimed as CRC/decompression-verified | 0 |

Measure benign-corpus throughput, memory usage and operator review cost. Do not compare speed against a full extraction workflow without matching validation scope. The v0.1 metadata-only scanner intentionally does less work than extraction.

## 8. Future controlled extraction gate

A later extraction worker, if justified, must be separate from metadata preflight and run inside an isolated/quarantined environment. It must:
- use a pinned, advisory-reviewed ZIP implementation;
- disable symlink materialization by default;
- use no-follow path operations and verify filesystem containment, not only lexical path strings;
- enforce actual streamed byte and entry limits, not just header declarations;
- use atomic writes and no-overwrite semantics;
- validate CRC and member content before publication;
- reject partial extraction after any error;
- never execute, dynamically import, or automatically parse untrusted content as code.

This document authorizes only defensive parsing and synthetic tests. It does not authorize live provider calls, production credentials, host filesystem extraction, procurement, or changes to operational infrastructure.