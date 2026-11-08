# Independent replication protocol for mass-balance zero-discharge lettuce v0.1

**Parent:** [CEA-ROOTZONE-001 #4099](https://github.com/Luminous-Dynamics/symtropy/issues/4099)  
**Experiment design owner:** [CAVERN-ANALOG-001 #4002](https://github.com/Luminous-Dynamics/symtropy/issues/4002)  
**Element accounting:** #2277  
**Whole-facility boundary:** #3957

## 1. Why replication is the immediate frontier

Langenfeld & Bugbee (2026), *Using mass-balance principles to optimize nutrition of lettuce*, Frontiers in Plant Science 17, DOI 10.3389/fpls.2026.1885837, report 25 crop cycles over 707 days without replacing nutrient solution, with most measured element recovery around 100 ±10%. But the original methods report one eight-plant cultivar community per 56-L container (n=1 for each cultivar), and the authors explicitly call for future work to replicate the study and eliminate pseudoreplication. The 2026 source-case fixture records the exact study design and source-reported anomalies.

Primary article: https://pmc.ncbi.nlm.nih.gov/articles/PMC13500615/

## 2. The experimental unit

The treatment is applied to the nutrient reservoir / hydraulic loop. Therefore:
- independent reservoir loop = experimental unit;
- plants in the same reservoir = subsamples;
- repeated crop cycles in the same reservoir = repeated measures;
- multiple sensors on the same shared reservoir do not create independent biological replicates.

Randomize each management policy across independent reservoirs and block for chamber/rack/environment. Estimate between-reservoir variance in a pilot, then perform prospective power analysis around one pre-registered primary outcome. Do not use plant count or repeated crop cycles to inflate independent n.

## 3. Four management policies plus external comparator

A. Conventional EC management with a declared solution replacement / purge policy.  
B. Fixed mass-balance recipe using the source's 3 g dry biomass/L transpired-water starting assumption.  
C. Adaptive mass balance updated from measured WUE, tissue composition, crop stage and environmental observations.  
D. Hybrid adaptive mass balance with ISE / laboratory sentinel assays triggered by a residual or uncertainty threshold.  
E. Always-on ISE ion control as a metrology-intensive comparator where the sensors are qualified.

These arms must use the same crop density, seed lot, crop environment, source water, crop management, measurement schedule and harvest acceptance rules wherever the contrast requires them. The purpose is not to assume one paradigm wins; it is to establish the conditions where each remains valid.

## 4. Stage-gated campaign

**Stage 0 — reference admission:** reproduce the source recipe and accounting boundary; qualify stock composition, pH reagents, source water, calibration and assay detection limits.

**Stage 1 — independent DWC replication:** multiple separate reservoirs per arm, random allocation, independent calibration, three cultivars (or a pre-registered cultivar set), repeated crop cycles. Maintain the source's high-DLI environmental condition as a reproduction arm but log all climate covariates.

**Stage 2 — environmental transfer:** hold out a distinct indoor CEA environment with independently controlled VPD, CO2 and DLI. The 3 g/L WUE prior must be treated as a hypothesis, not a hard-coded constant. Measure transpired water and dry biomass for each independent loop/cycle.

**Stage 3 — recipe adaptation:** compare fixed recipe against adaptive WUE/tissue update and residual-triggered ISE / lab measurement. Keep the measurement decision rule frozen before the holdout data are revealed.

**Stage 4 — system transfer:** evaluate substrate/NFT only after DWC performance is independently qualified. Substrate adsorption/desorption, hydraulic residence time and sample recovery need distinct accounting terms.

**Stage 5 — biosecurity integration:** reuse #4099's separate authorized sampling / hazard evidence protocol. Nutrient balance is not pathogen testing, produce release or food-safety certification.

## 5. Element and water balance contract

For every reservoir/crop cycle, record initial and final solution inventory, all refill water, nutrient salts, pH reagent contributions, tissue export by roots/young/mature leaves, analytical samples, filter/precipitate retention, substrate storage where applicable, purge/overflow/leak/discharge, and any measured gaseous loss. Unknown streams remain unknown; do not silently close the balance by assuming zero.

Use explicit analyte basis (e.g. nitrate-N vs nitrate ion), units, assay, limit of detection, calibration, uncertainty and sample custody. The source article's pH-control solution contributes nitrogen and sulphur and must remain in the input accounting.

## 6. The source paper's key negative controls

- WUE varies with cultivar and environment; the paper cites potential values up to 6 g/L under elevated CO2. This is not a measured result of its own elevated-CO2 trial, but a required transfer challenge.
- Zn recovered at 116% was attributed to contamination.
- Mn recovery was 50%; Mo 86%; Si 88%. These are unresolved balance anomalies, not positive efficiency.
- Some tissue/solution values in Study 3 were estimated or interpolated because samples were missing. Our primary replication should not interpolate primary outcome data silently.
- The original high DLI (25–40 mol m^-2 d^-1) is not assumed to represent commercial vertical farms.
- Young-leaf-only samples do not represent all elements in whole-plant uptake; immobile elements such as Ca and Mn are concentrated differently in mature tissue / roots.

## 7. Acceptance and reporting

Report separately:
- accepted fresh and dry biomass per cycle and per calendar time;
- tissue nutrient sufficiency / toxicity and tipburn;
- water input, transpiration, discharge / purge and treatment residuals;
- fertilizer inputs and element-specific mass recovery with uncertainty;
- WUE per independent reservoir/cycle;
- calibration and cost of ISE/laboratory sensing;
- energy and maintenance costs for aeration, pumping and dosing.

A mass-balance claim must include a residual and uncertainty for every measured element. Recovery above 100% is a diagnostic flag; a deficit is unexplained loss or storage until reconciled. Zero liquid discharge is not proof of zero gaseous nutrient emission or microbial safety.

## 8. Proposed validation ambitions

After pilot variance and assay uncertainty are frozen:
- demonstrate replicated crop yield and tissue adequacy under adaptive mass balance on independent reservoir units;
- target 90–110% closure for analytes where the measurement chain supports that uncertainty, with analyte-specific exceptions declared rather than averaged away;
- no false claim of superiority from pseudoreplicated or incomplete data;
- demonstrate transfer to at least one held-out environment or cultivar, otherwise mark the model outside its qualified domain;
- show adaptive or hybrid sensing improves reliability enough to justify its lifetime calibration/maintenance cost;
- preserve no-discharge as a qualified result only when water and element inventories, all inputs, outputs, residuals and appropriate environmental/fate measures are complete.

These are proposed validation targets, not achieved performance.

## Claim ceiling

This protocol qualifies independent experimental design and source-bounded mass-balance replication only. It does not prove zero nutrient emissions, certify produce, approve wastewater reuse, qualify other crops or substrates, or authorize microbial challenges.