# Preservation Benchmark Interoperability — SPECstorage and SNIA Emerald

**Related programme:** [Preservation Service Tournament #7582](https://github.com/Luminous-Dynamics/symtropy/issues/7582)  
**Purpose:** reuse established performance and energy measurement discipline while adding preservation-specific acceptance evidence.

## 1. Why interoperate instead of inventing another headline benchmark

The preservation tournament should not claim to replace:
- **SPECstorage Solution 2020**, which provides standardized application-style storage workloads and throughput/response-time metrics for file-serving/storage configurations.
- **SNIA Emerald**, which provides standardized power measurement procedures for data-center storage devices, including active/idle states, taxonomy, test configuration, instrumentation, measurement interval and stability.

These benchmarks measure important parts of the problem. They do not, by themselves, establish exact preservation recovery, authenticity, independent reader replacement, representation understandability, physical-media lifetime, or migration fidelity.

The preservation-specific overlay should therefore be reported alongside standard benchmark results, never substituted for them.

## 2. SPECstorage mapping

SPEC identifies SPECstorage Solution 2020 as its current storage benchmark suite and provides distinct workloads including AI image, genomics, VDA, software build and EDA blended. SPEC's own results guidance says comparisons must use the same workload; different workloads generate different metrics and are not directly comparable.

Use these rules:

1. When a candidate system supports an eligible SPEC configuration, run the official workload and retain the unmodified SPEC workload/version, run configuration, result artifacts and published metrics.
2. Compare only the same SPEC workload and compatible benchmark version/configuration. Do not compare AI_Image JOBS against VDA STREAMS or an archive restore metric.
3. Do not relabel a SPEC throughput/response result as preservation throughput.
4. Do not embed licensed SPEC traces, binaries or derived material into Luminous fixtures unless the relevant licence permits it. Link to the official suite and results.
5. Add the preservation overlay only after the standard workload completes: independent expected digests, accepted-object accounting, manifest/provenance validation, reader dependency evidence and preservation objectives.

### Workload crosswalk

| Preservation workload | SPECstorage relationship | Treatment |
|---|---|---|
| Selective restore / small objects | May share I/O-pattern characteristics with VDA, EDA or software-build traces | Run the official SPEC workload separately; add a distinct preservation replay with fixed object truth |
| Genomics-style archives | SPEC genomics is a useful application workload comparator | Compare only to SPEC genomics results; do not assume the corpus is an archival-preservation workload |
| AI-image archives | SPEC AI_Image is a candidate application-throughput comparator | Keep application benchmark result separate from exact archive recovery |
| 1 PB cold full restore | No direct equivalent established by the named SPEC workloads | Keep as a separate service-level sequential restore benchmark |
| Integrity scan/repair | No direct one-to-one equivalent established | Use preservation-specific deterministic fault and fixity workload |
| Offline reader replacement | Not established by SPECstorage workloads | Use Offline Trust Closure and reader-replacement protocol |
| Cross-medium migration | Not established by a storage-server throughput result | Measure source read, destination write/readback, exact fidelity and provenance |

## 3. SNIA Emerald energy measurement mapping

Use the applicable current SNIA Emerald specification revision and taxonomy for any storage-device power claim. Every trial receipt must pin:
- specification name and exact revision;
- device/system taxonomy and tested capabilities;
- complete test configuration and power boundary;
- instrument identity and calibration evidence;
- active/idle state and workload phase;
- measurement interval, stability test and raw samples;
- average power and the metric defined by the relevant specification.

Do not invent a conversion that makes an Emerald device result equivalent to end-to-end archive-service energy. Report two separate energy views where possible:

1. **Storage subsystem power/efficiency:** measured under the applicable SNIA Emerald procedure and taxonomy.
2. **Preservation service energy:** whole service boundary from request admission through independently accepted output, including readers, compute, staging, network, verification and migration where in scope.

Then calculate preservation-specific ratios from the whole-service meter:
- joules per independently accepted TiB;
- joules per independently accepted object;
- whole-service energy per completed migration;
- energy and time for repair versus full restore.

Keep idle baseline, active interval and whole-service energy separately visible. Do not subtract or normalize idle energy unless the protocol and measurement boundary explicitly justify it.

## 4. Preservation overlay metrics

The core metric remains:

verified throughput = independently accepted payload bytes / end-to-end elapsed time

Additional dimensions:
- exact-byte and object acceptance;
- false acceptance and abstention;
- manifest/provenance/signature status;
- full stage evidence and deadline;
- first accepted byte latency and p50/p95/p99 selective-object latency;
- independent reader agreement and transitive dependency closure;
- migration semantic/fixity fidelity;
- preservation objective results;
- service energy and meter boundary.

A standard SPEC or SNIA pass never implies a preservation pass. A preservation receipt never implies SPEC or SNIA conformity unless the relevant official procedure was actually run and its result is preserved.

## 5. Anti-gaming and reproducibility

- Freeze the official benchmark version, workload, configuration, system topology and run artifact IDs.
- Freeze preservation payload truth and expected digests outside candidate control.
- Report timeouts, failed restores, incomplete energy readings, failed migration and abstention.
- Preserve raw meter samples, calibration and uncertainty instead of only a computed ratio.
- Do not compare cross-workload, cross-version or cross-taxonomy results.
- Report both the standard result and preservation overlay; no weighted score may erase a failure.

## 6. References

- SPEC Storage benchmark suite: https://www.spec.org/storage/
- SPECstorage Solution 2020 results and same-workload comparison guidance: https://open.spec.org/storage2020/results/
- SNIA Emerald standards and specification index: https://www.snia.org/tech_activities/standards/curr_standards
- SNIA Emerald program: https://www.snia.org/emerald


## 7. Current public comparison context (reviewed 27 November 2026)

The latest published SPECstorage Solution 2020 Q4 2026 page currently lists:
- WEKAIO WEKApod3, 31-server AI_IMAGE run: 9,000 AI jobs, 880,157 MB/s result field, 1.8 PiB usable capacity; published 12 November 2026.
- H3C UniStor Polaris UltraStor EDA_BLENDED run: 4,500 job sets, 32,225 MB/s result field, 104.79 TiB capacity; published 10 November 2026.
- The Q4 page lists no Genomics, software-build or VDA result entries at review time.

These are official SPEC workload-specific results; they are context for application-style storage scale, not preservation-service baselines. Do not convert either result into archive restore throughput or compare across AI_IMAGE and EDA_BLENDED.

The SNIA Emerald programme page currently identifies Power Efficiency Measurement Specification v4.0.0, adopted 3 July 2020, as the revision it describes and makes all versions available. Pin v4.0.0 when performing the corresponding procedure and record the exact taxonomy/configuration. SNIA's page describes ISO/IEC 24091:2019 as technically equivalent to Emerald v3.0.3, so do not conflate that ISO edition with Emerald v4.0.0.

A contemporaneous technology context:
- Fujifilm announced US availability of LTO-10 40 TB cartridges on 2 April 2026; published native transfer rate is up to 400 MB/s and advertised archival life is 30+ years under recommended conditions. Its WORM option is listed for the 30 TB variant, not the 40 TB cartridge.
- Microsoft's 18 February 2026 Project Silica publication reports accelerated-ageing evidence suggesting at least 10,000 years for written borosilicate glass. The programme describes a research demonstrator and says it continues to consider learnings; this is not a measured 10,000-year operational archive service.

Sources:
- SPECstorage Q4 2026 results: https://www.spec.org/storage2020/results/res2026q4/
- SNIA Emerald programme: https://www.snia.org/emerald/programs
- Fujifilm LTO-10 40 TB announcement: https://newsroom-us.fujifilm.com/press-releases/press-release-details/2026/FUJIFILM-North-America-Corporation-Announces-U-S--Availability-of-FUJIFILM-LTO-Ultrium-Generation-10-40TB-Data-Cartridges-04-02-2026/default.aspx
- Microsoft Project Silica research update: https://www.microsoft.com/en-us/research/blog/project-silicas-advances-in-glass-storage-technology/
