# FJSPLib indexed corpus

**Upstream:** https://github.com/ScheduleOpt/benchmarks  
**Pinned upstream Git revision:** `8a6e36ef31263d36975e56fb481e0ba04e188d6f` (2026-10-22)  
**Index:** `fjsplib-index-2026-10-22.tsv`  
**Index Git blob:** `ffca6cdf55103205bcca73746324f94691ceeb87`

## Inventory

This index records **452 logical FJSPLib instances** across **854 upstream text-file representations**:

| Class | Logical instances | FJSP text | FAJSP text |
|---|---:|---:|---:|
| Classic flexible job shop | 138 | 138 | 138 |
| Machine-independent flexible job shop | 264 | 264 | 264 |
| Flexible assembly job shop | 50 | 0 | 50 |
| **Total** | **452** | **402** | **452** |

The 402 FJSP / FAJSP common cases have paired encodings. The 50 assembly cases have only FAJSP encodings. Count the 452 logical IDs, not the 854 file rows, when reporting the corpus size.

## Fields

Each row records a logical instance ID, problem class, source family, source instance ID, variant, text format, upstream relative path, upstream Git blob SHA-1 and byte count. Git blob SHA-1 is a source identity check, not a cryptographic attestation of benchmark semantics. The index is pinned to a Git commit so the metadata can be regenerated.

## Scope and licensing

The ScheduleOpt/benchmarks root repository identifies CC-BY-SA-4.0. This index contains metadata and source references, not copied benchmark source files. Retain attribution to ScheduleOpt and the original instance families. Verify any additional per-instance terms or third-party provenance before mirroring raw benchmark content or redistributing modified instances.

## CI contract

CI verifies:
- exactly 854 data rows and 452 logical IDs;
- 402 FJSP and 452 FAJSP representations;
- all three class totals by format;
- the checked-in index Git blob hash.

CI does not download or parse all 854 source files. The current adapter parses the eight frozen Fattahi smoke fixtures, checks the paired Fattahi5/10/20 semantics, and runs the independent exact/CP-SAT known-answer check only for Fattahi1. Full-corpus content retrieval, normalization and engine comparison belong to FACTORY-FJSPLIB-TOURNAMENT-001 #5528.

