# Water Junction Mixing Benchmark Matrix v0.1

**Public steady-junction dataset intake:** [#5308](https://github.com/Luminous-Dynamics/symtropy/issues/5308)  
**Transient operator:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274), [draft #5288](https://github.com/Luminous-Dynamics/symtropy/pull/5288)  
**Engine interoperability:** [#5290](https://github.com/Luminous-Dynamics/symtropy/issues/5290), [draft #5293](https://github.com/Luminous-Dynamics/symtropy/pull/5293)  
**Positive model class:** [#5294](https://github.com/Luminous-Dynamics/symtropy/issues/5294), [draft #5297](https://github.com/Luminous-Dynamics/symtropy/pull/5297)  
**Constrained identification:** [#5298](https://github.com/Luminous-Dynamics/symtropy/issues/5298), [draft #5306](https://github.com/Luminous-Dynamics/symtropy/pull/5306)  
**Evidence class:** benchmark protocol and public-source inventory; no external benchmark data have yet been retrieved or scored

## 1. Paradigm: three benchmark claims, three separate evidence tracks

A model should never be allowed to transfer success from one evidence regime into another.

| Track | Question | Valid evidence | What it cannot prove |
|---|---|---|---|
| S — steady junction | Can the model predict outlet-resolved concentration split across geometry and operating state? | Measured steady experiments and experimentally validated CFD datasets | Transient residence-time memory, sensor lag, impulse response |
| T — transient operator | Does the local junction have identifiable causal memory beyond the steady transfer matrix? | Time-resolved tracer pulse/step experiments with calibrated sensors and complete mass boundaries | Network-scale solver compatibility by itself |
| N — network integration | Does the qualified local operator compose correctly with network hydraulics and pipe-parcel transport? | Exact-engine differential tests and physical network validation | Physical local mixing accuracy without Track S/T |

Gallo et al. (2025) gives us an open steady-junction benchmark. It is not a transient RTD or tracer impulse-response dataset. The T1 non-potable campaign remains necessary for Track T.

## 2. Public benchmark anchor: Gallo et al. 2025

Citation: Gallo, P. et al. (2025), *Accurate and efficient junction mixing prediction in water distribution networks via surrogate modelling*, Journal of Water Process Engineering 77, article 108505, DOI https://doi.org/10.1016/j.jwpe.2025.108505.

Dataset: https://doi.org/10.5281/zenodo.15753653  
Ghent University catalogue: https://biblio.ugent.be/publication/01K4PZV12VJ3FGPK4KBT7N9K84

The catalogue describes a publicly available CSV dataset from a high-resolution screening over operating conditions for three common junction geometries using an experimentally validated CFD model. The data were used to train an ANN surrogate that was coupled to EPANET. The catalogue states CC BY-SA 4.0. The article abstract reports median CFD validation error of 1.15% and network-level differences up to 80% when imperfect mixing is considered. These are the authors' reported figures, not independently reproduced Symtropy results.

**Retrieval status:** the DOI, catalogue record, CSV format, three-geometry scope and licence have been independently verified. The actual Zenodo binary/file list has not been retrieved in this authoring environment; therefore no CSV schema, row count, units, file digest, or numerical benchmark score is claimed. Dataset ingestion must start with exact record/file retrieval and schema inspection.

## 3. Comparator inventory

### 3.1 Complete-mixing baseline

Use EPANET 2.2/OWA-EPANET as the established complete instantaneous junction-mixing baseline. Preserve the existing Lagrangian pipe-parcel transport, source application, flow reversal and tank model.

### 3.2 Experimental and empirical incomplete-mixing models

- Yousefian & Duchesne (2024/2025), *Improving incomplete mixing modeling for junctions of water distribution networks*, DOI https://doi.org/10.2166/hydro.2024.041. Published work covers six cross-junction configurations including unequal pipe sizes and larger 150 mm pipes. Its supported configurations and input domain must be respected; do not silently generalize to tees or other topologies.
- AZRED / AZRED variants and other published cross-junction closures should be compared only where geometry, flow ratio, pipe size and output definition match the benchmark.
- The 2023 EPANET INP polynomial-equation workaround uses bypass pipes and setpoint boosters to place outlet-specific concentrations into a standard EPANET input file; the paper reports relative concentration differences up to 14% in its tested cases. This is an important practical industry comparator, but its altered network representation and booster semantics must be accounted for.

### 3.3 CFD + ANN surrogate

- Gallo et al. (2025): public three-geometry CSV dataset and ANN surrogate coupled to EPANET. Reproduce the published ANN only if code/weights, preprocessing, exact splits and output definitions can be obtained. Otherwise reproduce the baseline from the public CSV with documented architecture and avoid calling it the original Gallo ANN.
- Wéber et al. (online 19 September 2026), *Modeling Incomplete Solute Mixing at Pipe Junctions in Water Distribution Networks: Parametric CFD Simulations and a Predictive Regression Approach*, DOI https://doi.org/10.1061/JHEND8.HYENG-14715. Its CFD/regression work focuses on cross-junction pipe connection angles and flow distributions. The publisher says supporting data/models are available on reasonable request. Do not claim direct parity until the source data or a faithfully reproduced implementation is available.

### 3.4 Symtropy candidate models

1. Complete-mixing baseline (nested zero-delay limit).
2. Empirical steady outlet transfer matrix P.
3. Published empirical closure where its domain matches.
4. Positive, passive, constrained reduced-order steady operator.
5. Dynamic causal kernel / positive state-space model, but only on Track T after transient identifiability is demonstrated.

## 4. Dataset intake protocol

The source dataset must be treated as immutable third-party evidence.

1. Retrieve the exact Zenodo record/version and every source file.
2. Record file names, sizes, source timestamps, DOI, record version, CC BY-SA 4.0 terms and SHA-256 digests.
3. Store source file references and hashes; do not rewrite or normalize raw files.
4. Inspect real headers, units, categories, geometry labels, row keys, input/output definitions, precision and missing-value encodings before implementing the parser.
5. Retain every source row; rejected rows remain in a rejection ledger with machine-readable reasons.
6. Verify whether each row is a CFD operating point, an experimental point, a train/test split, or a derived response.
7. Determine if the CFD outputs conserve the scalar mass on the declared control volume; only test passivity where all necessary boundary terms are represented.
8. Create a schema version and deterministic source-to-canonical transformation receipt with input/output digests and row counts.
9. Preserve the CC BY-SA attribution and share-alike terms in any derived dataset or redistributed adapter output.

No assumed column names or unit conversions are allowed before step 4.

## 5. Leakage-resistant split design

Random row splitting is invalid for a parameter sweep if adjacent rows share geometry, boundary state or a CFD run.

Freeze splits before fitting:
- whole geometry holdout;
- whole flow-ratio/momentum-ratio region holdout;
- interpolation holdout within a geometry but outside the fit grid cells;
- extrapolation holdout outside the covered operating envelope;
- optional whole-source/facility holdout if independent physical datasets become available.

Report each metric separately. Never average a high-error extrapolation stratum into a good interpolation score without showing both.

## 6. Metrics and research targets

Report per geometry and per operating regime:
- outlet-resolved concentration MAE, RMSE, bias and maximum absolute error;
- error by outlet, not only a pooled average;
- high-quantile error and uncertainty interval coverage/width where uncertainty is available;
- conservation/passivity tests only where the dataset supplies a complete mass boundary;
- interpolation versus extrapolation error;
- domain coverage, abstention/refusal rate and unsupported-case reasons;
- inference runtime, memory, model complexity and calibration/compute cost.

Research targets (not achieved):
- beat the published ANN/empirical baseline on held-out geometries or operating regions while preserving positivity/passivity constraints;
- zero accepted negative concentrations for passive nonnegative inputs;
- zero passive-gain violations where the complete outlet boundary is known;
- 90% predictive interval coverage on genuinely blind, independent physical Track S/T data, with interval width published;
- lowest model complexity that meets the accuracy/coverage requirement;
- abstain outside the validated domain instead of silently extrapolating.

The Gallo paper's reported CFD median validation error (1.15%) is contextual information, not a target applied to a different data split or a result already obtained by this project.

## 7. Transient evidence remains distinct

The Gallo dataset must not be used to claim:
- a physical local residence-time distribution;
- a causal impulse response;
- time-dependent storage or memory;
- sensor response or sample-line delay calibration;
- full tracer tail recovery;
- time-domain mass closure.

Track T still requires the controlled non-potable T1 campaign. The input excitation, sensor response, clock alignment, every outlet and tail mass must be measured independently.

## 8. Network integration remains distinct

For Track N:
- operator-disabled parity against a pinned engine;
- exact zero-delay complete-mix parity;
- outlet-specific segment creation with no change to hydraulics/parcel ordering;
- source/demand/leakage/tank/reaction accounting;
- flow reversal and hydraulic-event handling;
- network-level volume and tracer mass reconciliation;
- per-node operator eligibility, domain and refusal receipt.

A steady benchmark win does not authorize the EPANET hook or make a drinking-water model safe.

## 9. Evidence ceiling

This file defines a benchmark protocol and verified public source inventory. The Gallo CSV has not yet been ingested or independently evaluated. The 2025 ANN and 2026 regression baselines are not reproduced here. No physical transient operator, sensor, contaminant detector, potable system, toxicology claim, SANS 241 compliance or water-release authority is established.
