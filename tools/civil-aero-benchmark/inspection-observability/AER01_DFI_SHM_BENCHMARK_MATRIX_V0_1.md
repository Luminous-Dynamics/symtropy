# AER-01 design-for-inspection and sensing benchmark matrix v0.1

**Parent work:** #4165 / PR #4167  
**Related engineering owners:** #3921, #4056, #4093, #3495, #2890, #3237.

## 1. State of the art: compare like with like

| Reference / method | What it contributes | Correct benchmark role | What it does not establish |
|---|---|---|---|
| NASA Design for Inspection / Inspectability Metric (NTRS 20240003953, 20240007593) | Simulation-based, automation-friendly inspectability assessment; defect findability and sensitivity to variability; inspectability as a multiobjective design variable alongside mass/manufacturability | Reference DFI methodology and interface. Compare structural design candidates and the efficiency/coverage of their inspection test plans | The public abstract/presentation record does not provide an openly specified universal numeric score or a general-purpose executable implementation. Do not claim reproduction from a similar field name |
| SAE ARP6461A | Fixed-wing SHM implementation guidance: measurements, accuracy, reliability, maintenance/airworthiness-document integration | Process and requirements crosswalk for a fixed-wing SHM concept | Not a numerical DFI benchmark, not a regulatory approval, and not permission to replace required NDE. Authority agreement is required when SHM is proposed to fulfil inspection requirements |
| ASTM E2862-23 | Statistical analysis of hit/miss POD data for a specified examination parameter set; POD curve formulation, validation and interpretation | Statistical baseline for eligible physical hit/miss NDE studies | Not a design inspectability metric, not applicable to all POD data forms, and not evidence obtainable from synthetic cases |
| MIL-HDBK-1823A | NDE system reliability assessment guidance and detailed POD-study framework | Guidance comparator for study design and method/equipment/operator reliability | The active DLA record explicitly says it is guidance only and cannot itself be cited as a requirement |
| 2026 sensing digital twin (Di Fiore et al.) | Separates structural damage and sensing faults; estimates sensing reliability; includes sensor bias, drift, gain change, saturation, dropout and stuck-signal mechanisms | Research baseline for sensing-layer health before downstream SHM | Demonstration is numerical, on a small virtual composite panel. Local stiffness weakening is a simplified damage representation with acknowledged fidelity limitations |
| Conventional scheduled NDE | Qualified method/procedure and trained inspectors applied to a known material/defect family | Baseline physical inspection workflow | Must not be treated as a universal performance result across materials, operators, access conditions or defect morphologies |
| Naive SHM | Structural inference from measured response assuming sensor channels are healthy | Deliberate negative baseline to expose sensor-fault confounding | Not a trustworthy baseline for maintenance or safety decisions once sensor faults/ODD changes are present |

Primary records:
- NASA DFI: https://ntrs.nasa.gov/citations/20240003953
- NASA Inspectability Metric: https://ntrs.nasa.gov/citations/20240007593
- SAE ARP6461A: https://saemobilus.sae.org/standards/arp6461a-guidelines-implementation-structural-health-monitoring-fixed-wing-aircraft
- ASTM E2862-23: https://store.astm.org/e2862-23.html
- DLA MIL-HDBK-1823: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=206122
- 2026 sensing digital twin: https://link.springer.com/article/10.1007/s00158-026-04324-8
- SACAA Airworthiness guidance: https://www.caa.co.za/industry-information/airworthiness/

## 2. Paradigm shifts

### 2.1 Design for inspection from the first structural topology decision

The structure must be optimized not just for load carrying and manufacturability but also for whether the relevant defects can be found and assessed using a feasible procedure. Compare spar/rib/skin/fastener/bond architectures, probe paths, access, surface preparation and disassembly. A design that is lighter but cannot be reliably inspected in a critical region is not automatically superior.

### 2.2 Separate damage observability from sensor observability

A structural state may be observable with healthy sensors, while the deployed sensing system may itself be unhealthy. Explicitly model:
- physical structure state;
- sensor state;
- environment and operating condition;
- measurement and calibration chain;
- inference state and applicability.

A sensing twin is an upstream data-quality assessor, not an airworthiness oracle. Correct a sensor only when the correction has a validated physical basis and preserve the uncorrected data, correction, residual and reliability state. A corrected signal must not erase a plausible damage indication.

### 2.3 Replace a scalar "inspectability score" with a claim-specific vector

Keep critical-region coverage, defect-family applicability, POD/confidence, false calls, method variability, access cost, sensor reliability and evidence maturity separate. If a scalar is useful for a particular optimization, publish its weights, constraints, Pareto alternatives and sensitivity. Hard critical-region failures must not be hidden by a good average.

### 2.4 Treat inspection as an experiment with value of information

Inspection frequency and method selection should depend on how the result could change a maintenance/architecture decision, defect criticality, POD support, inspection burden and the cost of false calls/misses. Compose with #2890 rather than creating a separate generic VOI engine.

## 3. Matched benchmark design

For every candidate architecture, freeze:
- exact geometry / structural model / material and manufacturing process;
- region and critical-detail list;
- defect family, morphology, orientation, size/depth/location range;
- NDE method/procedure revision, equipment/probe, calibration and operator population;
- environment, access, surface, coupling and inspection conditions;
- ground-truth method, destructive confirmation where appropriate, and uncertainty;
- sample allocation, test/train/held-out split, analysis method and cost boundary.

Candidate methods must receive matched test specimens or statistically defensible matched sets. Do not tune a detection threshold on held-out flaws or use the same specimen repeatedly as independent evidence without accounting for dependence.

### Physical NDE metrics

Report per defect/material/method/condition:
- POD curve and confidence intervals using the appropriate current analysis method;
- a90/95 only where the chosen method and study design support that quantity;
- probability of false call / false alarm separately;
- flaw localization error and sizing error, when those are part of the declared decision;
- invalid / uninspectable / unresolved fraction;
- inter-inspector and equipment variability;
- specimen, flaw, operator and procedure applicability.

For ASTM E2862-23, use the hit/miss POD method only when its scope and data form fit. Do not collapse false calls into hits/misses. Do not claim POD from simulation, synthetic labels, or classifier accuracy.

### DFI / design metrics

Report:
- percentage of required critical regions physically accessible;
- scan path / surface coverage and blind or occluded regions;
- feasible probe orientation, stand-off, coupling and required surface prep;
- number and severity of required disassembly steps;
- time to access, inspect, restore and independently verify;
- sensitivity of detection opportunities to geometry, defect placement, operator/equipment variation and environmental variation;
- incremental mass, drag, joint complexity, manufacturing burden, and repair burden created by access features.

### SHM sensing-layer metrics

Use separate held-out sets for:
1. structural damage / healthy sensors;
2. healthy structure / sensor fault;
3. structural damage + sensor fault;
4. environmental/operational change;
5. unknown and out-of-domain state.

Compare:
- naive SHM with healthy-sensor assumption;
- analytical redundancy / observer residual baseline;
- a sensing digital twin that evaluates sensor health;
- the same models with sensor faults not seen during training;
- common-mode and correlated channel failures.

Report confusion matrix by physical cause, false-clear rate, false-damage rate, fault isolation coverage, abstention, calibration of probabilistic reliability when applicable, reconstruction error, and preservation of damage-sensitive signal components. A low reconstruction RMSE alone is not sufficient: a method that smooths away a damage signal is a failure.

### Total lifecycle cost

Feed measured task times to #4056 rather than duplicating its ledger. Include:
- inspection setup and execution;
- access/disassembly and restoration;
- tool/equipment calibration and specialist availability;
- false calls, repeat inspection and unnecessary repair;
- missed defects and risk/maintenance consequences as separately assessed items;
- sensor mass, wiring, power, calibration, replacement and data processing;
- scheduled downtime and dispatch impact.

## 4. Statistical and generalization requirements

- Hold out entire specimens and flaw placements, not just randomly selected signal samples from the same specimen.
- Hold out at least one defect size band, one location/orientation region and a material/layup or geometry family before making transfer claims.
- Keep operator and equipment batches identifiable; quantify between-operator and between-equipment variation.
- Use paired comparisons where feasible; report confidence intervals/effect sizes, not just means.
- Preserve failed, invalid and unresolved inspection attempts in the all-attempt ledger.
- Distinguish simulation, coupon, component, subcomponent and aircraft evidence.
- Treat class imbalance explicitly; report per-family false negatives and misses, not aggregate accuracy alone.
- If probability estimates are emitted, show calibration on untouched grouped holdout data and do not transfer calibration to a different material/procedure/geometry without evidence.

## 5. Proposed gates

These are engineering research goals, not regulatory thresholds.

### Synthetic contract gate
- 100% rejection of stale geometry/frame, wrong configuration, expired calibration and invalid procedure identity.
- Zero false completion when any required critical region is unobserved.
- Zero promotion of synthetic cases to physical POD or inspection acceptance.
- Zero sensor-correction-based clearance while a structural damage hypothesis remains.
- 100% abstention for unknown sensing state or unqualified method/defect/material combinations.

### Physical qualification gate
Thresholds must be set from defect criticality, material, method, the actual procedure, the sample/operator population, measurement uncertainty and the agreed approval programme. There is no universal DFI or SHM "accuracy target" valid across all materials and aircraft regions. Where a90/95 is required, study design and acceptance must be controlled by the applicable engineering/certification programme and authority.

## 6. Competitive result we should seek

The credible claim is not "our AI detects aircraft damage better than every NDE vendor" or "our sensor twin makes scheduled inspection unnecessary."

The differentiated result would be:

> For a fixed set of declared structural requirements, critical regions and physical defect families, our co-design process identifies lighter/maintainable structures whose required inspections are demonstrably more feasible, selects the least-cost qualified combination of NDE and supplemental SHM, distinguishes sensor degradation from plausible structural change, and reproduces all claims from source-bound evidence at lower total lifecycle cost than a matched baseline.

This is a much harder claim, but it is testable and useful. It requires physical NDE studies and aircraft-specific authority agreement; synthetic software tests alone cannot establish it.
