# Textile factory scheduling — industrial incumbent comparison v0.1

Status: benchmark protocol/template only. No Fibersort, Trimclean or Luminous production line has been measured by this repository.

Parent: [TEXTILE-MULTISTAGE-FACTORY-001 #4875](https://github.com/Luminous-Dynamics/symtropy/issues/4875).

## 1. Why the incumbent matters

Valvan publicly describes:
- **Fibersort:** NIR-based fibre/blend concentration prediction with RGB colour sorting for post-consumer textiles.
- **Trimclean:** a downstream trim-extraction system described as removing zippers, buttons and labels from fibre-sorted textiles.

Source: https://www.valvan.com/en/solutions/textile-sorting-recycling

This is a real industrial process comparator. Its public product description does not supply an independently reproduced, matched-lot performance dataset for every feedstock, damage mode, residual-trim level, operating cost or accepted downstream grade. Do not turn supplier descriptions into validated quantitative claims.

The research-review finding that attachment-removal AI is sparse concerns the published research corpus. It does not establish that commercial trim-removal systems do not exist.

## 2. Matched process arms

### Arm A — incumbent line

Observe the actual installed configuration and freeze its process map. A representative logical path to verify on-site is:

1. lot intake, weighing and custody assignment;
2. operator presentation / pre-sort, if present;
3. Fibersort NIR/RGB identification and material/colour routing;
4. Trimclean trim extraction;
5. post-trim inspection and reject/rework;
6. output-lot weighing, sampling and downstream recycler acceptance.

This is a proposed data-collection template, not an assertion that every site uses this exact route or order. Record bypasses, manual rework, recirculation, maintenance stops and rejects.

### Arm B — evidence-gated line candidate

Use the same incoming lot and product acceptance criteria. Candidate stages may include:

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
| Throughput | items/min, kg/h, total elapsed time, active time and downtime, with the measurement boundary |
| Routing | predicted route, actual route, reject, abstain, remeasurement and manual override |
| Composition | reference method, per-fibre error, blend error, unknown fraction, instrument and calibration |
| Sorting quality | output stream purity, contamination, false accept, false reject and unassigned mass |
| Trim removal | target trim mass, removed mass, residual trim, partial/failed extraction, component identity |
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
- residual trim by attachment family;
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
- equal or higher downstream accepted kg per input tonne;
- lower damage and residual-trim burden at matched accepted throughput;
- lower manual intervention burden without unsafe automation;
- schedule feasibility with zero hard machine, labour, precedence or utility violations;
- stable recovery of service after machine, operator, supply or utility disruptions.

A valid win must improve a meaningful end-to-end outcome under the same feedstock and acceptance rules, not merely top-1 classification accuracy or simulated cycle time.

## 9. Evidence and claim ceiling

The public Valvan description is evidence that Fibersort and Trimclean are marketed as a combined textile sorting/trim-extraction route. It is not proof of any site's achieved throughput, purity, cost, safety or recovery yield. This protocol is not a trial result. A comparative claim requires source-lot custody, exact configurations, calibrated meters, independent output inspection and receiving-process acceptance receipts.
