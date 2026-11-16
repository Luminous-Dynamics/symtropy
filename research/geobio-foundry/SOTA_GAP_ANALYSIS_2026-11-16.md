# GeoBio Foundry — State-of-the-Art Gap Analysis

**Cut-off:** 2026-11-16  
**Status:** Literature-informed engineering analysis; not a systematic review  
**Purpose:** identify what is already demonstrated, avoid duplicate novelty claims, and select testable gaps for Luminous Dynamics.

## 1. Executive conclusion

The field is not waiting for a generic integrated lunar industrial concept. NASA's current portfolio already includes regolith electrolysis, solar-carbothermal oxygen extraction, a regolith-to-solar-products demonstrator, cooperative rover mapping, modular robotic assembly, autonomous excavation, and power infrastructure.

BioAsteroid has also demonstrated microgravity microbe–meteorite interactions on the ISS. Lava-tube researchers have published point-cloud reconstruction and finite-element stability work. Bioprocess researchers are explicitly moving toward multiscale modelling and intelligent control.

GeoBio Foundry should therefore not claim to invent integrated ISRU, robotic site mapping, modular construction, bioleaching, lava-tube FEM, or digital twins.

The defensible research opportunity is a **site-to-capability assurance loop**: preserve geological observation uncertainty; compile it into constrained candidate designs; co-account for material, energy, heat and process histories; select the next most valuable measurement; then independently reconcile prediction against physical tests and retain the evidence lineage.

## 2. Industry and research baselines

### 2.1 NASA lunar ISRU

NASA's Lunar Surface Technology portfolio reports:
- **Molten Regolith Electrolysis (MRE):** vacuum testing of a reactor that extracted oxygen from lunar-regolith simulant; metal-rich slag is a potential manufacturing feedstock.
- **Large-scale MRE demonstration:** the NASA NTRS record for the late-2024 test reports 25 kg simulant processed, 0.07 kg oxygen/hour average production, and 9 hours of sustained molten operation under vacuum. The public presentation summary does not provide a complete process-energy denominator, so it cannot by itself establish kg-O2/kWh.
- **Carbothermal Reduction Demonstration (CaRD):** NASA reports a 2025 thermal-vacuum reactor demonstration and 2026 integrated prototype testing with concentrated solar energy, carbon-monoxide extraction and oxygen production from simulated regolith.
- **Blue Alchemist:** NASA describes integrated, autonomous operation of eight technologies processing regolith simulant into silicon solar cells, aluminium wires, oxygen, iron and slag. NASA TechPort describes the integrated system as reaching TRL 6.
- **Infrastructure Pilot Excavator (IPEx):** NASA states an engineering objective of delivering on the order of 20 metric tonnes of regolith over a mission. This is a design capability statement, not a demonstrated lunar excavation rate.
- **CADRE:** NASA describes a flight-demonstration architecture with three small rovers and a base station for cooperative mapping, obstacle avoidance and ground-penetrating-radar surveys.
- **ARMADAS:** NASA is developing autonomous robotic assembly of reconfigurable structures from lightweight modular voxels.
- **M-Coor:** NASA reports TRL 6 and flight heritage for slip/embedding-detection algorithms incorporated into VIPER flight software.

**Competitive implication:** a generic proposal to integrate excavation, processing, robotic mapping, modular construction and surface power is not a sufficient differentiator. GeoBio must compare against these systems where data and equivalent test conditions exist.

Sources:
- NASA Lunar Surface Technology: https://www.nasa.gov/lunar-surface-technology/
- NASA NTRS, Large-Scale Demonstration of Molten Regolith Electrolysis for Oxygen Production (ICES 2026): https://ntrs.nasa.gov/citations/20260006167
- NASA TechPort, ISRU-Based Power on the Moon: https://techport.nasa.gov/projects/146991
- NASA, Overview: In-Situ Resource Utilization: https://www.nasa.gov/overview-in-situ-resource-utilization/

### 2.2 BioAsteroid: microgravity proof of principle, not an asteroid factory

The January 2026 npj Microgravity BioAsteroid paper reports microgravity bioleaching tests of 44 elements from L-chondrite meteorite material on the ISS. It reports a 5.5-fold palladium-release effect for Penicillium simplicissimum relative to the abiotic control, with the reported yield at 549.3 ± 234.4% of the non-biological control.

Critical experimental context:
- meteorite sample aliquots averaged 0.79 ± 0.14 g;
- the material was crushed to approximately 1–3.5 mm fragments;
- each sample received 5 mL of 50% v/v R2A medium;
- the medium included yeast extract, peptone, casamino acids, glucose, starch, sodium pyruvate and mineral salts;
- the authors report substantial variability and limited replicates;
- the paper's economic illustration estimated only about $10 of palladium value for a hypothetical 1000 m³ scaled tank under its experimental assumptions, and states that near-term profitability is unlikely under terrestrial economic comparison.

**GeoBio response**
1. Reproduce the published BioAsteroid conditions and the abiotic controls before trying to optimize organisms.
2. Report reagent/medium mass and volume as inputs, not hidden laboratory overhead.
3. Measure recovered product after separation and purification, not only dissolved concentration in leachate.
4. Compare biological, abiotic and hybrid routes on the same mineral batch and functional unit.
5. Report confidence intervals, independent batch variance, detection limits and the resource cost of maintaining the culture.
6. Treat resource self-sufficiency as a possible mission utility, not an established profit claim.

Source: https://www.nature.com/articles/s41526-026-00567-3

### 2.3 Biomanufacturing is becoming a process-systems problem

Two 2026 reviews sharpen our direction:
- A September 2026 review in Biotechnology Advances describes bioreactor scale-up as a spatiotemporal heterogeneity problem coupling hydrodynamics and cellular physiology, and argues for multiscale modelling and intelligent control that can exploit environmental histories rather than assume ideal mixing.
- An October 30, 2026 precision-fermentation review argues that strain performance alone does not guarantee industrial scalability; process intensification, process analytical technology, soft sensors, AI-enabled control, downstream recovery, techno-economic analysis and life-cycle assessment must be integrated.

**GeoBio response**
- Make reactor spatial and temporal histories explicit.
- Separate measured sensor variables from inferred soft-sensor states.
- Include downstream separation, sterilization, feedstock conditioning, utilities and waste management in the functional-unit boundary.
- Score a process on recovered on-spec product and whole-system resource burden, not titre or extraction ratio alone.

Sources:
- https://www.sciencedirect.com/science/article/pii/S0734975026001059
- https://link.springer.com/article/10.1007/s00253-026-14051-1

### 2.4 Lava-tube geometry and stability are already active research

Published work includes:
- 2025 high-resolution point-cloud reconstruction of a terrestrial lava tube followed by finite-element analysis of geometric effects on stability;
- a January 2026 study of point-cloud morphological analysis and surface reconstruction for extraterrestrial lava tubes;
- a January 2026 finite-element proposal to reshape parts of a lava-tube roof to improve selected numerical stability metrics;
- a 2026 three-dimensional finite-element limit-analysis study of collapse-pit geometry above irregular lava-tube models.

These are meaningful numerical and geometric foundations, but they do not prove that a particular lunar tube is accessible, mechanically safe, pressure-tight, or suitable for occupation.

**GeoBio response**
- Benchmark against point-cloud reconstruction and conventional deterministic FEM, not against a straw-man static cylinder.
- Preserve uncertainty in geometry, fractures, material properties, boundary conditions and sensor coverage.
- Validate blind predictions against independent terrestrial analogue observations.
- Report false-safe predictions separately from false alarms.
- Treat roof reshaping as a model-specific hypothesis subject to independent geotechnical review, not a generic intervention recommendation.

Sources:
- https://www.mdpi.com/2079-9292/14/15/3062
- https://www.sciencedirect.com/science/article/pii/S0273117726000797
- https://www.sciencedirect.com/science/article/pii/S0273117725012955
- https://www.sciencedirect.com/science/article/pii/S0094576526005138

### 2.5 Small-body robotics: mission-chain robustness is the right objective

The 2026 SmartBot review emphasizes coupled mobility, sampling and anchoring. Flight heritage supports short-contact/rapid-retreat approaches, but those methods remain limited in sample depth and throughput. It recommends progressing toward sustainable repeated task chains with reliable material handling and retreat.

**GeoBio response**
- Benchmark approach → contact → stabilize/anchor → sample → disengage → retreat → verify.
- Score net mission success, sample recovery, impulse, attitude disturbance, failed anchoring, and retreat margin together.
- Prefer reversible engagement and recoverability over maximum single-anchor force.

Source: https://onlinelibrary.wiley.com/doi/10.1002/smb2.70032

### 2.6 Planetary protection is a design constraint, not a final checklist

COSPAR's 2026 policy introduced a unified policy for icy worlds and updated the structure of its planetary-protection policy. GeoBio site records must therefore carry destination-, mission- and activity-specific preservation and contamination constraints.

Source: https://cosparhq.cnes.fr/cospar-policy-on-planetary-protection/

## 3. Where GeoBio should be differentiated

| Existing frontier | What is already demonstrated or actively developed | GeoBio's testable extension |
|---|---|---|
| Lunar ISRU | MRE vacuum extraction; CaRD; Blue Alchemist; resource system studies | Full functional-unit resource and exergy ledger, uncertainty-aware site constraints, byproduct qualification, and comparison of routes at matched purity |
| Geological mapping | LiDAR/point-cloud reconstruction, GPR and cooperative rover mapping | Preserve coverage gaps, calibration, correlated sensor errors, and expected decision value; blind transfer to unseen sites |
| Underground stability | Real-scene FEM and 3D limit analysis | Probabilistic failure prediction, model discrepancy, calibration, independent analogue truth and false-safe accounting |
| Robotic construction | ARMADAS modular voxel assembly and IPEx excavation design | Construction plans generated from measured geology and explicit pressure-boundary/utility interfaces |
| Bioleaching | ISS microgravity bioleaching proof of principle | Reproducible batch-level comparison including media, purification, energy, water, and nutrient make-up |
| Bioprocess control | Multiscale and intelligent-control research | Evidence-qualified autonomy with independent sensor residuals, out-of-domain abstention and safe process recovery |
| Small-body robotics | Flight heritage for short-contact/retreat and integrated sampling concepts | Mission-chain benchmark with explicit reversibility and recovery |
| Industrial assurance | Component-specific qualification and engineering reviews | A cross-domain evidence bundle linking site, model, process batch, instrumentation, resource claim and independent replay |

This is a differentiated research hypothesis, not a novelty or freedom-to-operate conclusion. A formal prior-art review is still required before publication or IP claims.

## 4. Paradigm shifts to pursue

### 4.1 Geological prospecting becomes value-of-information optimization

Optimize which observation to collect next, not only how much data to collect. Choose among scans, assays, boreholes, seismic measurements and material tests according to their expected reduction in decision regret, their uncertainty, resource cost, and the downstream design decisions they affect.

### 4.2 Co-design matter, energy, heat and exergy

A process can conserve energy while degrading its useful work potential. GeoBio should track gross energy, recovered energy, temperature levels, heat rejection, conversion efficiency and exergy destruction separately. Do not assume low-grade waste heat can replace high-temperature process heat.

### 4.3 Optimize production portfolios, not individual organisms

Bioleaching, abiotic chemistry and hybrid sequences may each win for different elements, mineralogies, feedstocks and energy contexts. The optimizer should select the best qualified route for a specified product and site, not assume that biology is always better.

### 4.4 Optimize environmental histories, not ideal steady states

Rock masses experience load histories; pressure shells experience cycling; microorganisms experience gradients; robots experience transient contact. Benchmarks must include histories, transients, damage accumulation and recovery—not only steady-state snapshots.

### 4.5 Make reversibility and abstention explicit system capabilities

A robot should preserve retreat options; a planner should abstain when out of domain; a resource ledger should reject a missing assay; a pressure controller should isolate uncertain compartments. These are measurable capabilities, not failures to optimize.

### 4.6 Replace “closed-loop” marketing with measured closure

Every relevant element and water stream needs measured inventory, recovery, purge, make-up and uncertainty. If any material stream is unknown, call the loop partially characterized or unquantified—not closed.

## 5. Updated research priorities

1. Resource and exergy accounting with conservative inventory completeness.
2. Geological observation provenance, uncertainty and value-of-information.
3. Pressure-compartment leakage and isolation reference models.
4. BioAsteroid replication and matched abiotic/biological/hybrid comparisons.
5. Geomechanics and robotic contact-sequence benchmarks.
6. Cross-site design compiler after individual reference models have independent tests.

## 6. Explicit claims discipline

**Supported by sources**
- MRE oxygen extraction from regolith simulant has been demonstrated in a vacuum test.
- NASA reports a 25 kg, 9-hour MRE demonstration with 0.07 kg/hour average oxygen production.
- NASA reports integrated prototype activity for CaRD and Blue Alchemist, and development programmes for CADRE, ARMADAS and IPEx.
- Microgravity microbial bioleaching of L-chondrite material was tested aboard the ISS at small experimental scale.
- Point-cloud and finite-element analysis of lava-tube geometry is active published research.
- Small-body sampling/mobility/anchoring coupling is a recognized challenge.
- 2026 bioprocess reviews emphasize scale-up, sensor/control reliability, process integration and techno-economic assessment.

**Not demonstrated by those sources**
- A safe pressurized lunar lava tube.
- An industrially profitable asteroid platinum-group-element mine.
- A self-sufficient extraterrestrial biological factory.
- A universal geological construction method across caves, craters, canyons and asteroids.
- Flight-qualified autonomous industrial operation by GeoBio Foundry.
- GeoBio superiority over NASA, ESA, JAXA or commercial systems.

## 7. Decision

Build a reproducible resource-accounting reference kernel and benchmark protocol first. Use Symtropy terrain and basin as existing simulation foundations; do not duplicate them. Keep the first package isolated from production. The next physical milestone should be an uncrewed analogue with independent metrology, pressure/thermal instrumentation and controlled process equipment.
