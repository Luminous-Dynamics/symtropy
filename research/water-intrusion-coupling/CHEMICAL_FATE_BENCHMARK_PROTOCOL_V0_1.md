# Chemical-Fate Model Qualification — Comparator and T0 Protocol v0.1

**Programme:** WATER-INTRUSION-CHEMICAL-FATE-T0  
**Parent:** [#5192 model-coupling qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5192)  
**Snapshot:** 14 November 2026  
**Evidence ceiling:** synthetic analytical equations and model-scope contracts only

## 1. Purpose

Qualify the *chemical-fate interface*, not declare a preferred chemical solver or invent site kinetics. The stage receives a contaminant source term and hydraulic transport history from earlier qualified layers, then predicts parent compounds, transformation products, bulk/wall partitioning and reaction residuals within a declared water matrix.

The benchmark separates analytical equation verification, implementation regression, numerical convergence, calibration against a declared water matrix, blind physical/laboratory validation, and health/toxicological interpretation. A pass in one layer never implies a pass in the next.

## 2. Comparator landscape

The machine-readable scope/rights register is [chemical-fate-comparator-register-v0.1.json](chemical-fate-comparator-register-v0.1.json).

### 2.1 EPA EPANET-MSX 2.0 — strongest maintained open multi-species baseline

Pinned repository snapshot: [USEPA/EPANETMSX commit `85a303dca05859389eced63fabd5ea145c096333`](https://github.com/USEPA/EPANETMSX/commit/85a303dca05859389eced63fabd5ea145c096333). MIT licence is present in `license.txt`. Its CMake build pins EPANET 2.2 submodule `1e2fc3d909c0df654ead95e6e98c3307ce1375b1`.

EPANET-MSX defines dissolved/mobile **bulk** species and immobile **wall/surface** species, with kinetic ODE and equilibrium algebraic relationships (DAEs). Its transport algorithm is a Lagrangian segment solution with react, advect, node mixing, segment release and optional dispersion steps. Version 2.0 added optional dispersion, OpenMP for reaction/dispersion and improved mass-balance reporting.

The official manual makes several assumptions important to the benchmark:
- flows mix completely and instantaneously at junctions;
- storage nodes mix completely (alternative plug-flow tank schemes are available);
- wall species do not exist in storage tanks; tank reaction expressions cover bulk species only;
- bulk species use mass-per-volume units and wall species use mass-per-area units; modelers must supply unit conversions in reaction expressions.

These are explicit modelling assumptions, not universal physical truths. The manual also notes that reducing the segment limit can increase numerical diffusion, so resolution/segment-count convergence must be tested.

**Boundary:** EPANET-MSX uses the EPANET 2.2 hydraulic engine. It does not itself qualify high-frequency water-hammer pressure histories, pressure-driven intrusion hydraulics, bidirectional defect ingress, or source-term estimation. It is a chemical reaction/transport stage downstream of the pressure and ingress owners.

**Upstream verification maturity:** official workflow builds Windows, Ubuntu, Intel macOS and Apple Silicon, then runs a packaged example; a CMake `BUILD_TESTS` option is commented out. Do not describe this as a comprehensive independent physical chemistry test suite.

Sources: [EPA MSX 2.0 manual](https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P100XGNO.TXT), [conceptual framework](https://epanetmsx2manual.readthedocs.io/en/latest/2_framework.html), [repository](https://github.com/USEPA/EPANETMSX).

### 2.2 EPyT-C 1.0.0 — broad open research comparator

Pinned snapshot: [SWIL-IITK/EPyT-C commit `e86750c27b4da66c752c2916715707904040a7f9`](https://github.com/SWIL-IITK/EPyT-C/commit/e86750c27b4da66c752c2916715707904040a7f9). The source `setup.py` reports version 1.0.0; a MIT licence is present.

EPyT-C delegates hydraulic simulation to EPANET 2.2 via EPyT, then runs its own 1D multi-species reactive-transport implementation. Its README documents an explicit Method of Characteristics plus fourth-order Runge-Kutta method, time-varying/variable injection inputs, randomized source patterns, randomized rate-coefficient perturbations and configurable accuracy/runtime.

Four published modules are present: chlorine/THM formation; bacterial regrowth; arsenite oxidation and arsenate attachment/detachment; and PFOA formation. Its repository has five test scripts and a 3-OS × 3-Python-version CI matrix (Python 3.9, 3.10 and 3.11), with a sample run and a notebook run. The regression scripts use fixed Net3 values and module-level function calls. This is useful software regression evidence; it is not proof of physical transferability of reaction rates or field chemistry.

**Boundary:** EPyT-C remains a Python research comparator and does not replace the Rust production runtime. Its underlying EPANET 2.2 hydraulic state must not be confused with a qualified transient pressure history or reverse-flow ingress model. Pin the EPyT/EPANET dependency environment and network revision for every reproduction.

Sources: [repository](https://github.com/SWIL-IITK/EPyT-C), [README](https://github.com/SWIL-IITK/EPyT-C/blob/e86750c27b4da66c752c2916715707904040a7f9/README.md), [test workflow](https://github.com/SWIL-IITK/EPyT-C/blob/e86750c27b4da66c752c2916715707904040a7f9/.github/workflows/run_tests.yml).

### 2.3 EPANET-PMX — paper-only pressure-driven multispecies comparator

Seyoum and Tanyimboh (2017), *Integration of Hydraulic and Water Quality Modelling in Distribution Networks: EPANET-PMX*, Water Resources Management 31, 4485–4503, DOI [10.1007/s11269-017-1760-0](https://doi.org/10.1007/s11269-017-1760-0).

The paper proposes a pressure-driven multi-species extension combining pressure-dependent hydraulic demand and multi-species quality modelling, with steady-state and dynamic simulations and multiple reactions. No maintained public source/build/test artefact was established in this snapshot; therefore it is a literature comparator, not an executable benchmark entry. Its "dynamic" water-quality simulation must not be presumed to mean high-frequency water hammer or cavitation/column separation.

## 3. T0 known-answer corpus

The executable synthetic set is [chemical-fate-t0-cases-v0.1.json](chemical-fate-t0-cases-v0.1.json), compared with the separate [oracle](chemical-fate-t0-oracle-v0.1.json). It covers first-order decay; sequential A→B→C transformation with unequal and equal rate-constant limits; Langmuir bulk/wall equilibrium; a conserved-moiety ledger distinct from parent-compound mass; explicit stoichiometric/molecular-weight conversion; missing residual streams; missing water matrix/temperature/pH; missing rate coefficients; unsupported wall species in storage tanks; pressure-driven transient input incorrectly supplied by EPANET-MSX-only hydraulics; required dispersion with dispersion disabled; and invalid negative concentration/rate values.

For the first-order case, parent loss is reported as an equivalent concentration, **not** as a measured transformation-product concentration. Sequential A/B/C concentrations must declare a common `conserved_moiety_equivalent` basis.

**Mass-balance rule:** compound concentrations in mg/L must not be summed across species with different molecular weights and declared conserved mass by assumption. Convert each compound using molecular weights and stoichiometric coefficients, or track a declared conserved element/moiety on a consistent equivalent basis. Parent loss alone is not evidence of a measured product. Missing products, side streams or wall fractions remain unresolved rather than being assigned zero.

The equations are deliberately small and analytically checkable. A successful arithmetic vector is not a chemical model qualification.

## 4. Water-matrix and reaction-parameter contract

Do not transfer kinetic constants across water sources without recording source and sample matrix identity; temperature, pH, alkalinity, ionic strength and relevant organic-matter measures; disinfectant species and residual; parent species and transformation products; pipe material, age, surface condition and surface-area-to-volume ratio; bulk/wall partition and mass-transfer regime; residence time, shear, velocity, stagnation and hydraulic mixing; and method, detection/quantification limit, laboratory scope, calibration and uncertainty.

Unknown inputs produce an indeterminate parameter state. No generic literature coefficient, zero reaction, or "inert" assumption is substituted silently.

## 5. Benchmark and SOTA goals

**Numerical verification**
- exact first-order and sequential-reaction solutions across short/long time horizons;
- stiff and near-equal rate-constant limiting cases;
- dimensional analysis across concentration, rate, time and surface-loading units;
- ODE/DAE solver tolerance and time-step convergence;
- mass balance of the conserved moiety across bulk, wall and transformation products, with residual uncertainty preserved.

**Physical validation**
- blinded parent and transformation-product concentration time series under multiple temperatures, pH and water matrices;
- bulk and wall samples where the model claims sorption/biofilm or pipe-material exchange;
- independent disinfectant residual, total organic carbon and relevant precursor/product measurements;
- different pipe materials and ages held out from calibration;
- repeated measurements and uncertainty-aware recovery.

**Metrics**
- parent/product concentration MAE/RMSE and normalized error by species;
- peak and arrival-time error for concentration breakthrough;
- integrated conserved-moiety recovery and unexplained residual;
- uncertainty-interval coverage and width;
- identifiability and sensitivity of kinetic/sorption parameters;
- numerical convergence, solver failures and mass-balance residual;
- runtime and memory, reported only alongside equal physical scope and error.

**Research targets (not achieved):**
- at least 90% coverage of nominal 90% predictive intervals on blind held-out physical matrices, with widths reported;
- no false-safe clearance from missing species or missing analytical coverage;
- conserved-moiety recovery within a pre-registered tolerance derived from combined measurement uncertainty;
- no transferable reaction coefficient accepted without a water-matrix and pipe-material qualification.

## 6. Nonclaims

No T0 result identifies real contaminants, predicts a consumer dose, establishes toxicity/pathogen risk, proves a treatment barrier, establishes potability, certifies SANS 241, or authorizes water release. Laboratory results, competent water-safety professionals and statutory authorities retain their respective roles.