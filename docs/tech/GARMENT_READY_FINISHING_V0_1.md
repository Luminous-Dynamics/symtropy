# Garment Ready-State, Finishing and Robotic Handling Benchmark v0.1

**Status:** benchmark-contract proposal; no physical observations or performance results.  
**Tracker:** [GARMENT-READY-001 #5561](https://github.com/Luminous-Dynamics/symtropy/issues/5561).  
**Composes with:** laundry service #5471, care morphology #5518, professional care automation #5519 and material passport #5549.

## 1. Paradigm shift: optimize for garment readiness, not dry mass alone

A low residual moisture value is not itself a successful garment-care outcome. The useful output is a garment ready to wear, store or iron. These are different terminal states, with different moisture, shape, appearance and energy requirements.

Process chain:
`spin/extraction → drying endpoint → wrinkle/shape management → optional steam/press → inspect → fold/hang/sort → store/return`.

Explicit endpoint classes:
- `ready_to_wear`: dry enough for immediate use with declared appearance/shape acceptance, without needless over-drying.
- `ready_to_store`: moisture and condition suitable for the declared storage environment; not a sterilization or hygiene claim.
- `ready_to_iron`: controlled residual moisture/conditioning for the selected ironing process; downstream energy counts in the service total.

Endpoint class is chosen before execution or explicitly changed by the user. Never over-dry for a dryer-only score when the requested endpoint is ready-to-iron.

## 2. Why measure the whole chain

The European Commission washer/washer-dryer preparatory study describes coupled effects: higher spin can reduce drying time but increase wrinkles/ironing effort; too-low final moisture can increase ironing effort; re-wetting at ironing adds water that must be evaporated; storage delay can increase wrinkling.

Reference: https://ecodesign-washing-machines.eu/ewm-wAssets/docs/env-sr3-hh-washing-machine/20251210_EC_Report_ESPR_SR3_D3_Tasks1-7_v2.pdf

These are literature signals, not results from this programme. Freeze a workload and measure the washer → dryer → finish chain.

## 3. Appearance-method standards watch

ISO 15487:2018 remains the published predecessor. ISO/DIS 15487 Edition 4 is a 2026 Draft International Standard under development and intended to replace the 2018 edition. Its scope includes colour change, pilling, fuzzing, matting, flat-fabric/seam smoothness, pressed-crease retention and component damage after domestic washing/drying.

References:
- Current draft: https://www.iso.org/standard/89247.html
- Published predecessor: https://www.iso.org/standard/67602.html

Record the exact edition/status used. Do not call the DIS a published International Standard. Visual ratings are not hygiene or cleaning-efficacy tests.

## 4. Robotic folding: why a fair benchmark is missing

Epoch AI's 2026 review describes laundry-folding demonstrations but notes that robots can be up to 10× slower than an average human. It cites Physical Intelligence demonstration claims of 95% success on T-shirts/shorts and 75% for harder jeans-type items.

A Sunday Robotics ACT-2 preview reports 99.1% over 785 attempts across its garment categories. This is a vendor-published preview, not an independently audited, matched comparison. The trial sets, task definition, garment mix, human interventions and time accounting may differ.

References:
- Epoch AI: https://epoch.ai/publications/where-autonomy-works-evaluating-robot-capabilities-in-2026
- Sunday Robotics ACT-2 preview: https://www.sunday.ai/blog/act-2-preview

These values are not directly rankable. Build a reproducible, same-cohort benchmark rather than copying a vendor success percentage.

## 5. Benchmark work

### A. Ready-state endpoint control

Measure moisture distribution as well as mean mass loss, including seams, cuffs, pockets, waistbands, layered regions and thick areas. Bind each run to endpoint class, target range, load mix, garment mass/shape, ambient temperature/humidity, programme, transfer/storage delay, sensor calibration and filter/heat-exchanger loading.

Compare timed programmes, conventional moisture sensing, garment-aware endpoint control, heat-pump drying and bounded steam/air finishing. Include washer spin energy, vibration/noise, textile wear and downstream finishing energy. Count steam, spray, pressing and ironing water/energy; do not hide re-wetting.

### B. Wrinkle and appearance restoration

Compare tumbling, controlled steam/air, low-force tension/forming, pressing and combinations. Measure wrinkle/crease and smoothness rating, colour, pilling/fuzzing/matting, dimensional change, pressed-crease retention, seam/lining/trim/button/zip/adhesive damage, tensile/tear strength, final moisture, time, energy, water, labor and rework.

Stratify woven shirts, cotton knits, synthetics, trousers, denim, wool/delicates, structured jackets, blends and layered constructions. Refresh is not stain removal or dry cleaning.

### C. Robotic garment handling

Evaluate a whole task: pick from pile/basket/hanger/conveyor; singulate/identify; unfold/flatten/orient; fold or hang to a declared geometry; sort/place; recover from failed grasp, occlusion, tangle and mixed load; detect damage and abstain on unsupported garments.

Use the same randomized garment cohort and environment for robot and human. Report:
- first-pass and eventual accepted success by garment class/difficulty;
- final fold geometry/quality and placement accuracy;
- P50/P90/P95 completion time and accepted garments/hour;
- human interventions per 100 garments and recovery time;
- dropped, lost, mis-sorted, stretched or damaged garments;
- robustness to pile/basket/conveyor, lighting, colour, background and overlap;
- robot energy, noise, guarded footprint, serviceability and uptime;
- setup, rework and supervision in cost per accepted garment.

Never compare success percentages from different cohorts or task definitions.

### D. Modular finishing cell

Prefer purpose-built fixtures/rails over assuming a humanoid robot is necessary. Evaluate low-force manipulation, adjustable sleeve/trouser/torso forms, steam/air, heat-pump dehumidification, optional pressing, inspection, fold/hang/sort/return, manual recovery and maintenance access.

## 6. Proposed targets (hypotheses only)

| Metric | First target | Stretch direction |
|---|---:|---:|
| Endpoint first-pass acceptance | ≥95% in validated scope | ≥99% |
| Unnecessary post-endpoint drying energy | ≥20% reduction vs matched endpoint control | Minimize without appearance/storage penalty |
| Appearance | Non-inferior to matched control under frozen ISO edition | One grade improvement in selected wrinkle classes |
| Core robot fold first-pass success | ≥95% | ≥99% broader supported set |
| Robot P50 time | ≤2× skilled-human median on identical set | Human parity |
| Human interventions | ≤1 per 100 simple garments | ≤0.2 per 100 |
| Garment loss/damage | 0 in frozen challenge set | Maintain in longer field exposure |

Establish human timing variance, garment cohort, endpoint thresholds, measurement uncertainty and a preregistered scoring rubric before freezing gates. Zero observed damage still requires sample-size/confidence reporting.

## 7. Functional units and hard gates

Use **per accepted garment** for appearance, manipulation, handling, damage and cost; also report **per kg accepted dry-equivalent textile** for washer/dryer/finish energy, water, chemistry and throughput.

Hard gates:
- parent cleaning and rinse acceptance;
- endpoint moisture distribution;
- appearance, dimensional/colour/strength and component integrity;
- no lost, damaged or untracked garments;
- independent physical guarding, steam/heat/electrical protection and interlocks;
- emergency stop and manual recovery;
- offline essential operation and privacy;
- no hygiene/sterilization claim from steam, drying or appearance;
- Symthaea advisory-only, no physical safety authority.

No composite score compensates for failed cleaning, residue, material-care or safety gates.

## 8. Evidence rules

Separate literature/vendor references from our results. Separate simulation, desk evidence, analytical models, bench measurement, field evidence and independent replication. Preserve failed attempts, interventions, rework and timeouts. Contract PASS qualifies the research specification only, not a robot or finish appliance.

**Claim ceiling:** benchmark contract only. No endpoint, robot or finishing performance has been physically qualified by this programme.

## 4A. Updated state of the art: separate commercial linen folding from household generalization

Epoch AI's February 2026 review identifies DYNA as the strongest evidence of sustained commercial textile folding in its review:
- DYNA-1 completed a reported 24-hour continuous run folding 850+ napkins at roughly 60% of human speed, 99.4% success and zero interventions.
- Epoch reports a Monster Laundry commercial deployment folding 200,000+ towels across 10 clients over three months, with 99% quality acceptance.
- Epoch reports DYNA shirt folding at roughly 40 shirts/hour.
- Epoch explicitly limits the commercial evidence to relatively uniform items (towels/napkins), not the full range of household clothing.

Sunday Robotics' July 2026 ACT-2 preview reports 99.1% zero-shot folding success across 785 attempts, nine garment types and unseen home environments, with zero per-home adaptation. It describes a two-stage annotation process with a fixed five-star quality rubric. This is a vendor-published evaluation, not independent replication.

Physical Intelligence demonstrations cited by Epoch report 95% success for T-shirts/shorts and 75% for jeans. The evidence is a different task/cohort from the above.

References:
- Epoch AI review: https://epoch.ai/publications/where-autonomy-works-evaluating-robot-capabilities-in-2026
- DYNA Robotics: https://www.dyna.co/
- Sunday ACT-2 preview: https://www.sunday.ai/blog/act-2-preview

These external reports are valuable baselines, but they do not share a single frozen garment set, scoring rubric, timing protocol or independent test facility. Never combine their percentages into a single leaderboard.

### Revised two-tier benchmark

**Tier 1 — standardized commercial textiles**: napkins, towels, hospitality and workwear items with repeatable shape/size/material classes. Proposed first target: >=99.5% accepted quality, >=0.8x skilled-human throughput, <=0.5 interventions per 100 items, zero lost/damaged textiles in the frozen challenge. Stretch: human throughput parity or better with equal quality and service-life gates. This intentionally targets above the reported DYNA napkin baseline (99.4%, about 0.6x human speed), with matched independent human controls required.

**Tier 2 — diverse household garments**: shirts, trousers, jeans, blouses, knits, mixed piles/baskets, sizes, colours and textures. Proposed first target: >=99.5% accepted success in the frozen support envelope, <=2x skilled-human median time, and zero per-home adaptation on the held-out-home test. Stretch: >=99.9% and <=1x human P50, with broader transfer. This targets beyond the vendor-reported ACT-2 99.1% aggregate result; large enough sample sizes and confidence intervals are required to distinguish small percentage-point improvements.

Do not pool Tier 1 and Tier 2 or use a towel result to claim household garment competence. Both tiers require identical garment cohorts and task definitions for robot/human comparison; preserve failures/timeouts, interventions, rework, damage, energy and availability.
