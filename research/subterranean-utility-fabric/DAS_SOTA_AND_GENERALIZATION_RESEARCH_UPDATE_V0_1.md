# DAS State-of-the-Art and Generalization Research Update v0.1

**Snapshot:** 2026-11-13  
**Owner:** Symtropy #4972 / active calibration #5002  
**Status:** literature triage; metrics are paper-reported and not directly comparable across tasks

## 1. Why this update matters

The DAS field is advancing along several different axes at once:
- event detection/classification;
- vehicle trajectory reconstruction from distributed signals;
- buried fibre route geolocalization;
- cross-domain/few-shot recognition;
- large-scale data handling and storage.

These are not one leaderboard. A result in traffic trajectory F1 does not establish buried-fibre geolocation accuracy; a laboratory classifier accuracy does not establish a field fault detector; a 190 km product range does not establish per-event detection probability or false-alarm burden.

The programme should compare methods within matched tasks and then evaluate the full evidence-to-decision workflow.

## 2. New literature baselines identified

### 2.1 Semantic-perception and topology-based vehicle trajectory reconstruction (2026)

Ma et al., *A semantic perception-topological reasoning framework for robust vehicle trajectory reconstruction in distributed acoustic sensing*, Expert Systems with Applications, 1 July 2026, DOI 10.1016/j.eswa.2026.131886.

The publisher's article highlights report:
- semantic perception and topology-aware reasoning for vehicle trajectory reconstruction;
- lightweight CNNs with physics-guided graph optimization;
- 89.13% F1 under low-SNR DAS environments;
- handling of nonlinear trajectories and intersection ambiguities.

**Correct use in SUF:** add as a trajectory-reconstruction baseline for the traffic/vehicle observation stage in active calibration. It is not a buried fibre route-geolocation score and must not be placed on the same leaderboard as Cohen et al.'s route reconstruction metric.

Required diligence: obtain full method, dataset/population, exact F1 definition, route split, class distribution, low-SNR definition, topology inputs and code/data terms. Reproduce the published task before adopting its reported result as an attainable baseline.

### 2.2 Cross-domain few-shot DAS recognition (2026)

*Cross-Domain Few-Shot Recognition in Distributed Optical Fiber Acoustic Sensing based on Contrastive Learning with Generative Data Augmentation*, IEEE Internet of Things Journal early access, 3 September 2026, DOI 10.1109/JIOT.2026.3730549.

The publisher abstract reports:
- contrastive self-supervised representation learning and generative augmentation;
- few-shot adaptation to DAS target domains;
- 98.56% and 98.44% accuracy on laboratory and field datasets under the reported full-unlabelled-data / 50-labelled-samples-per-class setting;
- over 90% accuracy when using 10% unlabelled data and 10 labelled samples per class;
- open-set recognition results of 93.79% and 91.17% overall accuracy in its reported conditions.

**Correct use in SUF:** an important candidate baseline for cross-domain event recognition and low-label adaptation. The claims require exact task, split, target-site leakage and open-set definitions to be checked before replication. These numbers are not directly comparable with the five-class Mendeley power-cable intrusion dataset or the DAS-10 loudspeaker-replay dataset.

Required diligence:
1. Extract train/validation/test grouping and source/target-domain construction.
2. Confirm whether target-site labels or recordings enter representation learning.
3. Freeze open-set classes and measure unknown rejection separately from known-class accuracy.
4. Report calibration, false alarms per fibre-km-day and site-held-out transfer, not accuracy alone.
5. Reproduce both its full-data and low-label settings before claiming improvement.

### 2.3 DAS big-data architecture review (2026)

Rafi et al., *Big data architectures for distributed acoustic sensing: a review of storage, real-time processing, and scalable analytics*, Results in Optics 24 (March 2026), DOI 10.1016/j.rio.2026.101028.

The review treats acquisition, storage, real-time processing, compression, distributed analytics and hardware acceleration as one end-to-end system. It identifies unresolved issues including uncertainty in lossy compression, model drift under concept shift and the need for standardized operational benchmarks.

**Correct use in SUF:** data lifecycle and system economics are first-class benchmark axes. The platform must demonstrate that compression, windowing, edge inference and event-triggered retention do not erase rare-event information or make claims irreproducible.

### 2.4 Subsurface DAS method and benchmark review (2025)

*A Systematic and Comprehensive Review on the Application of the Distributed Acoustic Sensing Data in Subsurface Application: Methodologies, Deep Learning Approaches, and Future Prospects*, IEEE Access 13 (31 October 2025), DOI 10.1109/ACCESS.2025.3627765.

The publisher page reports an initial pool of 532 publications and 149 selected preprocessing, event-detection and classification experiments. It notes deficiencies in annotation methods, benchmarking criteria and incorporation of domain expertise.

**Correct use in SUF:** research differentiation should focus on repeatable, field-relevant benchmark design, explicit domain expertise, held-out infrastructure profiles and complete operational denominators—not simply a different neural architecture.

## 3. Revised end-to-end task decomposition

A useful DAS/traffic observability system contains at least these separate tasks:

1. Sensor/interrogator data-quality assessment.
2. Timebase synchronization and trajectory/event association.
3. Traffic trajectory reconstruction from DAS.
4. Event screening and event-class hypothesis.
5. Fibre-relative chainage estimation.
6. Fibre-to-route calibration.
7. Geographic registration and uncertainty.
8. Owner asset attribution.
9. Causal engineering diagnosis.
10. Investigation/maintenance decision.

Each task needs its own reference truth, population, metric and abstention condition. A downstream claim cannot be more certain than an unsupported upstream transformation.

## 4. New paradigm: topology-constrained, multi-hypothesis calibration

The candidate research direction is to compose:
- Ma et al.'s semantic/topological vehicle trajectory reasoning;
- Cohen et al.'s traffic-induced DAS strain-map route geolocalization;
- independently referenced survey/tap endpoints;
- Sol-Atlas route topology and revision history;
- Symthaea's active measurement selection.

The system should solve for a graph of plausible fibre paths, not just one unconstrained curve. It should preserve:
- branch and endpoint identity;
- route crossings and self-near segments;
- alternative road/fibre alignments;
- calibration ambiguity and unresolved gauge freedoms;
- local confidence/uncertainty by segment.

This is a research hypothesis, not an implemented estimator. Topology should constrain solutions only when its source identity and correctness are independently supported; a wrong legacy graph must not force the estimate onto the wrong route.

## 5. Data lifecycle and compression benchmark

Build a source-preserving, multi-resolution storage policy:

- **Tier 0 — source capture:** preserve raw/native files and exact interrogator configuration where rights and storage permit.
- **Tier 1 — lossless archive:** immutable, digest-bound raw chunks for selected qualified windows and calibration events.
- **Tier 2 — normalized representation:** reversible/declared transformations, unit/timebase metadata and exact source-parent references.
- **Tier 3 — derived features:** event candidates, spectrograms, strain maps, embeddings and quality indicators with method/configuration revisions.
- **Tier 4 — decision records:** claim level, uncertainty, coverage, action/value calculation and independent adjudication.

Any lossy compression or downsampling is a new derived artifact with a declared distortion/error budget and separate performance qualification. Never overwrite original bytes with compressed features.

Compare:
- lossless compression;
- declared lossy compression at multiple rates;
- feature-only retention;
- event-triggered high-rate capture with pre/post buffers;
- continuous lower-rate screening plus high-rate burst capture;
- edge screening with cloud/human review.

Metrics:
- bytes per fibre-km-day and cost per retained km-year;
- read/write throughput and p50/p95 query latency;
- event POD and critical miss rate before/after compression;
- localization error and uncertainty coverage before/after compression;
- rare-event retention and timebase fidelity;
- CPU/GPU, memory and energy per km-day;
- recovery/replay success using immutable raw artifacts.

The goal is not maximum compression. It is minimum total data lifecycle cost subject to pre-registered detection, localization, uncertainty and replay constraints.

## 6. Updated SOTA comparison protocol

For every paper/vendor:
- record the exact task and claim;
- distinguish simulated, laboratory, field and operational populations;
- record fibre type, coupling, soil/conduit, route geometry and interrogator;
- freeze label and truth-generation process;
- recover exact split strategy and acquisition grouping;
- extract metric definitions and denominators;
- record access and reuse rights;
- replicate before attempting to beat the score.

For Luminous:
- report site/corridor-held-out evaluation;
- include open-set and unknown-event cases;
- report abstention/risk-coverage and calibration;
- report false alarms per observable fibre-km-day;
- report event chainage and geographic location separately;
- report blind/degraded length and time;
- report human and physical dispatch burden;
- report total data/compute/storage and lifecycle cost.

## 7. Proposed research targets (not achieved results)

- Reproduce relevant literature baselines before tuning candidate methods.
- At matched false-dispatch cost and critical-miss rate, improve held-out event detection or reduce human/robot dispatches.
- For trajectory reconstruction, match or exceed the 2026 topology-aware 89.13% F1 only on the same task, population and split; additionally report route consistency and uncertainty.
- For cross-domain recognition, reproduce the 2026 few-shot method's exact splits and open-set metrics before comparing; do not adopt its accuracy numbers as universal DAS performance.
- Preserve 95% location-interval coverage on held-out in-domain routes while minimizing interval width.
- Reduce retained bytes and lifecycle cost without statistically degrading rare-event detection, chainage/geographic localization or replay.
- Demonstrate transfer to a new fibre/coupling profile without hidden target-site training.

## 8. References

- Ma et al. (2026), trajectory reconstruction, DOI: https://doi.org/10.1016/j.eswa.2026.131886
- Cross-domain few-shot DAS recognition (2026), DOI: https://doi.org/10.1109/JIOT.2026.3730549
- Rafi et al. (2026), DAS data architecture review, DOI: https://doi.org/10.1016/j.rio.2026.101028
- IEEE Access subsurface DAS review (2025), DOI: https://doi.org/10.1109/ACCESS.2025.3627765
- Cohen et al. (2026), buried fibre geolocalization, DOI: https://doi.org/10.1109/TGRS.2026.3729504
- Kamaruzaman et al. (2026), SCADA communications fibre fault screening, DOI: https://doi.org/10.1016/j.rio.2026.101093

## 9. Claim ceiling

This is literature triage and a proposed benchmark update. The cited metrics are publisher-reported and task-specific. No literature result has been reproduced by Luminous, and no new field, commercial, cost or SOTA superiority claim is made.
