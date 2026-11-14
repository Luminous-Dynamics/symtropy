# Textile factory scheduling — industrial incumbent comparison v0.1

Status: benchmark protocol/template only. No Fibersort, Trimclean or Luminous production line has been measured by this repository.

Parent: [TEXTILE-MULTISTAGE-FACTORY-001 #4875](https://github.com/Luminous-Dynamics/symtropy/issues/4875).

## 1. Why the incumbent matters

Valvan publicly describes:
- **Fibersort:** NIR-based fibre/blend concentration prediction with RGB colour sorting for post-consumer textiles.
- **Trimclean:** a downstream trim-extraction system described as removing zippers, buttons and labels from fibre-sorted textiles. Its published process is important to represent accurately: Fibersort output is first cut into **20–100 mm clippings**; an optical camera system and a metal sensor detect unwanted material in the clippings; trim-bearing clippings are then pneumatically separated from textile clippings.

Sources:
- Valvan process overview: https://www.valvan.com/en/solutions/textile-sorting-recycling
- Trimclean technical leaflet: https://www.valvan.com/uploads/general/VALVAN_trimclean_leaflet-A4_08_lowres.pdf?v=1685433923

This is a real industrial process comparator, but it is a post-cutting/clipping-level trim separation route—not a whole-garment seam-ripping robot. A Luminous pre-shredder garment-disassembly system would test a different value proposition: whether components can be recovered and textile panels/fibre length preserved before shredding, with lower residual trim and damage. Its comparison must therefore include output size/length distribution, fines, recovered-component reuse, contamination, downstream grade and energy/labour per accepted kilogram.

The supplier's public descriptions do not provide an independently reproduced, matched-lot performance dataset for every feedstock, damage mode, residual-trim level, operating cost or accepted downstream grade. Do not turn supplier descriptions into validated quantitative claims.

### Trimclean capacity — supplier-stated, not a measured comparator result

The Trimclean leaflet lists **up to 1,000 kg/h maximum**. It qualifies that figure as capacity observed under tests with industrial loads in best-possible conditions; actual throughput depends on the specific weight of clippings and conveyor coverage ratio, and the stated operating context is dry textile waste at regular humidity.

This is a supplier-stated clipping-feed capacity envelope. It is **not** an independently measured Fibersort capacity, accepted recovered-textile output rate, whole-garment processing rate or guaranteed continuous throughput. In the matched benchmark, report:
- incoming clipping mass / active hour;
- total line input mass / elapsed hour;
- textile clipping output accepted downstream / hour;
- trim-bearing clipping reject/rework mass;
- uptime, conveyor coverage, clipping specific weight and moisture;
- purity and contamination of both separated streams.

Never compare 1,000 kg/h of clipping feed directly against accepted kg/h from a pre-shredder garment-disassembly process.

### Adjacent incumbent: reuse grading and plant capacity

Valvan's **Hypersort** is an adjacent comparator for whole-item reuse grading/pricing, not a like-for-like fibre-recycling sorter. Valvan advertises up to 1,200 garments/hour for an automated grading line and 50% higher throughput for its automated machine; these are supplier-stated figures that require site-specific measurement and a defined denominator (items presented, accepted grades, operators, uptime).

Valvan also describes a 55,000-square-foot Salvation Army facility processing up to 20 tonnes/hour of donated clothing and other textiles. That is a whole-facility donation-processing capacity, **not** an independently measured Fibersort-only or Trimclean-only capacity. Never use it as a direct sorter-throughput baseline.

Sources:
- Hypersort: https://www.valvan.com/en/solutions/textile-pricing-hypersort
- SATCoL facility case: https://www.valvan.com/en/cases/processes-up-to-20-tons-per-hour-of-donated-clothing

The research-review finding that attachment-removal AI is sparse concerns the published research corpus. It does not establish that commercial trim-removal systems do not exist.

### Wider industrial landscape — sorting plants and integrated recycling networks

The incumbent set must not be reduced to a single Valvan line.

| System / programme | Publicly stated scope and scale | Benchmark interpretation |
|---|---|---|
| Valvan semi-automatic Fibersort | Supplier says one textile/second; up to approximately 1,200 kg/h, derived using its estimate of about three used textiles per kg | Supplier estimate; actual kg/h depends on garment mass and presentation. Measure item/s, input kg/h and accepted output separately |
| Valvan Trimclean | Up to 1,000 kg/h maximum of clipping feed under best-possible industrial-load conditions | Clipping-level separation only; depends on specific clipping weight, conveyor coverage and dry/regular-humidity conditions |
| ANDRITZ / Nouvelle Fibres Textiles, France | ANDRITZ's October 2026 company presentation lists an automated post-consumer textile sorting installation, 5,000 t/y capacity and 2023 start-up; it describes AI-based material/colour identification and contaminant removal | Important industrial-scale sorting comparator. The public slide does not specify annual operating hours, exact boundary of the 5,000 t/y, uptime, output purity, accepted downstream grade or whether the capacity is nameplate vs achieved. Do not convert to kg/h without verified operating hours and process boundary |
| Valvan Hypersort | Supplier advertises up to 1,200 garments/hour for automated reuse grading/pricing | Whole-item reuse grading, not a fibre-composition sorter; compare in a reuse/grade lane, not against Fibersort NIR accuracy |
| Rieter / Säntis Textiles / Valvan Recycling Powerhouse | Venture announced July 2026 to standardize and scale recycling into certified yarns; first blueprint franchise planned for early 2027 in Qatar; Rieter contributes tearing/spinning, Valvan sorting including Fibersort/Trimclean | Announced future integrated competitor/partner, not an already operating global franchise network. Its end-to-end model makes accepted yarn quality, traceability and plant replication central competitive benchmarks |

Primary supplier/company sources:
- Fibersort capacity description: https://www.fibersort.com/en/products/semi-automatic-fibersort
- ANDRITZ October 2026 company presentation, page 18: https://www.andritz.com/resource/blob/105244/327cba4881dab1af236a52cd8f0caa92/download-company-presentation-april-2025-en-group-data.pdf
- Rieter Recycling Powerhouse announcement (17 July 2026): https://www.rieter.com/media/media-releases/newsdetail/recycling-powerhouse-circularity-at-industrial-scale

These figures have different denominators and evidence levels: garments/hour, clipping kg/hour and annual facility tonnes are not directly comparable. Record rated capacity, measured throughput, uptime and accepted downstream output separately. Company announcements and supplier brochures do not replace matched-lot or independent receiver evidence.

## 1.1 Commercial baseline landscape (November 2026)

The incumbent is not one machine. The comparison should include a configurable chain of feed preparation, recognition, picking/routing, trim separation and downstream fibre preparation.

| Commercial system | Publicly described capability | What must be independently measured |
|---|---|---|
| Valvan Fibersort | Fibre composition, colour and woven/knitted construction sorting; the official product page states 1 piece/s. Siemens describes automated picking at roughly 2,000 garments/hour using delta + robotic handling, with a reported 1.8 s pick cycle in its latest controller configuration. | Input presentation quality, singulation rate, double picks, overlap/mis-picks, class confusion, false accepts, output purity, uptime and actual accepted kg/h. The piece rate is not a mass throughput guarantee. |
| Valvan Trimclean | Camera + metal-sensor detection and pneumatic separation of trim-bearing 20–100 mm clippings after cutting. | Residual trim by type, textile loss in trim stream, clipping-size distribution, fines, energy/air demand, purity and downstream acceptance. |
| STADLER + TOMRA at IGERS (Italy) | STADLER reports a facility inaugurated 21 September 2026 with two dosing units, two TOMRA NIR sorters and 16 conveyors; stated capacity is 4 t/h and authorization up to 19,200 t/year. The line buffers pure fractions and recirculates material through a second NIR sort to separate fibre types. | Distinguish design capacity and authorized annual capacity from observed sustained throughput. Measure feed stability, buffer occupancy, recirculation fraction, stoppages, per-stage yield/purity and receiver acceptance. |
| Specim RETEX | Supplier-marketed hyperspectral + AI textile recognition and automated material separation; advertised coverage includes common fibres, blends, elastane, impurities and colour. | Independently reproduce across held-out physical specimens, instruments, dyes, contamination and multilayer garments. Measure false-accept risk, abstention, calibration, latency, sorting purity and accepted downstream grade. |

Primary sources:
- Fibersort capabilities: https://www.fibersort.com/
- Siemens / Valvan automation and reported picker cycle: https://www.siemens.com/en-gb/company/insights/valvan-fibersort/
- STADLER / IGERS installation: https://stadler-engineering.com/company/news/detail/from-textile-waste-to-spinning-grade-fiber-stadler-group-engineers-integrated-sorting-solution-for-igers-in-italy
- Specim RETEX: https://www.specim.com/retex/
- Trimclean technical process: https://www.valvan.com/uploads/general/VALVAN_trimclean_leaflet-A4_08_lowres.pdf?v=1685433923

Supplier product pages and partner announcements establish commercial availability and supplier-stated configurations; they are not independent comparative performance trials. Treat the 4 t/h and 2,000 pieces/hour figures as stated design/operating claims with their own denominators, not as directly comparable measurements.

### Paradigm shift: flow-aware sensing

A sensor sees a textile only when feeding has presented a measurable surface. Bulk textile handling creates its own failure modes: overlapping items, double picks, folds, entanglement, variable mass, orientation, buffer overflow and re-circulation. The benchmark must therefore separate:
1. **feed presentation:** fraction of items presented singly, flat enough and within sensor field-of-view;
2. **recognition conditional on a valid presentation:** per-class and open-set performance;
3. **pick/route execution:** whether the physical item actually enters the commanded stream;
4. **output quality:** purity, damage, mass conservation and accepted downstream grade.

Report both end-to-end correct-route rate per incoming item and conditional sensor accuracy on successfully presented items. A model must not gain apparent accuracy by dropping hard-to-present items from the denominator.

## 2. Matched process arms

### Arm A — incumbent Fibersort + Trimclean line

Observe the actual installed configuration and freeze its process map. A representative path to verify on-site is:

1. lot intake, weighing and custody assignment;
2. operator presentation / pre-sort, if present;
3. Fibersort NIR/RGB identification and material/colour routing;
4. cutting of sorted textiles into 20–100 mm clippings (per the published Trimclean leaflet);
5. Trimclean optical/metal detection and pneumatic separation of trim-bearing clippings;
6. post-separation inspection, reject/rework and output-lot weighing;
7. downstream recycler sampling and acceptance.

This is the published supplier process flow, not a claim that every installation uses identical settings or sequencing. Record bypasses, manual rework, recirculation, maintenance stops, rejects, clipping geometry and whether trim-bearing clippings are discarded, recirculated or separately processed.

### Arm B — evidence-gated line candidate

Use the same incoming lot and product acceptance criteria. The proposed differentiator is **pre-shredder disassembly at garment/panel scale**, compared with the incumbent's post-cutting clipping separation. Candidate stages may include:

1. visual garment-family/presentation assessment;
2. calibrated NIR composition estimate;
3. uncertainty and open-set check;
4. optional remeasurement or human review;
5. explicit physical routing decision;
6. attachment/trim removal by the incumbent, a guarded script or a robot only when the join/task is admitted;
7. independent post-inspection, mass accounting and output-lot creation;
8. downstream acceptance/rejection.

The sensing model does not determine recovery eligibility by itself. A physical removal outcome does not establish fibre purity or downstream acceptance.

## 3. Freeze the comparison before measurement

For each run, pin:
- source lot ID, custody and rights to test;
- garment/specimen IDs and garment-condition strata;
- fibre composition references and method/uncertainty;
- contamination, dye/finish, wetness, multilayer and attachment strata;
- item count and total incoming mass;
- line configuration, software/firmware, sensor IDs and calibration;
- machine/operator roster, skills/certifications and shift calendars;
- line availability, changeovers and planned maintenance;
- sampling policy, acceptance tests and reject criteria;
- measurement start/stop rules and treatment of downtime;
- energy, steam, water and wastewater meter identities, calibration and intervals.

Use independent incoming-lot allocation or randomized matched sublots where practical. Do not send easier garments to one arm. Preserve specimen identity across sorting, trim removal and output testing.

## 4. Minimum measured fields

| Domain | Per-run/per-item observations |
|---|---|
| Feed presentation | single-item presentation rate, double/missed pick rate, overlap, fold/occlusion, conveyor occupancy, orientation, singulation latency and recirculation |
| Throughput | items/min, kg/h, total elapsed time, active time and downtime, with the measurement boundary |
| Routing | predicted route, actual route, reject, abstain, remeasurement and manual override |
| Composition | reference method, per-fibre error, blend error, unknown fraction, instrument and calibration |
| Sorting quality | output stream purity, contamination, false accept, false reject and unassigned mass |
| Trim removal | target trim mass, removed mass, residual trim, partial/failed extraction, component identity; stage of removal (whole garment vs 20–100 mm clipping) |
| Material preservation | recovered panel dimensions, fibre-length distribution, fines/dust fraction and whether components are reusable before shredding |
| Textile damage | holes, tears, severed yarns, damaged area, strength-retention method where measured |
| Mass balance | input, accepted textile output, recovered trim, reject, residual and unexplained mass |
| Labour | operator minutes by task, intervention count/type, setup/changeover time and certification |
| Utilities | electricity, steam, water and wastewater by interval; meter uncertainty and missingness |
| Operations | machine availability, bottleneck utilization, queue time, recirculation, rework, plan changes |
| Downstream | accepted/rejected output lot, receiving process, grade and functional acceptance test |

Keep the denominators explicit. For example, purity by mass, trim-removal completion by attachment count and garment recovery by incoming mass are different metrics.

## 5. Scheduling benchmark input

Translate only measured/declared values into the independent verifier:

- each physical operation becomes an operation with a frozen duration estimate/distribution and release/due bounds;
- each machine becomes a capability + availability + sequence-dependent setup model;
- each production and setup task uses a separately assigned qualified operator;
- utility use is recorded by interval and mapped to time-varying constraints only when meter data exist;
- rework and recirculation become explicit operations/precedence edges, not hidden cycle-time inflation;
- unavailable, unmeasured or uncalibrated utility capacity remains unknown and blocks a qualified feasibility claim.

Never infer labour or energy requirements from vendor throughput alone. Do not optimize against unsupported capacity assumptions.

## 6. Required matched metrics

Report both arms and each product/fibre/condition stratum:

- accepted kg per elapsed hour and per active hour;
- output stream purity and downstream acceptance rate;
- residual trim by attachment family and process stage (whole garment vs clipping);
- recoverable component reuse and textile panel/fibre-length retention before shredding;
- fines/dust fraction and downstream spinning/yarn acceptance where measured;
- damage rate/severity and strength retention where measured;
- total labour minutes and intervention burden per accepted kg;
- electricity, steam and water per accepted kg;
- reject, rework, recirculation and unexplained mass;
- uptime, bottleneck utilization and schedule stability;
- cost per accepted kg only when the cost boundary and depreciation assumptions are explicit.

For scheduling policies, additionally report feasibility rate, makespan, tardiness, throughput, operator utilization/workload, setup time, utility peaks, plan churn after disruptions, lower-tail performance and exact-oracle optimality gap on small instances.

## 7. Benchmark controls and anti-gaming rules

- Same incoming garments/lots, acceptance tests and environmental conditions across arms.
- Keep garment/specimen identities grouped across train, calibration and final test splits.
- Report per-category and worst-stratum results; do not use overall accuracy to hide weak blends or attachment types.
- Report abstention coverage and conditional risk; abstaining on hard examples must remain visible in throughput and operator cost.
- No synthetic sample may be counted as a physical trial.
- No simulated removal may be counted as physical trim extraction.
- No closed mass balance by arithmetic alone may be treated as chemical purity.
- No sorter prediction may be treated as an accepted downstream grade.
- If a vendor claim is not independently measured in the comparator run, label it “supplier-stated” and exclude it from the measured leaderboard.

## 8. Research objectives, not achieved results

After the first incumbent dataset and measurement repeatability study, set preregistered improvement thresholds. Initial stretch objectives to test—not current achievements—include:

- no increase in false acceptance of unknown/out-of-domain material;
- higher end-to-end correct-stream yield per incoming item after accounting for singulation, dropped/overlapped items and actuator misroutes;
- equal or higher downstream accepted kg per input tonne;
- lower residual trim and textile damage while preserving longer textile fractions or reusable components at matched accepted throughput;
- lower manual intervention burden without unsafe automation;
- schedule feasibility with zero hard machine, labour, precedence or utility violations;
- stable recovery of service after machine, operator, supply or utility disruptions.

A valid win must improve a meaningful end-to-end outcome under the same feedstock and acceptance rules, not merely top-1 classification accuracy or simulated cycle time.

## 9. Evidence and claim ceiling

The public Valvan description is evidence that Fibersort and Trimclean are marketed as a combined textile sorting/trim-extraction route. It is not proof of any site's achieved throughput, purity, cost, safety or recovery yield. This protocol is not a trial result. A comparative claim requires source-lot custody, exact configurations, calibrated meters, independent output inspection and receiving-process acceptance receipts.
