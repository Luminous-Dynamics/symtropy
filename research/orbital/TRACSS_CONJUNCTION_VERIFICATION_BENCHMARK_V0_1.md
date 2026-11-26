# TraCSS Conjunction Assessment Verification Benchmark v0.1

**Status:** external diagnostic benchmark integration proposal  
**Tracking:** [#7579](https://github.com/Luminous-Dynamics/symtropy/issues/7579)  
**Claim ceiling:** no dataset acquisition, reproduction, event-detection score, Pc accuracy or operational qualification is claimed by this document.

## 1. Why this benchmark

The NOAA Office of Space Commerce published the TraCSS Dataset for Conjunction Assessment Verification in March 2026. The official page describes an open CC0-1.0 test set and answer keys designed for SSA providers to self-assess conjunction assessment algorithms:

- [Official dataset page](https://space.commerce.gov/dataset-for-conjunction-assessment-verification/)
- [Official user guide, March 2026](https://space.commerce.gov/wp-content/uploads/2026/03/Conjunction_Screening_Testset_Users_Guide.pdf)

The set contains OCM-formatted ephemerides, two CDM-style answer-key CSVs (10 km spherical and SFSH rectangular screening), an object-to-screening-volume mapping and the user guide. The large ephemeris archive is listed at about 20.73 GB on the official web page; the guide says 21.74 GB. The SFSH result CSV is listed as 62.4 MB on the web page, while the guide says 144 MB. Do not assume these are identical dataset revisions or dismiss the differences as unit conversions. Record the actual downloaded byte count and SHA-256 of every artifact, and reconcile filename, release and digest before running a benchmark.

The web page says that download access is currently provided through Google Drive / an access request. The data are not assumed to be present locally or in this repository. Never commit the 20+ GB input archive or derived bulk files to Git.

## 2. What the dataset tests—and what it does not

The user guide says the test set is primarily intended to evaluate:
1. finding the same conjunction events;
2. accurately computing TCA;
3. primary / secondary states and covariance at TCA.

It explicitly says direct Pc comparison is not a key metric unless both tools use the same Pc method. The answer-key Pc field is identified as Alfano 2004.

The guide also explicitly warns that:
- its ephemeris set is not comprehensive;
- the CSieve answer key is expected to find nearly all events but is not guaranteed exhaustive;
- the dataset is a diagnostic self-evaluation tool, not operational validation or formal certification.

Accordingly, a missing answer-key event is a discrepancy against this finite reference corpus, not proof of a missed real-world collision or proof that the corpus is exhaustive. A perfect answer-key match is not a safety case. Event screening, TCA/state estimation, covariance quality, collision probability and mission-risk aggregation are distinct evaluation layers.

## 3. Frozen reproduction configuration

### 3.1 Shared configuration

Use the screening interval:

- Start: `2025-01-01T12:00:00Z`
- Stop: `2025-01-08T12:00:00Z`

The guide requires:
- all-vs-all screening across the supplied objects;
- no conjunction comparisons between candidate OCMs belonging to the same object designator, nor between those candidates and that object's nominal OCM;
- each candidate OCM screened against all other objects;
- each OCM's usable start/stop interval respected;
- OD epochs less than 14 days before the beginning of the screening window;
- no Pc pre-filter;
- only events whose local minimum / TCA is inside the analysis interval reported.

Do not silently interpret a boundary penetration as a conjunction if the local minimum is outside the screening interval.

### 3.2 Round 1 — constant spherical screening

Run the spherical configuration first to reduce screening-configuration differences:

- spherical screening radius: 10 km;
- HBR: 0.5 m for any Pc calculation only;
- all-vs-all under the shared candidate/nominal exclusion rule;
- for a conjunction pair, emit both primary/secondary orderings at the same TCA, as required by the symmetric spherical key.

Compare event set, TCA, relative state and covariance before interpreting Pc.

### 3.3 Round 2 — SFSH rectangular screening

Run as a separate configuration:

- use the per-object UVW/RIC screening-volume and HBR mapping file;
- treat mapping dimensions as half-volumes / maximum absolute component offsets, not full widths;
- an object with no primary screening volume must not be used as primary, but must remain eligible as a secondary;
- preserve primary-relative directionality: A screened against B does not imply B screened against A;
- use the mapped HBR only for any Pc calculations.

Do not merge spherical and SFSH outputs into one answer key or one recall score.

## 4. Data acquisition and source manifest

The harness should consume external file paths and a versioned manifest. The manifest must record, per artifact:

- official source URL and retrieval timestamp;
- exact advertised filename and actual filename;
- actual byte count;
- SHA-256 computed over the downloaded bytes;
- CC0-1.0 license declaration and source attribution;
- user-guide filename, published date and digest;
- archive decompression/tool version;
- dataset release identity and a human-readable reconciliation of page-vs-guide metadata.

No hash, retrieval date, byte count or data revision may be invented or copied from an unrelated run. A missing source artifact or partial archive must produce a typed preflight failure.

Keep raw input files outside Git. Benchmark receipts should refer to content-addressed local artifacts or an access-controlled object store, not embed bulk data.

## 5. Staged implementation plan

### Stage A — source and configuration preflight

Before parsing or screening:
- validate presence and digest of every artifact;
- verify compressed archive integrity before extraction;
- reject duplicate archive paths, path traversal and unsupported file encodings;
- inventory all OCM files and object designators;
- verify candidate OCM / nominal OCM relationships;
- verify usable intervals, OD epochs, frame and time-system metadata;
- verify mapping-file uniqueness and per-object applicability;
- verify that each benchmark run declares exactly one screening configuration.

### Stage B — OCM parsing and source qualification

Parse each OCM while retaining:
- original file digest and path;
- object designator and candidate / nominal role;
- metadata and data epoch;
- declared time system and reference frame;
- usable start and stop;
- covariance presence, coordinate frame, ordering and units;
- force/model metadata where present.

All frame, time-scale and unit transformations must be explicit, versioned and recorded. Never silently convert UTC, TAI, TT or another time system; never infer a covariance frame from column position alone.

### Stage C — candidate-aware all-vs-all screening

Construct the legal screening pair set before propagation:
- remove candidate-to-candidate and candidate-to-own-nominal pairs for identical object designators;
- retain each candidate-to-other-object pair;
- apply the primary eligibility rule for the selected screening configuration;
- preserve permitted directed pairs for SFSH;
- prove deterministic pair-set generation under input permutation.

Emit an independent pair-set receipt before event screening. This makes candidate-exclusion errors independently testable.

### Stage D — answer-key comparison

Normalize outputs to a stable event representation without erasing primary/secondary direction where the key requires it.

Measure:
- event true positives / false positives / false negatives relative to each answer key;
- precision and recall against that key;
- exact missing and extra event identities;
- TCA residual distribution, including median, p95, p99 and maximum;
- primary and secondary position/velocity residuals at TCA;
- relative-state and local UVW/RIC residuals;
- covariance element residuals, symmetry, positive-semidefinite checks and missingness;
- runtime, peak memory, number of legal object pairs, number of propagations and candidate-pair throughput.

Pre-register the event matching identity and TCA tolerance from the cited TraCSS validation methodology before looking at results. Do not choose a matching tolerance after observing the errors.

### Stage E — independent challenger and reproducible receipt

Use a second implementation to challenge:
- normalized source manifests;
- OCM inventory and usable intervals;
- legal object-pair set;
- event normalization and directionality;
- TCA/state/covariance comparison;
- all metrics and summary counts.

The receipt must bind exact Symtropy commit, dataset artifact digests, user-guide digest, parser version, configuration, answer-key digest, benchmark environment, output digest and challenger version.

## 6. Adversarial known-answer corpus

At minimum, test:
- candidate OCM screened against the nominal OCM for the same designator;
- two candidates for the same designator screened against one another;
- candidate-to-other-object screening omitted;
- missing, duplicated, reversed or zero-length usable interval;
- usable interval beginning or ending inside the screening window;
- OD epoch just below, exactly at and just above the 14-day eligibility boundary;
- TCA exactly at both window boundaries and just outside;
- a screening-volume crossing at a boundary while the local minimum lies outside the window;
- a local minimum just inside and just outside the spherical 10 km boundary;
- symmetric spherical event reported in only one direction;
- asymmetric SFSH event incorrectly mirrored;
- object without a primary volume incorrectly omitted as a secondary;
- object without a primary volume incorrectly used as primary;
- mapping interpreted as full-width instead of half-volume;
- Pc prefilter accidentally enabled;
- duplicate or missing answer-key rows;
- wrong HBR mapping;
- malformed time system, frame or units;
- covariance missing, asymmetric, non-PSD or assigned to the wrong frame;
- archive truncated, corrupted, duplicated or path-traversing;
- dataset size/version discrepancy not reconciled;
- an answer-key omission incorrectly treated as proof of real-world universe completeness.

## 7. Acceptance criteria

Prospective targets for this finite diagnostic corpus:
- deterministic parse and normalized output for identical bytes/configuration;
- exact legal candidate-pair set agreement on finite controls;
- zero missed answer-key event rows for each correctly reproduced configuration, with false positives reported separately;
- event precision/recall reported separately for spherical and SFSH configurations;
- TCA, state and covariance residuals reported independently of event detection;
- zero hidden frame, epoch, time-scale or unit transformations;
- independent challenger parity on a frozen sample and all mutation controls;
- complete source provenance for every reported benchmark number;
- diagnostic-only claim ceiling preserved in the receipt and documentation.

These targets do not convert the answer key into exhaustive ground truth or qualify operational collision avoidance.

## 8. Relationship to Symtropy research

This is an external screening/reference corpus for:
- [#7503 — mission-level encounter risk](https://github.com/Luminous-Dynamics/symtropy/pull/7503);
- [#7541 — per-portfolio post-action evidence](https://github.com/Luminous-Dynamics/symtropy/pull/7541);
- [#7533 — canonical joint-action identity](https://github.com/Luminous-Dynamics/symtropy/pull/7533);
- [#7542 — paired finite-support variance kernel](https://github.com/Luminous-Dynamics/symtropy/pull/7542);
- [#7578 — multi-portfolio paired comparison](https://github.com/Luminous-Dynamics/symtropy/pull/7578);
- [#7525 — paired trajectory counterfactual research](https://github.com/Luminous-Dynamics/symtropy/issues/7525);
- [#7535 — post-action evidence bridge](https://github.com/Luminous-Dynamics/symtropy/issues/7535).

The dataset tests screening outputs and state/covariance-at-TCA comparisons. It does not validate complete mission unions, non-additive manoeuvre consequences, the true continuous orbital distribution, or a new Pc estimator.
