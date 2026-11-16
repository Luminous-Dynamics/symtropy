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


## 4E. Reuse standardized cloth-manipulation benchmarks instead of reinventing them

The RAL-SI **Benchmarking Bimanual Cloth Manipulation** resource already defines cross-platform tasks for tablecloth spreading, towel folding and partial T-shirt dressing. It includes multiple starting configurations (two-point grasp, one-point grasp, flat, folded and crumpled), complexity levels and scoring for phase success, execution time, force and task quality.

Reference: https://ral-si.github.io/cloth-benchmark/

This is a useful lower-level calibration layer, not an end-to-end laundry service benchmark. Its towel-fold completion criterion can be as small as one fold, so it does not measure final table-ready stack placement, commercial throughput or garment-ready acceptance. We should reproduce its tasks and scoring where feasible, then add the missing end-to-end acceptance steps rather than claim to invent the field's first cloth benchmark.

A February 2026 systematic review of 41 cloth-manipulation papers (2019–2024) finds that 70.7% focus on folding/unfolding only; 53.7% manipulate towels/napkins, while 29.3% address garments. It identifies diverse initial states, cloth-property simulation fidelity, Sim2Real, real-world dataset availability, multimodal sensing and inconsistent metrics as open problems.

Reference: https://doi.org/10.3389/frobt.2026.1752914

Our benchmark should therefore have three layers:
1. **Primitive comparability:** replicate RAL-SI spread/towel-fold/partial-dressing tasks, their starting states, success phases, time, force and quality metrics.
2. **Garment generalization:** hold out garment families, fabric properties, initial states, home/site, lighting and hardware revisions. Include RGB-D/point-cloud and force/contact channels where available; treat simulation-only results as simulation.
3. **Service acceptance:** add quality inspection, correct bin/stack/destination, downstream-ready state, recovery, intervention, damage, throughput and 18-hour availability.

A further physical-AI opportunity is adaptive primitive selection: choose between fling, pick/place, drag/mop, regrasp and human handoff according to garment state and uncertainty. The systematic review describes these primitive families and the need to combine them rather than expect a single policy to handle every cloth geometry. A classifier should not force a high-risk action when state estimation is weak; abstention and recovery are part of the capability.

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

### Ready-state endpoints
- First-pass endpoint acceptance: >=95% in the declared and validated garment scope; stretch >=99%.
- Unnecessary post-endpoint drying energy: >=20% reduction against matched conventional endpoint control without an appearance, material or storage penalty.
- Appearance: non-inferior to a matched control under the frozen ISO 15487 edition; explore a one-grade improvement in selected wrinkle classes.
- Moisture: no hidden wet regions; report local seam/cuff/pocket/lining moisture, not only load-average mass loss.

### Tier 1 — standardized commercial textiles
Current comparator: DYNA's August 2026 company report says Dyna-2 folds about 95 napkins/hour, 93% meet its table-ready quality bar and approximately 1,590 table-ready napkins are produced in an 18-hour shift. DYNA describes correctly assigned bin placement and organized stacks as part of the finished task. This is a company-reported production baseline, not independent Luminous validation.

- First target: >=98% first-pass quality acceptance, >=100 correctly folded and stacked/table-ready items/hour, >=1,800 per 18-hour shift and >=99.5% correct destination placement.
- Stretch: >=99.5% first-pass quality acceptance, >=120 accepted items/hour, >=2,160 per 18-hour shift and >=99.9% correct destination placement.
- Human throughput ratios must use the same frozen items, station presentation, shift length and acceptance rubric. No human-speed claim is made from DYNA's company-reported throughput alone.

### Tier 2 — diverse household garments
- First target: >=99.5% accepted success in the declared support envelope, <=2x skilled-human median time on the same garment set, and zero per-home adaptation on held-out-home tests.
- Stretch: >=99.9% accepted success and human P50 time parity, with broader unseen-garment and unseen-home transfer.
- Keep shirts, trousers, jeans, blouses, knits, layered garments and mixed piles stratified. Never pool these into the standardized napkin/towel rate.

### Shared handling and reliability
- Human intervention: <=1 per 100 simple garments in the declared core set; stretch <=0.2/100.
- Garment loss/damage: zero observed in the frozen qualification cohort, with a confidence bound and sample size reported.
- Report P50/P90/P95 time, accepted throughput, rework, intervention, placement, uptime, maintenance, energy and cost per accepted output.
- All targets are proposals. Freeze workload, human skill distribution, quality rubric, non-inferiority margins, uncertainty and sample sizes before confirmatory evaluation.

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

## 4A. Current production frontier: Dyna-2 and the complete textile workflow

An August 2026 DYNA company report materially updates the earlier Dyna-1 demonstration baseline. The company reports:
- Dyna-1 was producing around 35 napkins/hour, with 75% meeting the company's table-ready quality bar.
- Dyna-2 now folds around 95 napkins/hour, with 93% meeting that quality bar, yielding approximately 1,590 table-ready napkins in an 18-hour shift.
- The production task includes precise placement into one of ten instructed bins/stacks. DYNA describes the earlier failure mode: napkins were folded but placed disorganized, leaving staff to square stacks before carrying them to tables.
- The article reports a larger commercial rollout and claims the system crossed its ROI threshold. These are company-reported operational figures and claims, not an independent Luminous evaluation.

Primary source: https://www.dyna.co/research/scaling-customer-deployments (DYNA, August 2026; Dyna-2 production and stack/bin steerability).

This is a stronger commercial comparator than the earlier Dyna-1 lab demo. Do not mix the Dyna-1 99.4% / 850-napkin demonstration metric from Epoch AI's earlier review with Dyna-2's current table-ready production quality rate; the task definitions and quality bars differ.

### Paradigm shift: fold success is not service success

The end-to-end output is **a correctly folded item, assigned to the correct stack/bin, organized for downstream pickup and accepted without human correction**. A correct fold in the wrong bin or a disorganized stack is a failed production outcome.

Benchmark the entire cell:
`receive → singulate → identify → unfold/flatten → fold → inspect → assign stack/bin → organize stack → mark ready for pickup`.

Measure quality and throughput at the final accepted output boundary, not at the first visually recognizable fold.

## 4B. Two non-pooled robotics suites

### Tier 1 — standardized commercial textiles

Cohort: napkins, towels, hotel/workwear linen items with repeatable geometry, size and material classes. Include the actual production input state (e.g. unfolded, pre-oriented, conveyor/basket), folding recipe, bin count, stack-height limits and pickup/transport requirement.

**Proposed first target:** at least **1,800 accepted, correctly stacked/table-ready items per 18-hour shift** (100 accepted items/hour), at least **98% first-pass quality acceptance**, and at least **99.5% correct stack/bin placement**.

**Stretch target:** at least **2,160 accepted items per 18-hour shift** (120/hour), at least **99.5% quality acceptance**, and at least **99.9% correct stack/bin placement**.

These targets are deliberately above DYNA's reported Dyna-2 baseline of ~1,590 table-ready napkins/18h and 93% meeting its quality bar. The comparison is still not a matched head-to-head; replicate with a frozen cohort, matched human process and identical table-ready rubric.

Also report: raw items/hour, accepted items/hour, rejected/reworked items/hour, P50/P95 cycle time, stack capacity, misplacement rate, stack squareness/transport readiness, 18-hour uptime, maintenance downtime, intervention rate, energy and cost per accepted table-ready item.

### Tier 2 — diverse household garments

Cohort: shirts, trousers, jeans, blouses, knits and mixed garments across size, colour, material, texture and initial state (pile, basket, hanger, inside-out, sleeves inverted, partially folded and overlapping). Hold out entire homes and garment families, not just camera frames.

**Proposed first target:** at least **99.5% accepted success** within the frozen support envelope, no per-home adaptation on held-out homes, and no more than **2× skilled-human median time** on the same garment set.

**Stretch target:** at least **99.9% accepted success** and human P50 time parity, with broader unseen-home and unseen-garment transfer.

Sunday Robotics ACT-2's July 2026 vendor preview reports 99.1% zero-shot success over 785 attempts, nine garment types and unseen homes. This is a vendor-published result, not independent replication. Epoch AI's February 2026 review cites Physical Intelligence demonstrations at 95% for T-shirts/shorts and 75% for jeans. These are different cohorts and are not directly rankable.

References:
- Epoch AI review: https://epoch.ai/publications/where-autonomy-works-evaluating-robot-capabilities-in-2026
- Sunday ACT-2 preview: https://www.sunday.ai/blog/act-2-preview

## 4C. Physical-AI research opportunity: unified garment manipulation

A February 2026 systematic review examined 41 cloth-manipulation papers published from 2019–2024. It identifies irregular sizes, diverse initial states, simulation-to-real mismatch, limited real-world datasets, over-reliance on vision-only sensing and inconsistent folding metrics as open challenges. It also proposes a unified end-to-end policy that transforms random initial garment states into an accepted folded result instead of training unfolding and folding as disconnected tasks.

Reference: https://doi.org/10.3389/frobt.2026.1752914

Our research direction should be:
- one end-to-end task contract from unstructured garment state through fold, sort and ready-to-pickup;
- RGB-D plus force/tactile and joint-state signals where justified, with sensor calibration and contact uncertainty;
- domain-randomized simulation with actual fabric mass, bending stiffness, friction, drape and stretch distributions;
- real-world held-out garment and home cohorts; never call simulation success physical validation;
- explicit recovery primitives (regrasp, shake/untangle, reorient, flatten, inspect and human handoff);
- quality scoring that captures hidden damage, seam stretch, missed garments and stack organization.

## 4D. Non-negotiable evaluation rules

- Keep standardized commercial textiles and diverse household garments as separate suites.
- Preserve every failure, timeout, retry, rework, intervention and dropped item.
- Human comparison must use the same items, task definition, shift length and staging conditions.
- Report final table-ready output, not fold-only count.
- Record availability, cleaning/finish handoff, maintenance and downstream staff time.
- Treat DYNA figures as company-reported production data; treat ACT-2 figures as vendor-published evaluation; treat Epoch AI as an external review. None is a Luminous result.
- A benchmark-contract PASS is not physical robot qualification.
