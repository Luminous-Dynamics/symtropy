# Preservation Service Tournament — v0.1

**Programme:** PRESERVATION-SERVICE-TOURNAMENT-001  
**Tracking:** [Symtropy #7582](https://github.com/Luminous-Dynamics/symtropy/issues/7582)  
**Parent compiler:** [Symtropy #7482](https://github.com/Luminous-Dynamics/symtropy/issues/7482)  
**Recovery package:** [Symtropy #7571](https://github.com/Luminous-Dynamics/symtropy/issues/7571)

## 1. Purpose

This protocol defines a common workload and scoring boundary for evaluating complete preservation services. It is deliberately narrower than the Physical Memory Compiler: the compiler generates and filters candidate architectures; this benchmark asks whether a candidate can actually fulfil the same frozen recovery obligations as its comparators.

The benchmark's unit of success is **independently accepted payload**. Device bytes read, nominal capacity, decoder confidence, simulated retention and candidate-reported success are not accepted output.

The v0.1 corpus at research/physical-storage/fixtures/preservation_service_workloads_v0_1.json is synthetic and normative only for workload semantics. It contains no measured performance, cost, energy, retention or physical failure probabilities.

## 2. System boundary

A measured restore begins when an authorized request is admitted and ends only when the requested objects have been reconstructed, delivered to the declared boundary, checked against independent expected digests, checked for manifest/provenance consistency and accepted.

Include all in-scope time and resources:
- operator approval, media mounting, reader bootstrap and key/bootstrap recovery;
- cartridge/tape staging, disk spin-up, queueing, seek, random access and cache misses;
- sensor readout, calibration, retries, ECC, alignment, decoding and reconstruction;
- object packaging, decompression, decryption and manifest/fixity validation;
- network transfer, egress, staging storage and destination verification;
- migration writes, destination readback, preservation-event recording and independent acceptance.

A report that omits a required stage is marked **incomplete**, not a faster successful restore. Define separately whether ingest, routine integrity scans, media refresh and migration are included in lifecycle cost/energy.

## 3. Workload families

The frozen v0.1 workload set covers:

1. **Cold full restore** — archive-scale sequential restoration under an explicit deadline.
2. **Selective restore** — random small-object access with concurrency and tail-latency reporting.
3. **Integrity scan and repair** — locate planted corruption, recover or abstain, then verify exact bytes.
4. **Offline reader replacement** — withhold the original vendor runtime and network access; require independently built recovery paths.
5. **Cross-medium migration** — source read, destination write/readback, exact payload comparison and preserved migration provenance.
6. **Compound outage** — site/power/reader/network/key/calibration dependencies, including common-cause and unknown-dependency cases.

Every workload freezes payload profile, byte count, object count, entropy/compression/encryption policy, access pattern, request fraction, concurrency, deadline, reader/network budget, fault schedule, offline/vendor-runtime boundary and acceptance obligations. Workload schema evolution must be versioned; never silently change the meaning of a frozen workload ID.

## 4. Matched comparison

Each arm receives identical payload bytes, workload trace, required service profile, deadline, fault schedule, verification method and hardware/network/compute budget. If a factor cannot be equalized, create a separate explicitly resource-normalized comparison and report the difference.

Comparison arms, as supported by the evidence:
- incumbent operator-selected tiering;
- single-medium HDD, tape, SSD/NVM and glass;
- conventional replication/tiering;
- physically diverse hybrid media;
- pinned conventional ECC/layout;
- Symthaea-selected code/layout and compiler-generated designs;
- exhaustive exact solutions on small instances as an oracle upper bound, never as a deployable baseline.

Apply identical compression, deduplication, encryption, caching and object packaging policies, or report each as a separate controlled factor. Count parity, metadata, calibration, controls, spares, replicas and capsule overhead against net payload capacity. Do not compare a gross-density glass figure with formatted/usable disk capacity or a native tape rate with a verified end-to-end archive restore.

## 5. Metrics

Keep a vector of metrics; do not hide hard failures inside a weighted score.

### Correctness
- requested, reconstructed, delivered and independently accepted bytes/objects;
- object and whole-payload digest equality;
- missing, extra, duplicate, stale, unrecoverable and rejected objects;
- false acceptance, false rejection, abstention and incomplete-run counts;
- recovery outcome per workload and fault stratum.

### Service
- full end-to-end elapsed time and its stage decomposition;
- verified throughput = independently accepted payload bytes / end-to-end elapsed seconds;
- time to first accepted object and full completion;
- p50/p95/p99 selective-read latency;
- deadline attainment; unfinished runs are right-censored, not dropped.

### Resources and economics
- energy for ingest, preservation, scan, restore, verification and migration, with meter boundary and uncertainty;
- cost components with currency, price date, scope, evidence class and uncertainty;
- cost per exactly accepted TB restored;
- cost per contract-qualified TB-year only when the service contract defines the eligible capacity, qualification state, audit cadence, gap handling and lifecycle cost boundary;
- operator time, media/reader utilization, storage staging, network/egress, consumables, repair and rework.

Do not infer that one successful checksum verifies every instant in a long retention interval. A contract-qualified TB-year is a service-accounting measure whose inclusion rules must be published; it is not direct observation of centuries of survival.

### Portability and independence
- offline bootstrap and recovery success;
- time and resources needed to replace the reader;
- vendor runtime, network, cloud, license-server and key dependencies;
- source/build/firmware/model/calibration lineage for every reader and decoder;
- shared libraries, toolchains, operators, custody, site, power and network failure domains;
- migration byte fidelity, manifest integrity and preservation-event lineage;
- common-cause cut sets and unresolved dependency coverage.

Hash/fixity, authenticity, exact recovery and operational independence are different claims. A second binary or second programming language is not an independence proof.

## 6. Archive-scale reference workload

The 1 PB / ten-day example is a useful lower-bound arithmetic check:

10^15 bytes / 864,000 seconds = 1.1574 GB/s (decimal units).

Three ideal 400 MB/s LTO-10 streams sum to 1.2 GB/s before mounts, seeks, retries, decoding, fixity, staging or networking. This arithmetic does not qualify a 1 PB library restore. The actual workload must measure all stages and show headroom against its SLA.

## 7. Anti-gaming and statistics

- Split by specimen, batch, reader, site, media lot and independent workload trace. Do not randomly split neighboring voxels or reads from the same specimen across training and test.
- Freeze input bytes, scorer revision, candidate resource budget, optimizer evaluation budget, timeouts and seeds before the run.
- Publish every run, including timeouts, faults, failed migrations, failed restores, abstentions and incomplete measurements.
- Report per-medium, per-workload and per-fault-stratum values with sample count and uncertainty. Do not pool incomparable media or evidence classes.
- Use paired comparisons where possible. Physical replicate count is based on independent specimens/units, not synthetic seeds.
- Seal evaluation labels and expected digests from the candidate. The independent scorer owns acceptance.
- Include negative controls for digest mismatch, stale calibration, omitted/duplicated objects, wrong key, tampered manifest, decoder substitution, offline network access, shared site/power failure, media mismatch and lost provenance.
- Keep deterministic synthetic, published reproduction, model-derived, HIL and physical measurement results separate.

## 8. Qualification and claim ladder

- **W0 — contract:** schema and semantic validation, canonicalization, unit checks and negative controls.
- **W1 — synthetic:** deterministic known-answer workload replay and independent scorer.
- **W2 — published reproduction:** repeat published device/channel results under their exact source-defined setup; report definitions and deviations.
- **W3 — controlled system benchmark:** physical media/readers, calibrated meters, frozen workload and independent payload truth.
- **W4 — reader replacement/migration:** offline, independently implemented readers/decoders, exact payload and provenance acceptance.
- **W5 — service qualification:** repeated independent units/lots, full restore SLA, lifecycle accounting, common-cause and retention evidence under a declared service contract.

W0/W1 do not establish physical performance. W2 is not an independent new physical result. W3/W4 do not establish long-term retention. W5 applies only to the exact qualified service, media, reader, process, environment and contract scope.

## 9. Prospective acceptance gates

- 100% exact bytes for all finite known-answer positive cases.
- Zero false acceptance on the frozen corruption, stale-evidence, manifest, key and provenance negative controls.
- 100% accounting of requested, accepted, missing, duplicate and unrecoverable objects.
- 100% of runs either report the full declared system boundary or are marked incomplete.
- Independent offline reader replacement passes only when both decoder outputs are byte-identical, match the scorer-held expected digest and satisfy the declared dependency review.
- Migration passes only when source and destination payload bytes, manifests and required preservation lineage all pass.
- No physical cost, energy, throughput, retention or SOTA target is set before a measured incumbent baseline and uncertainty budget exist.

These are finite-corpus engineering targets, not empirical claims or standards requirements.

## 10. Ownership and interfaces

- **Symtropy #7482:** service contract, design-space and candidate decisions.
- **Symtropy #7477/#7567:** glass and DNA observations, physical channel provenance and calibration.
- **Symtropy #7571:** capsule, decoder/reader package, representation information and migration record.
- **Symthaea #13712/#13662/#13729/#13734:** ECC, spatial optimization, DNA baselines and independent recovery scorer.
- **Mycelix #5762:** common-cause and transitive dependency assessment.

The optimizer proposes; the independent scorer accepts; the physical owner qualifies the channel; common-cause qualification remains external to the optimizer. No benchmark receipt grants physical actuation, archive deletion, key release or preservation authority.

## 11. References and source status

- Microsoft Project Silica, Nature (18 February 2026): https://www.nature.com/articles/s41586-025-10042-w
- Lawson et al., property-driven glass screening, Optica (April 2026): https://www.microsoft.com/en-us/research/publication/property-driven-analysis-of-glasses-for-data-storage-via-femtosecond-laser-writing/
- Seagate Mozaic 4+ production announcement (3 March 2026): https://investors.seagate.com/news/news-details/2026/Seagate-Delivers-Industrys-Highest-Capacity-Hard-Drives-with-Next-Generation-Mozaic-4/default.aspx
- Fujifilm LTO-10 40 TB specification (2026): https://asset.fujifilm.com/www/us/files/2026-04/f504b921d82ca915758da2ea1b660c81/FNAC-DSS_LTO-10_40_TB_Tape_Spec_Sheet-02.2026.pdf
- Six-codec DNA-storage comparison, Nature Communications (14 March 2026): https://doi.org/10.1038/s41467-026-70548-3
- Polus/SeqFormer, Bioinformatics (3 August 2026): https://doi.org/10.1093/bioinformatics/btag563

Published lab results, in-vitro results, vendor production statements, device specifications and prospective projections must remain separate in every generated report.


## 12. Reference receipt evaluator — v0.1

A dependency-free Rust scoring kernel now exists at [preservation-service-reference](preservation-service-reference/README.md). It scores independently produced receipts against a frozen WorkloadContract and distinguishes candidate reconstruction, delivery and independently accepted bytes. It includes hard gates for exact requested object/byte accounting, digest/manifest/provenance evidence, independent reader-path IDs and agreement, deadline, energy metering, required stage evidence, false acceptance, extras and duplicates.

The scorer returns separate Pass, Fail, Incomplete and NotEvaluated dispositions. A candidate full-success claim that the independent evidence cannot support is a hard failure; an honest abstention or absent evidence remains incomplete. Accepted throughput is computed from independently accepted bytes divided by end-to-end elapsed time. Stage timings remain separately reported because parallel service stages may overlap.

The synthetic receipt KAT corpus and schema are:
- [preservation_service_receipts_v0_1.json](fixtures/preservation_service_receipts_v0_1.json)
- [preservation_service_receipts_v0_1.schema.json](preservation_service_receipts_v0_1.schema.json)

The ten receipt cases currently encode one positive pass, hard-failure negative controls, honest incomplete outcomes and a no-run case. A separate JavaScript semantic oracle matched all ten fixture dispositions. The Rust unit tests are authored but not executed here: Cargo, rustc and rustfmt are unavailable. Full Draft 2020-12 validation, a second independently implemented production scorer and measured service runs remain pending. This is contract/reference evidence only, not an end-to-end physical storage result.
