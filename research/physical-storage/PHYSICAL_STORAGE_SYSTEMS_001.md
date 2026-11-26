# Physical Information Storage Systems — Research Protocol v0.1

**Programme:** PHYSICAL-STORAGE-SYSTEMS-001  
**Owner:** Symtropy physical/process twin and qualification  
**Companion:** Symthaea [PHYSICAL-STORAGE-CODING-001](https://github.com/Luminous-Dynamics/symthaea/issues/13630)  
**Tracking:** [Symtropy #7447](https://github.com/Luminous-Dynamics/symtropy/issues/7447)  
**Snapshot date:** 2026-11-25

## 1. Thesis: optimize recoverable information over its required lifetime

A storage medium is not useful merely because it can physically represent many bits. A useful system must write the intended payload, preserve it under a declared environment and lifetime, read it with an available instrument, reconstruct it exactly, and prove that the returned bytes are the intended bytes.

The system boundary is therefore:

`payload → framing/metadata → ECC and interleaving → physical layout → write process → medium → read process → calibration → decoding/ECC → payload digest verification`

The design objective is the **recoverability frontier**: maximize exact recoverable payload over a target lifetime while respecting capacity, latency, energy, yield, environmental, cost and reader-availability constraints.

Do not reduce this to one weighted score. Hard failures, uncorrectable regions, unsupported physical models and unknown outcomes remain visible.

## 2. 2026 technology baseline

The machine-readable register is adjacent at `fixtures/industry_baselines_v0_1.json`. The figures below preserve the source's own metric and system boundary.

| Technology | Status at snapshot | Reported reference point | What it is good at / what it does not establish |
|---|---|---|---|
| Femtosecond-written glass (Silica, fused silica) | Published end-to-end research demonstrator | 1.59 Gbit/mm³; 301 layers; 4.8 TB per 120 × 120 × 2 mm platter; 25.6 Mbit/s per beam; 10.1 nJ/bit | High-density long-term archive candidate. One-beam write rate is far below tape-drive streaming rates; full reader, writer and cost deployment must be included. |
| Glass phase voxels (borosilicate) | Published research regime | 0.678 Gbit/mm³; 258 layers; 2.02 TB per platter; 18.4 Mbit/s single-beam regime; 65.9 Mbit/s demonstrated with four beams; 8.85 nJ/bit | Lower-cost glass and simpler read/write path in the paper; accelerated ageing suggests >10,000-year voxel lifetime, which is an extrapolation, not a direct 10,000-year observation. The paper's 16+ beam thermal result is a projection, not a demonstrated write system. |
| Seagate Mozaic 4+ HAMR | Production ramp with hyperscale customers | Up to 44 TB per HDD; October 2026 call says ramp continuing with two leading cloud providers and further qualifications underway | Industrial nearline capacity and production deployment. This is the relevant shipping baseline, not the roadmap below. |
| Seagate Mozaic 5 | Roadmap / prequalification | More than 50 TB per drive targeted; qualification shipments expected toward end of calendar 2027; an 8 TB-per-disk laboratory demonstration was reported | Vendor development and lab figures, not production-available capacity. Do not benchmark as a shipped drive. |
| WD 40 TB UltraSMR ePMR | Shipping/ramping, reported August 2026 | Up to 40 TB per HDD | Industrial nearline capacity. UltraSMR trades some rewrite flexibility for density; workload compatibility matters. WD's first-generation 44 TB HAMR was still forecast for H1 2027 in the cited August report. |
| LTO-10 tape | Current format/product family | 40 TB native per cartridge; up to 400 MB/s native; up to 100 TB at 2.5:1 compression | Removable, offline, high-capacity archive. Compatible drives support hardware encryption; Fujifilm's 2026 product notice lists the WORM option on its 30 TB LTO-10 variant, not the 40 TB cartridge. Cartridge capacity is not comparable to a glass volume density figure. |
| Micron 6600 ION QLC SSD | Commercial product family | 245.76 TB usable / 256 TB raw in E3.L | Very high-capacity flash and low-latency access; a different cost, endurance, power and access tier from tape or glass archive. |

### Important arithmetic sanity check

The fused-silica birefringent regime reports 25.6 Mbit/s per beam. The borosilicate phase-voxel regime reports 18.4 Mbit/s in its single-beam configuration and 65.9 Mbit/s with four beams; thermal modelling suggested 16+ beams may be possible, but that is not a demonstrated system result. LTO-10's 400 MB/s native headline corresponds to 3.2 Gbit/s, about 49× the demonstrated four-beam glass rate and 125× the fused-silica single-beam rate. These are **channel-rate comparisons only**, not whole-system or same-workload verdicts: reader throughput, drive interfaces, seek/access semantics, multibeam scaling and energy boundaries must be measured separately.

Likewise, glass volumetric density, HDD areal density, tape cartridge capacity and SSD package capacity must remain separate dimensions. No direct ranking may be made by comparing those raw numbers.

## 2.1 Glass substrate selection is a Pareto problem

The April 2026 Optica material-screening study evaluated commercial glasses for multibit phase-voxel writing. In its tested set, SCHOTT K10 had the highest reported write efficiency (0.115 bit/nJ/voxel), while BOROFLOAT 33 had the highest reported voxel quality (1.76 bit/voxel). The same material cannot be declared the universal winner: write efficiency and state quality are separate objectives, and both can trade against net density, readout error, retention, batch variability and accepted-volume yield.

The source-linked register is `glass_material_candidates_v0_1.json`, governed by `glass_material_screen_v0_1.schema.json`. It records SCHOTT's BOROFLOAT 33 supplier properties separately from the laser-writing research results. Thermal diffusivity, exact experimental lot identity, pulse-timescale thermal response and measured spatial defect maps remain unresolved rather than being imputed from nominal datasheet values. The study-specific thermal-diffusivity correlation (Pearson r = 0.78) is a useful hypothesis for substrate screening, not a universal causal law.

Source: https://www.microsoft.com/en-us/research/publication/property-driven-analysis-of-glasses-for-data-storage-via-femtosecond-laser-writing/

## 3. Paradigm shifts to test

### 3.0 From storage density to verified information-years

A medium should be compared by the exact payload that can be restored under a declared environment and service lifetime, not by a raw capacity headline. Add service-level metrics such as verified TB-years per rack volume, cost per verified TB-year, energy per successfully restored TB, and restore-time SLA. These metrics require full device/library, reader, decoder, migration and environmental boundaries; until public cost and service-life data exist, record them as missing rather than estimating them.

### 3.1 From density to exact information survival

Primary outcome: exact payload recovered and digest-verified after a declared stress history. Report raw medium errors, post-ECC errors, uncorrectable blocks and local damage separately.

### 3.2 Joint physical and code design

Choose voxel/bit pitch, layer spacing, calibration marks, physical placement, code family, interleave geometry and redundancy together. The error model must include spatial correlation; independent bit flips alone are insufficient.

### 3.3 Self-calibrating and self-describing media

Distribute calibration references and format metadata across independent regions. The archive package must carry enough versioned information to decode without relying on a proprietary cloud service or a single undocumented reader calibration.

### 3.4 Local failure containment

Partition the medium into independently recoverable regions. A damaged tile should not silently contaminate unrelated blocks. Define repairable, degraded and irrecoverable states, and expose missing regions explicitly.

### 3.5 Reader diversity and future migration

Qualify at least two independent decode implementations or a portable reference decoder. Test reader replacement, calibration transfer, and migration to a new reader. Media durability without decoder durability is not archival durability.

### 3.6 Heterogeneous storage, selected by mission

Use SSD for latency-sensitive hot data, HDD for economical nearline scale, tape for removable offline archives and glass only where its retention / tamper / environmental properties justify its write and read costs. A cross-media manifest and independent payload digest can provide resilience without pretending one material wins every tier.

## 4. Division of engineering responsibilities

### Symtropy

Own the physical/process twin and evidence contract:
- geometry, coordinate frames, material identity and process history;
- manufacturability, tolerances, registration and spatially correlated defect maps;
- read/write alignment and instrument geometry;
- thermal/mechanical exposure only where the selected model actually supports it;
- predicted-versus-measured observation lineage and uncertainty;
- replayable process and metrology receipts.

### External qualified physics

Optical propagation, laser–glass energy deposition, nonlinear glass response, voxel formation, polarization/phase readout, micromagnetics and magnetic recording-head/media interactions require suitable specialist models. Use pinned and independently qualified external solvers or narrowly scoped reduced-order models.

**Do not present Symtropy's N-dimensional rigid-body dynamics as a Maxwell, FDTD, laser-material, LLG or micromagnetic solver.** A process twin may orchestrate qualified models and evidence without pretending to implement their physics.

### Reuse from current magnetic work

The current magnetic programme provides useful patterns: exact-grid field metrics, coordinate/frame identity, metrology covariance, analytical finite-magnet references, process variation, and solver qualification. See Symtropy #7310, #7332/#7333, #7334, #7365/#7366, #7368/#7369/#7370, #7376/#7378 and #7444/#7445.

Those projects concern magnetic field-function components and solver evidence. They do not yet implement data-recording physics. A future magnetic-memory tranche must separately model media grain statistics, anisotropy, exchange/demagnetizing fields, thermal activation, write-head field/heat profile, read-head response, track interference, transition jitter and read-channel noise.

## 5. Benchmark definitions

Every result must declare its medium, workload, payload, geometry, environment, reader/writer, calibration, code, system boundary and evidence class.

| Dimension | Required measurements |
|---|---|
| Density | Gross physical density; net payload density after ECC, calibration, metadata, spares and redundancy; capacity per medium |
| Throughput | Write/read per beam/channel and whole system; sustained and burst; sequential and random separately |
| Energy | J per written bit and J per successfully recovered payload bit; state whether laser, actuator, reader, controller and cooling are included |
| Integrity | Raw BER/symbol errors; post-ECC BER; exact block and whole-payload recovery; uncorrectable regions; digest mismatches |
| Spatial robustness | Burst length/orientation; correlated damage; layer loss; tile dropout; cross-talk; registration drift; spatial error maps |
| Latency | Access time, locate time, calibration time, read latency, decode latency, recovery time |
| Retention | Stress protocol, measured degradation, fitted acceleration law, uncertainty, confidence interval and extrapolation limit |
| Endurance | Rewrite cycles where applicable; read disturb and handling damage |
| Production | Yield, repeatability across builds/lots, accepted-part cost, process time, material use and lifecycle impact |
| Archival usability | Reader replaceability, decoder portability, format self-description, key/manifest availability and migration path |

Do not convert a synthetic error distribution into a physical failure probability. Do not claim 10^-15 error rates from small sample sizes. Publish raw counts and confidence bounds.

## 6. Challenge targets

These are gated research targets, not current results or forecasts.

1. Reproduce the Silica paper's reported metrics and definitions to within 5% before claiming a new optical result.
2. At equal net payload density and error target, pursue either 2× higher write throughput per beam or 2× lower energy per successfully recovered payload bit. Measure both as separate Pareto objectives.
3. Measure the full recoverable-payload-efficiency / fault-tolerance Pareto frontier; do not impose 90% on every architecture. The first 4 × 4 × 3 algorithmic demonstrator uses a linear MDS(6,4) stripe, four control tiles and two spares: 224 payload bytes in 384 physical tile bytes (58.33% end-to-end payload efficiency), intentionally trading density for recovery after a whole one-third-volume layer loss. For a 301-layer-class medium, evaluate higher-rate interleaving: a hypothetical 10-data + 1-parity stripe has 90.91% code rate before control, metadata and spare overhead and can tolerate one known erasure per stripe if every layer contributes at most one shard. This is a design calculation, not a measured result; report net efficiency after every overhead.
4. Recover unaffected regions exactly after bounded spatially correlated damage and identify every lost region.
5. Make the full archive package independently decodable: payload, format version, code parameters, calibration references, known-answer vectors, digest and provenance manifest.
6. Treat >10,000-year retention as an extrapolation target to be reproduced and stress-tested, not as an achieved service-life guarantee.

First establish repeatability and measurement uncertainty. Only then freeze physical improvement thresholds.

## 7. Qualification ladder

- **P0 — source and claims:** verify paper figures, units, geometry, definitions, product status and source dates; add negative controls.
- **P1 — software reference:** deterministic layout arithmetic, receipt validation and source-pinned baseline reproduction.
- **P2 — channel/coding:** independent classic ECC baselines and spatially correlated synthetic fault masks; exact payload digest checks.
- **P3 — accessible physical pilot:** 2D marks on inexpensive glass plus optical capture/decoder. This tests format and readout robustness only; it is not a reproduction of 3D femtosecond glass storage.
- **P4 — specialist lab:** calibrated multi-layer laser-written voxels, instrumented write/read, independent metrology and repeated samples.
- **P5 — archival qualification:** accelerated ageing with a stated model, reader replacement, environmental tests, repeated lots and matched workflows against tape/HDD.

## 8. V0.1 receipt and reproducibility contract

A run receipt must bind:
- payload digest and byte length;
- format/layout version and logical-to-physical mapping;
- code family, parameters, interleaver and metadata overhead;
- medium specimen, material batch, geometry and process identities;
- writer/reader/instrument identity and calibration record;
- environmental/stress history;
- physical map, raw read observations and output digests;
- decoder version, ECC result, recovered byte length and final payload digest;
- uncertainty, exclusions, evidence class and source/fixture identities.

A hash-shaped string is an identity field, not proof of authenticity. A solver's own error estimate, a comparison between solvers and measurement uncertainty must stay separate.


## 9. Executable linear-MDS layout reference

The companion Symthaea draft [PR #13631](https://github.com/Luminous-Dynamics/symthaea/pull/13631) now includes a standalone zero-dependency Rust reference at `research/physical-storage/mds-reference/` and a machine-readable layout/fault fixture.

The synthetic 4 × 4 × 3 layout uses 28 data tiles, 14 parity tiles, four redundant control tiles and two spares. Its seven systematic [6,4,3] linear MDS stripes use GF(256); each stripe has two shards in each of the three physical layers. Any two known shard erasures are recoverable. This code is explicitly described as a **linear MDS array code**, not as a standards-conformant Reed–Solomon/GRS implementation. The standard RS tournament remains future work.

The reference is designed to recover the exact 224-byte payload after any one of the three whole-layer dropouts and a localized 2 × 2 tile cluster. It must reject unflagged byte substitutions by final SHA-256 comparison, and fail closed when a stripe exceeds two erasures or the redundant format/calibration controls are unavailable.

Independent Python algebra/layout checks covered all 15 two-erasure patterns, three whole-layer dropouts, the localized cluster, an over-budget negative control and the payload digest. These are independent mathematical checks, **not Rust compilation or Cargo test results**. The current execution environment has no Rust toolchain; exact-head rustfmt, Clippy and Cargo tests remain pending.

The toy has 58.33% end-to-end tile payload efficiency because it deliberately tolerates loss of one-third of the physical tile volume. It is a proof-of-recovery fixture, not a density result. For a 301-layer-class medium, a hypothetical 10+1 stripe has 90.91% code rate before physical overhead and can tolerate one known erasure per stripe if the interleaver ensures at most one symbol from each stripe is placed in any one layer. This design calculation is not a measured result; it must be tested against actual optical defect maps and metadata/calibration overhead.

## 10. Evidence status

This document and the baseline register are research specifications. They do not constitute optical-solver qualification, physical fabrication, measured retention, SOTA improvement or product approval. No laser, write head or magnetization actuator control is authorized by this programme.

## 11. Sources

- Microsoft Research Project Silica Team, “Laser writing in glass for dense, fast and efficient archival data storage,” *Nature* 650, 606–612 (18 February 2026): https://www.nature.com/articles/s41586-025-10042-w
- Microsoft Research paper landing page: https://www.microsoft.com/en-us/research/publication/laser-writing-in-glass-for-dense-fast-and-efficient-archival-data-storage/

- Lawson et al., “Property-driven analysis of glasses for data storage via femtosecond laser writing,” *Optica* 13(4), 698–706 (2 April 2026): https://www.microsoft.com/en-us/research/publication/property-driven-analysis-of-glasses-for-data-storage-via-femtosecond-laser-writing/
- SCHOTT BOROFLOAT 33 technical properties: https://www.schott.com/en-us/products/borofloat/technical-details
- Micron 6600 ION technical product specification (245.76 TB SKU sequential read/write): https://tw.micron.com/content/dam/micron/global/public/products/storage/ssds/data-center/6600/6600-ion-ssd-technical-product-specification.pdf
- Seagate Q1 FY2027 earnings call transcript (27 October 2026): https://s24.q4cdn.com/101481333/files/content_files/STX-US-CORRECTED-TRANSCRIPT-Seagate-Technology-Holdings-PlcSTXUS-Q1-2027-Earnings-Call-27-ET-PM-26-10-2026-2026-10-26-0.pdf
- Seagate Mozaic 4+ launch and production qualification (3 March 2026): https://investors.seagate.com/news/news-details/2026/Seagate-Delivers-Industrys-Highest-Capacity-Hard-Drives-with-Next-Generation-Mozaic-4/default.aspx
- Western Digital 40 TB UltraSMR shipping status (StorageReview, 6 August 2026): https://www.storagereview.com/news/wd-bets-8x-bandwidth-beats-more-terabytes-as-40tb-ultrasmr-starts-shipping
- LTO-10 technology and specifications: https://www.lto.org/lto-10/
- Micron 6600 ION product specifications: https://www.micron.com/products/storage/ssd/data-center-ssd/6600-ion
- Skyrmion racetrack noise model, *Physical Review B* 113, L180407 (18 May 2026): https://journals.aps.org/prb/abstract/10.1103/73z4-ymkp


## 8. Instrument-bound optical channel observations

The follow-on observation contract is tracked in [OPTICAL-CHANNEL-OBSERVATION-001 #7534](https://github.com/Luminous-Dynamics/symtropy/issues/7534), with its schema, synthetic fixture and protocol on the [optical-channel-observations branch](https://github.com/Luminous-Dynamics/symtropy/tree/research/optical-channel-observations-001/research/physical-storage).

The handoff preserves four evidence classes (synthetic fixture, published reproduction, physical measurement and model-derived) and binds every observation batch to a specimen/material, writer/process record, reader/calibration, coordinate frame and provenance digest. Raw detector observations, hard symbols and optional soft information remain distinct. Ground-truth labels are separately referenced and held-out splits are grouped by specimen/batch/reader to avoid neighbouring-voxel leakage.

The Draft 2020-12 schema includes conditional checks: synthetic-only batches cannot label their soft information calibrated; calibrated symbol vectors require a calibration reference; and a reader declared current/traceably calibrated must carry calibration identity, a record reference and uncertainty. A semantic validator is still required for unique observation IDs, alphabet membership, likelihood-vector cardinality and normalization, calibration compatibility, coordinate bounds and split non-overlap. JSON Schema does not prove instrument calibration or probability quality.

The example fixture is explicitly synthetic and makes no BOROFLOAT 33/K10 measurement claim. Symthaea's decoder and optimizer may consume these records only with their evidence class, provenance and partition identity preserved. No confidence score may silently become a physical error probability.
