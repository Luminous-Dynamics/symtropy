# MAG-AM-CIRCULAR-MOTOR-LIFECYCLE-001

Date: 2026-11-28
Version: 0.1
Evidence status: protocol/evaluator only; no physical teardown trial or route-performance claim.

## 1. Paradigm shift: design for second life

The product boundary must include first-life function, service, inspection, repair, refurbishment/remanufacture and end-of-life separation. A rotor optimized only for peak torque density can externalize cost into magnet demagnetization, destructive adhesive removal, rotor replacement, contaminated streams and untraceable output mass.

Fraunhofer IPK describes reuse, repair, remanufacture, refurbishment and retrofit as second-life strategies for electric motors. It highlights loss of magnet performance through demagnetization and the importance of modular construction, disassembly and material separation. [1]

A 2026 traction-motor disassembly study maps a Tesla Model Y drive-unit component hierarchy and joining methods, then uses the map for robotic disassembly simulation. Its reported automated-time reduction is a potential supported by simulation, not a deployed production guarantee. [2]

A 2026 circularity decision-tool paper frames teardown as precedence-constrained operations connected to cleaning, diagnosis and decisions among reuse, remanufacturing and recycling. Thus the useful outcome is not mass removed, but a qualified destination for each output stream. [3]

## 2. Frozen boundary and functional unit

Every candidate and reference must share:
- application, torque-speed envelope and lifetime duty profile;
- expected lifetime energy throughput and service interval policy;
- operating-temperature, overload and fault envelope;
- rotor speed, overspeed, mechanical envelope and containment policy;
- motor-level efficiency and post-service functional acceptance;
- teardown facility, operator/tooling class and safety policy;
- mass-flow boundary, recovery destinations and material-accounting rules;
- measurement method, calibration, uncertainty and evidence-retention policy;
- objective, constraints, service-cost boundary and acceptance rules.

A different duty, lifetime output, teardown boundary or recovery definition is NotComparable. The primary functional unit is a motor assembly delivering the frozen duty and lifetime energy throughput inside the same performance and safety envelope. This v0.1 evaluator does not claim an audited LCA or universal environmental score.

## 3. Comparator strategy

At least one optimized conventional PM rotor is mandatory. Preregister every incumbent PM retention family applicable to the application:
- conventional bonded/adhesive PM rotor;
- mechanically retained segmented PM rotor;
- conventional serviceable/cartridge-based PM rotor where used.

The candidate is an AM-enabled modular/recoverable PM rotor or carrier. A serviceable control may be added.

PMaSynRM, EESM, induction and reluctance-only architectures are strategically important magnet-reduction or magnet-free alternatives. They belong in a parallel function-matched architecture tournament, not the mandatory PM recovery denominator: zero magnet mass makes a recovery fraction undefined or structurally different.

Each baseline needs an optimization receipt frozen before comparison, matching duty, objectives, constraints, search budget and independent restarts. A fairness budget is not proof of global optimality.

## 4. Mandatory post-service function and safety gates

M4 physical component acceptance must bind exact component, specimen, build and material lot. Mandatory measurements:
1. post-service torque capability within the frozen envelope;
2. lifetime energy-weighted efficiency under the exact duty;
3. temperature- and load-aware demagnetization margin;
4. magnet/rotor retention margin under speed, thermal and vibration exposure;
5. service-cycle survival or application-specific durability proxy.

Each teardown must confirm safe state before disassembly and component integrity after separation. A definitive hard-limit violation or unsafe teardown is Fail. Missing/unbound evidence, uncertainty crossing a hard limit, absent calibration or incomplete mass balance is Indeterminate. Synthetic fixtures cannot be physical evidence.

## 5. Teardown and recovery passport

Each physical trial records component/specimen/build/material-lot identity, teardown method, tool IDs, elapsed time and energy, safety confirmation, component integrity disposition, initial magnet mass and measurement uncertainty.

Output streams are mutually exclusive:
- reuse-grade magnets;
- remanufactured magnets that passed functional acceptance;
- recycling feedstock;
- reject/waste.

The mass balance must close conservatively. Let input mass interval be I and sum of output stream intervals be O. The full difference interval O-I must fit inside the predeclared absolute-plus-relative tolerance; overlap at one point is insufficient. An unclosed balance is Indeterminate, not an optimistic recovery percentage.

Reuse and remanufacture fractions are computed from receipt masses with conservative interval propagation. Recycling-feedstock fraction is reported separately. Do not call feedstock recycled material until downstream recycling yield and grade are evidenced.

## 6. Benefits, without a universal score

Preregister application-specific benefits: disassembly time/energy, qualified reuse/remanufacture mass fraction, recycling-feedstock fraction, repair/refurbishment cost per lifetime delivered kWh, module replacement cost and downtime, component retention and recovered part value.

No benefit can compensate for demagnetization, overspeed, containment, efficiency or unsafe-disassembly failure.

## 7. Decision outcomes

- NotComparable: invalid benchmark, missing mandatory PM family, scope mismatch or invalid candidate family.
- Fail: definitive post-service hard-limit violation or unsafe teardown.
- Indeterminate: missing physical evidence, M4 acceptance, unbound identity, invalid optimization receipt, uncertain hard limit or unclosed mass balance.
- InferiorToBaseline: a mandatory function misses the frozen conservative non-inferiority margin against a required comparator.
- NonInferiorNoCircularityBenefit: all function gates pass but no same circularity benefit metric robustly beats every required comparator.
- CircularityAdvantage: M4 evidence, closed mass balance, safe teardown, all absolute limits, non-inferiority against every required comparator, and strict interval-separated improvement in at least one same benefit metric against all required comparators.

This is an evaluator disposition, not product certification, production release, audited LCA, recycler qualification or a general industry claim.

## 8. Relationship to existing programme

The recovery passport validates source-lot eligibility, process genealogy, mass balance and route receipts. The manufacturing cohort evaluator protects yield denominators. The qualification-maturity evaluator limits claims by M1-M5. The architecture benchmark compares motor function against conventional PM/PM-assisted-SynRM references. This programme adds lifetime serviceability and second-life decisions, consuming those identity, evidence and recovery receipts rather than replacing them.

## 9. Standards and data crosswalk

Use applicable ISO 14040/14044 principles for future LCA claims and ISO 59020:2024 for circularity performance measurement where the boundary fits. This protocol does not claim conformity.

For AM digital thread and defect data, use the applicable scope of ISO/ASTM 52951:2026 (part data packages), 52953:2025 (metallic PBF-LB process-monitoring/QC registration) and 52948:2026 (metallic PBF imperfection classification). These are data/inspection references, not magnet reuse-acceptance standards.

A recovered magnet requires a specified magnetic-property test method, temperature, demagnetization history, geometry and uncertainty. Mass alone never establishes reuse grade.

## 10. Success target

First success is a replayable physical teardown pack, not a percentage headline: exact lifetime/service boundary; optimized comparator receipts; M4 post-service functional evidence; safe teardown and component-integrity evidence; closed uncertainty mass balance; separate reuse/remanufacture/feedstock/reject destinations; time, energy, cost and service records.

At least 3 independent builds and 2 material lots are an exploratory replication floor, not M5 production qualification. An independent team should be able to replay the mass balance and decision without trusting a vendor circularity score.

## Sources

[1] Fraunhofer IPK, Keeping Electric Motors in the Loop, FUTUR 2026-1: https://www.ipk.fraunhofer.de/en/media/futur/futur-2026-1/keeping-electric-motors-in-the-loop.html

[2] Hamidi et al. (2026), Maximising material recovery from end-of-life electric traction motors via disassembly, Resources, Conservation and Recycling 234, 109078. DOI: 10.1016/j.resconrec.2026.109078. The automated-time result is potential supported by process mapping and robotic simulation.

[3] Klein et al. (2026), Development of a Decision Tool for Assessing the Circularity of Electric Traction Motors, Procedia CIRP 140, 641-646. DOI: 10.1016/j.procir.2026.05.108.

[4] Fodor et al. (2026), Materials-Driven Additive Manufacturing for Electrical Machines: Qualification-Oriented Design Beyond Geometric Freedom. DOI: 10.1002/adem.71259.

[5] State-of-the-art additive manufacturing of NdFeB permanent magnets: challenges and pathways toward high performance, Journal of Science: Advanced Materials and Devices 11(3), 101226 (September 2026). DOI: 10.1016/j.jsamd.2026.101226.

[6] Liu et al. (2026), A review of structural design for reduced-rare-earth permanent magnet motors in new energy vehicles, Frontiers in Mechanical Engineering. DOI: 10.3389/fmech.2026.1957624.
