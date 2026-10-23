# Robust Ore-Feed Blending Benchmark v0.1

**Owner:** Symtropy  
**Canonical assay semantics:** Symthaea ORE-GEO-001  
**Campaign integration:** MINE-SYSTEM-001  
**Evidence class:** synthetic fixtures only

## Research basis

- Saavedra et al., “Blending Characterization for Effective Management in Mining Operations” (*Minerals*, 2025): https://doi.org/10.3390/min15090891. The review discusses data-driven blending under ore variability and operational constraints.
- Mwanga, Rosenkranz & Lamberg, “Testing of Ore Comminution Behavior in the Geometallurgical Context—A Review” (*Minerals*, 2015): https://doi.org/10.3390/min5020276. It describes comminution behavior as ore-specific and links energy, particle size and liberation.
- “Geometallurgy: A review of economic implications for the mining industry in a sustainability driven dispensation” (2026): https://doi.org/10.1016/j.exis.2026.101911.
- NIST AM Bench https://www.nist.gov/ambench is a methodology analogy for controlled challenge datasets, not an ore-performance reference.

No external assay values, recovery factors, or comminution parameters are included. All fixture numbers are synthetic.

## Paradigm shift

Do not rank ore lots by headline or nominal grade alone. Rank candidate blends on conservative, evidence-bounded useful-metal recovery while enforcing grade, contaminant, available-tonnage, energy and water constraints.

Nominal grade is not its lower uncertainty bound. Assay provenance is not interchangeable with lot identity. Below-detection is a bounded interval, not a measured zero. An assay is not a recovery prediction; an exhaustive oracle is not a production planner.

## v0.1 contract

Each lot records source/sample identity, exact valuable/contaminant analyte IDs bound to the scenario, assay state and detection limit, lower/upper valuable-element grade, contaminant bounds, analytical QC, dry-basis applicability, recovery bounds, and lower/upper specific-energy and specific-water units.

For a selected integer-tonne blend, the evaluator uses lower-bound grade and recovery, upper-bound contaminant grade, upper-bound energy and water, and exact selected lot masses. Lots with unresolved QC, sample/lot mismatch, unsupported basis, missing source, invalid grade/contaminant/recovery interval, invalid detection limit or inconsistent energy/water bounds are blocked.

The high-grade-first baseline is a named, deterministic heuristic. The bounded exhaustive oracle maximizes the lower-bound recovered-metal score, then minimizes worst-case energy, worst-case water and feed mass, with lexical tie-breaking. The reported recovered score is proportional to tonnes × grade-lower-ppm × recovery-lower-ppm; it is an exact synthetic objective, not a physical unit or prediction.

## Frozen cases

The 12 synthetic cases cover: high-grade-first regret under an energy cap; robust grade-bound failure; sample/lot lineage mismatch; contaminant upper-bound rejection; energy and water limits; below-detection interval semantics; invalid recovery bounds; assay QC failure; missing source provenance; and malformed detection-limit bounds, and cross-analyte substitution (for example, zinc grade in a copper-target scenario).

The independent checker reconstructs selected identities, quantities, grade, contamination, recovered lower-bound score, energy and water without calling the selector. Duplicate lots and unavailable lots fail. Search exhaustion returns an indeterminate status, never an optimality claim.

## Proposed scorecard

- 100% exact match on frozen statuses, selections and complete lot-block reason sets.
- Zero accepted blends violating grade, contaminant, energy, water, feed-capacity, availability or evidence constraints.
- 100% independent reconstruction agreement on chosen blend quantities and metrics.
- 100% deterministic replay for the same corpus bytes and source commit.
- For tiny instances whose finite search is exhausted, match the exact lexicographic oracle; if bounded search truncates, report indeterminate.
- Future measured-data comparisons must use admitted sources and lot/domain-separated holdouts, reporting assay uncertainty separately from recovery and comminution prediction error.

## Ownership and claim ceiling

Symthaea owns assay/QC/lot semantics; Symtropy owns this bounded synthetic blend reference; MINE-SYSTEM composes it into campaign metrics; Mycelix remains owner of real lot/custody/process provenance. A PASS establishes synthetic contract semantics only—not a real ore blend, reserve, saleable product, extraction recovery, cost or environmental improvement.
