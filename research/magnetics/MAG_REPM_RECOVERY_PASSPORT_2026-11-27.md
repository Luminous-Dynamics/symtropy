# Magnetic recovery passport and closed-loop remanufacturing

**Protocol ID:** MAG-REPM-RECOVERY-PASSPORT-001  
**Issue:** [Symtropy #7778](https://github.com/Luminous-Dynamics/symtropy/issues/7778)  
**Revision:** 0.1-draft  
**Date:** 27 November 2026  
**Status:** evidence contract and synthetic controls; no physical recovery result claimed

## 1. Decision question

For this exact end-of-life component and material lot, what recovery routes are actually admissible, what products and residues were produced, how much mass and target elements are accounted for, what quality does each recovered product have, and has any recovered product been used in a new component whose magnetic function was independently qualified?

A recycled-mass percentage alone cannot answer those questions.

## 2. Research basis and route families

### Polymer-bonded magnets

Lee et al. (2026) report continuous hydrolysis for separating NdFeB magnetic particles from polymer-bonded magnets. The paper makes this route relevant to compatible polymer-bonded material, but eligibility depends on actual binder chemistry, additives, coatings, post-cure and residual contamination. This is not a generic hydrolysis recipe and must not be transferred to every polymer system.

### Sintered NdFeB short-loop

Hydrogen processing of magnet scrap (HPMS), hydrogen decrepitation and related hydrogen-assisted routes are research families for suitable sintered NdFeB feeds. A 2025 study reports oxygen accumulation in the grain-boundary phase as a limitation to recycled magnet properties; separating this phase after hydrogen processing improved oxygen levels in powder and supported a wider range of recycled magnet grades.

Eligibility depends on exact feedstock chemistry, coating, oxygen/impurity state, grain-boundary phase, particle-size distribution and target recovered product. “NdFeB” alone is not a route-eligibility record.

### Elemental / alloy recovery

Hydrometallurgical, pyrometallurgical and electrochemical processes are distinct route families that may be necessary when short-loop reuse is unsupported or cannot meet product specifications. Preserve outputs as salts, oxides, alloys, separated elements or other explicit product forms. These outputs do not automatically qualify as magnet feedstock.

## 3. The forward/recovery twin

```
Original material and provenance
  -> process / modification history
  -> component geometry and joining / coating / binder system
  -> magnetic function and service history
  -> disassembly and liberation
  -> scoped recovery-route eligibility
  -> recovery stage mass / element flows
  -> recovered product form and measured quality
  -> remanufacturing material lot and build genealogy
  -> as-built component functional acceptance
```

The component passport records decisions that affect recovery during original design: grain-boundary diffusion, microalloying, densification, coating, binder, post-cure, adhesive, fastener and overmoulding history. This makes recovery-aware inverse design possible rather than bolting recycling onto the end-of-life stage.

## 4. Passport identity

Bind component and specimen IDs, source material lot, original manufacturing genealogy, REPM family, product form, grade/chemistry evidence, feedstock provenance, process/modification history, coating/binder/joining identities and service history. Values not measured or known remain explicitly unknown.

Each recovery route eligibility receipt binds the exact feedstock lot, its family/product form and the proposed route family. The evaluator consumes this receipt; it does not infer that a route is suitable from a broad material name.

## 5. Stage and mass-flow semantics

Each stage has one identified input lot and input-mass measurement, a route-family and method identity, an eligibility receipt, uniquely identified recovered products, and separately identified residual outputs.

For an input measurement interval (I=[m_i-u_i,m_i+u_i]) and all measured output intervals summed as (O), compute the residual interval (R=I-O) using conservative interval subtraction.

- (R=[0,0]): exact numeric closure (rare; measurement values and uncertainty may make this unavailable).
- (0\in R) and residual interval width within the declared closure tolerance: consistent within measurement uncertainty.
- (R.lower>0): definitely unaccounted input mass remains.
- (R.upper<0): measured outputs exceed input beyond uncertainty.
- Otherwise: unresolved mass balance.

Never silently assign unknown loss to a recovered product or normalize unexplained mass away. This v0.1 uses a fixed mass unit (mg) to avoid unit conversion ambiguity; a later adapter may add other units only with explicit conversion evidence. The v0.1 evaluator computes total mass-balance intervals only; element-specific mass balances remain a separate method-scoped assay adapter and must not be inferred from total magnet mass.

Elemental balances should be separate from total mass balances and require method-scoped composition assays. Do not infer Nd/Pr/Dy/Fe/B/Co/Ni/Cu/Al outputs from total magnet mass.

## 6. Product quality and route outcome

Product form is explicit:
- magnet feedstock / powder;
- remanufacturable alloy or consolidated magnet material;
- elemental product (salt, oxide or isolated element);
- non-magnetic separated fraction;
- residue / waste.

Recovered magnet feedstock requires composition, impurity/phase quality and magnetic characterization evidence. An elemental product requires composition and purity evidence. A waste/residue record needs a declared classification and disposition evidence. The evaluator does not treat an output label as proof of quality.

## 7. Downstream use and closed-loop claim

A recovered product becomes “used in a qualified new component” only when it is bound to:
- a new material-lot identity;
- downstream manufacturing genealogy and build/run;
- the material/product evidence consumed in that build;
- the existing as-built functional acceptance receipt for the new component.

Without that join, report recovered-product output, not closed-loop reuse. A new component pass cannot be inferred from recovered feedstock quality.

## 8. Route, lifecycle and economic benchmarks

Compare route strata only with compatible feedstock, target product form, input mass, measurement scope, facility scale and declared system boundary.

Keep these dimensions separate:
- input mass, recovered mass and unresolved loss;
- total magnetic-material yield and element-specific recovery yield;
- product-grade/purity yield and downstream qualified-use yield;
- impurity carryover and magnetic property retention;
- energy, water, reagent, emissions and waste per kg of recovered product;
- cost per kg of specification-qualified product;
- cost per qualified component or cost per torque/power/field performance;
- grade downgrade and degradation across repeated loops;
- material provenance, collection, transport, disassembly and facility utilisation.

Use explicit functional units and matched life-cycle boundaries for LCA/TEA. One kg of recovered oxide is not comparable to one kg of qualified magnet feedstock.

## 9. Evidence maturity

Use MOEM M1–M5 where applicable as an evidence-maturity description, not a formal certification:
- M1 conceptual route or model;
- M2 characterized recovery coupon / product;
- M3 controlled process–structure–property chain;
- M4 functional component using recovered material;
- M5 reproducible qualification-relevant closed loop with build/lot variability, durability and component evidence.

Do not compare a paper claim or coupon assay as though it were a functional component or repeatable industrial loop.

## 10. Benchmark ladder

- **R0:** provenance/source ledger and route-scope register.
- **R1:** synthetic passport, route-eligibility and mass-balance semantic controls.
- **R2:** characterized recovery product/coupon tests with exact methods and uncertainty.
- **R3:** repeated recovery/remanufacturing batches, process–property linkage and material-lot holdouts.
- **R4:** recovered material used in an as-built component; independent magnetic functional qualification.
- **R5:** repeated industrial cohorts, durability, process capability, LCA/TEA and end-to-end supply-chain reproducibility.

## 11. Safety and authority

This protocol grants no authority to handle hydrogen or other process gases, reactive powders, operate hydrolysis/chemical/electrochemical/thermal recovery facilities, disassemble energized or magnetized equipment, or run magnetizers or additive-manufacturing machinery. Physical work requires qualified facilities, hazard review, approved procedures, calibrated instrumentation and independent authorization.

## 12. Claim boundary

The first software tranche validates evidence semantics, identity, route scope, product forms, event sequences and conservative mass-balance arithmetic. It does not claim actual recovery yield, environmental superiority, commercial feasibility, process safety, production capability or certification.

## 13. Source register

1. Lee et al. (2026), *Sustainable recycling of polymer-bonded magnets through continuous hydrolysis for a circular economy*, Journal of Cleaner Production, 15 November 2026, DOI [10.1016/j.jclepro.2026.149829](https://doi.org/10.1016/j.jclepro.2026.149829). Study-specific PBM separation route; exact binder/feedstock scope required.
2. *Improving short-loop recycling of sintered NdFeB magnets by grain boundary phase separation following hydrogen processing*, Sustainable Materials and Technologies (2025), DOI [10.1016/j.susmat.2025.e01744](https://doi.org/10.1016/j.susmat.2025.e01744). HPMS/GBP evidence and oxygen limitation; material-specific.
3. Bai et al. (2026), *Rare earth permanent magnets in the energy transition: reconciling microstructure design with recyclability constraints*, Renewable and Sustainable Energy Reviews 241, 117244, DOI [10.1016/j.rser.2026.117244](https://doi.org/10.1016/j.rser.2026.117244). Route-aware, evidence-graded design/recovery framework, not a validated prediction model.
4. *A comprehensive review on advanced recovery of rare earth elements from NdFeB waste: Bibliometric insights and process advancements*, Journal of Rare Earths (2026), DOI [10.1016/j.jre.2026.04.026](https://doi.org/10.1016/j.jre.2026.04.026). Review of pyro/hydro/electrometallurgy and pretreatment routes.
5. *End of life permanent magnet recycling: State of the art and material flow analysis...*, Journal of Environmental Chemical Engineering 14 (2026), DOI [10.1016/j.jece.2025.120928](https://doi.org/10.1016/j.jece.2025.120928). LCA/material-flow comparator; its environmental rankings must remain bound to its system boundary.
