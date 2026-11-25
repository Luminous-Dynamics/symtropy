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
| Glass phase voxels (borosilicate) | Published research regime | 0.678 Gbit/mm³; 258 layers; 2.02 TB per platter; 18.4 Mbit/s; 8.85 nJ/bit | Lower-cost glass and simpler read/write path in the paper; accelerated ageing suggests >10,000-year voxel lifetime, which is an extrapolation, not a direct 10,000-year observation. |
| Seagate Mozaic 4+ HAMR | Production ramp with hyperscale customers | Up to 44 TB per HDD; continued ramp described in the 27 October 2026 Q1 FY2027 earnings call | Industrial nearline capacity and mature deployment. Magnetic recording is an operationally proven storage tier; this is not a lab-only candidate. |
| WD 40 TB UltraSMR ePMR | Shipping/ramping, reported August 2026 | Up to 40 TB per HDD | Industrial nearline capacity. UltraSMR trades some rewrite flexibility for density; workload compatibility matters. WD's first-generation 44 TB HAMR was still forecast for H1 2027 in the cited August report. |
| LTO-10 tape | Current format/product family | 40 TB native per cartridge; up to 400 MB/s native | Removable, offline, high-capacity archive with WORM and hardware encryption options. Cartridge capacity is not comparable to a glass volume density figure. |
| Micron 6600 ION QLC SSD | Commercial product family | 245.76 TB usable / 256 TB raw in E3.L | Very high-capacity flash and low-latency access; a different cost, endurance, power and access tier from tape or glass archive. |

### Important arithmetic sanity check

The Silica paper reports 25.6 Mbit/s per beam. LTO-10's 400 MB/s native headline corresponds to 3.2 Gbit/s. The latter is approximately 125 times the single-beam bit rate. This is a **channel-rate comparison only**, not a whole-system or same-workload verdict: multibeam optical parallelism, drive interfaces, read rate, seek/access semantics and energy boundaries must be measured separately.

Likewise, glass volumetric density, HDD areal density, tape cartridge capacity and SSD package capacity must remain separate dimensions. No direct ranking may be made by comparing those raw numbers.

## 3. Paradigm shifts to test

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
3. Keep at least 90% of gross physical payload after ECC, metadata, calibration and spare-region overhead in the first proposed layout; publish the exact overhead.
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

## 9. Evidence status

This document and the baseline register are research specifications. They do not constitute optical-solver qualification, physical fabrication, measured retention, SOTA improvement or product approval. No laser, write head or magnetization actuator control is authorized by this programme.

## 10. Sources

- Microsoft Research Project Silica Team, “Laser writing in glass for dense, fast and efficient archival data storage,” *Nature* 650, 606–612 (18 February 2026): https://www.nature.com/articles/s41586-025-10042-w
- Microsoft Research paper landing page: https://www.microsoft.com/en-us/research/publication/laser-writing-in-glass-for-dense-fast-and-efficient-archival-data-storage/
- Seagate Q1 FY2027 earnings call transcript (27 October 2026): https://s24.q4cdn.com/101481333/files/content_files/STX-US-CORRECTED-TRANSCRIPT-Seagate-Technology-Holdings-PlcSTXUS-Q1-2027-Earnings-Call-27-ET-PM-26-10-2026-2026-10-26-0.pdf
- Seagate Mozaic 4+ launch and production qualification (3 March 2026): https://investors.seagate.com/news/news-details/2026/Seagate-Delivers-Industrys-Highest-Capacity-Hard-Drives-with-Next-Generation-Mozaic-4/default.aspx
- Western Digital 40 TB UltraSMR shipping status (StorageReview, 6 August 2026): https://www.storagereview.com/news/wd-bets-8x-bandwidth-beats-more-terabytes-as-40tb-ultrasmr-starts-shipping
- LTO-10 technology and specifications: https://www.lto.org/lto-10/
- Micron 6600 ION product specifications: https://www.micron.com/products/storage/ssd/data-center-ssd/6600-ion
- Skyrmion racetrack noise model, *Physical Review B* 113, L180407 (18 May 2026): https://journals.aps.org/prb/abstract/10.1103/73z4-ymkp
