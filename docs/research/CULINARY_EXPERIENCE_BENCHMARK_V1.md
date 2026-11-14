# Culinary Experience Benchmark V1

**Status:** evaluation specification, not measured culinary performance.
**Workstream:** [CULINARY-EXPERIENCE-001 #2374](https://github.com/Luminous-Dynamics/symtropy/issues/2374)

## Thesis

The unit of success is not a recipe text, one recommendation, or a short schedule. It is a **completed meal and the human experience around it**: a feasible plan, actual cooking, safe handling, an acceptable result for the people eating it, sensible use of ingredients/resources, cleanup, and learning—while the cook remains author and decision-maker.

This is a systems benchmark around existing capabilities. It does not create another scheduler, recipe generator, food-composition database, food-safety authority, provenance system, or thermal simulator.

## Research-grounded problem statement

- The 2026 survey *Computational Recipe Intelligence* identifies fragmentation between recipe design, generation, recommendation, verification and outcome feedback. Its main implication for this work is to evaluate these as connected tasks rather than compare one isolated stage at a time: https://www.mdpi.com/2079-9292/15/15/3281
- Recipe2Plan (EMNLP Findings 2025) directly benchmarks multi-task cooking schedules with temporal constraints and shows why maximizing local parallelism can break a meal plan. Reuse its benchmark/protocol where comparable and evaluate the existing Symtropy planner; do not claim parity without aligned test instances and scores: https://aclanthology.org/2025.findings-emnlp.229/
- A 2026 many-objective recipe-design study optimized cost, time, sensory attributes and nutrition using sensory-trained surrogate models. Surrogate outputs are useful for search, but remain predictions, not proof of consumer liking: https://doi.org/10.1016/j.afres.2026.102145
- Food-waste evidence has a longitudinal gap. A review of household interventions recommends more durable measurement; a 2025 Australian randomized trial followed 671 households over seven months, and South African food-literacy research has tested interventions with 180 households in KwaDukuza. These motivate measured waste, local pilot context and persistence follow-up; none guarantees our intervention will work: https://www.mdpi.com/2071-1050/16/5/1963 ; https://doi.org/10.1016/j.resconrec.2025.108222 ; https://www.frontiersin.org/articles/10.3389/fsufs.2025.1556003/full
- Use ISO 6658:2017 for sensory-analysis design guidance and ISO 11136:2014 for consumer hedonic tests where relevant. Obtain and follow applicable full standard procedures before claiming a standards-conforming study: https://www.iso.org/standard/65519.html ; https://www.iso.org/standard/50125.html
- WHO's five keys—clean, separate raw/cooked, cook thoroughly, keep food at safe temperatures, and use safe water/raw materials—are safeguards, not objectives that can be traded against time or cost: https://www.who.int/activities/promoting-safe-food-handling/five-key-to-safer-food/
- For South African deployments, identify applicable requirements by setting; the Department of Health lists R638 (2018) for general hygiene of food premises, transport and related matters: https://www.health.gov.za/food-control-regulations/
- The October 2026 peer-reviewed study of 27 U.S. commercial kitchens measured NO₂, CO, benzene and temperature; it reported high NO₂ in 17 of 24 gas/propane kitchens and lower pollution/temperatures in the three all-electric kitchens studied. This limited sample motivates measured air/heat scenarios, not a universal conclusion or a South African exposure estimate: https://doi.org/10.1088/1748-9326/ae9d96 ; https://news.stanford.edu/stories/2026/10/pollution-levels-commercial-kitchens-gas-stoves

## Where this differs from current capability

| Existing work | Keep using it for | This benchmark adds |
|---|---|---|
| Symtropy KITCHEN-OPS-001 #2227 / PR #2230 | Resource-constrained schedule feasibility and makespan | Full journey completion and actual-vs-planned outcomes |
| Symthaea CULINARY-TRANSFER-002 #8170 | Frozen baselines, domain shift and real-world transfer | End-to-end outcome vector and meal/user feedback links |
| Symthaea CULINARY-BENCH-001 #7859 | Benchmark integrity and reproducibility | Cross-stage scenario coverage and acceptance gates |
| Symthaea CULINARY-EVIDENCE-001 #7952 | Sourced ingredient/food facts | Evidence attached to each consequential substitution/claim |
| Symthaea HUMAN-CRAFT-001 #8183 | Agency, authorship, dignity and skill retention | Human-craft metrics as first-class outcomes in culinary tests |
| Symtropy KITCHEN-STATION-001 #2324 | Sink, washing, splash and drying results | Reuse stage metrics for total-meal water/cleanup, without double-counting |
| Mycelix governance/provenance | Permissions, attribution and community-defined reuse rules | Required source/license/attribution declarations; no blanket permission from public availability |

## Paradigm shifts

1. **Recipe → meal system.** Plan multiple dishes, shared equipment, preparation windows, serving synchrony, safe holding, leftovers and cleanup.
2. **One “best recipe” → Pareto choice set.** Show time, cost, accessibility, measured waste, water/energy, nutritional-data coverage, preference, safety and craft as separate dimensions. No opaque all-in-one score.
3. **Predicted flavor → evidenced sensory results.** Mark estimates distinctly; validate claims with appropriate human tests and representative participant groups.
4. **One-shot output → closed-loop adaptation.** Capture actual timings, substitutions, yield, rework, discard and user edits with consent; update future plans only within declared permission.
5. **Personalization → user-directed control.** Let people set goals, exclusions, cuisine identity, effort, pace and automation level; avoid unsupported sensitive health inferences.
6. **Uniform culinary canon → plural, attributed culinary knowledge.** Preserve provenance, source rights, regional context, community protocols and cook attribution during adaptation.
7. **Automation → mastery support.** Explain why a timing/substitution is suggested, preserve manual execution, let cooks override, and measure independent skill after assistance is withdrawn.
8. **Sustainability slogans → measured boundaries.** Measure purchased, edible, discarded, water and energy quantities within a declared boundary. Do not claim carbon benefits from incomplete factors.
9. **Kitchen productivity → healthy working conditions.** Evaluate air quality, heat and ventilation at occupied zones during service and idle periods. Do not trade worker exposure against energy, throughput or cost; thresholds require local occupational-health review.

## Benchmark protocol

### Baselines

Use the same permitted information, candidate set and frozen scenario manifests for at least:
1. a fixed known-good recipe library with deterministic constraint filtering;
2. a transparent feasibility-first filter with simple tie-breaking;
3. a transparent weighted multi-objective baseline, publishing its weights and sensitivity;
4. the existing deterministic Symtropy kitchen planner paired with the same recipe/plan input;
5. the evaluated culinary policy, including abstentions, explanations and edits.

Add other models only when their inputs, versions, data cutoff and ability to act are documented. Never treat an abstention as a hidden failure or a fabricated recipe as a valid output.

### Splits and transfer

Split by whole recipe/task lineage, cuisine/region, ingredient family, equipment profile, season/availability, and household/commercial setting where applicable. Random split alone cannot support a domain-generalization claim. Combined multi-axis shifts form a harder tier and must be labeled separately.

### Outcome vector

- **Hard feasibility/safety:** constraint-verifier pass, safety/allergen/handling gate status, timing/resource conflicts, explicit abstentions and unknowns.
- **Journey completion:** plan execution success, actual vs predicted duration, missed deadlines, rework, serving synchronization and recovery.
- **Resources:** cost per completed serving, edible yield, edible discard mass, water and energy per completed serving, with full boundaries and denominators.
- **Food and preference:** independently sourced nutrient estimates/coverage, sensory acceptability, preference match, portion/yield accuracy and substitutions accepted.
- **Human outcomes:** cook agency, ability to edit/decline, task workload, enjoyment, learning and independent skill retention.
- **Culture and rights:** provenance coverage, attribution, permissions, participant-defined cultural fit and rate of prohibited/unattributed reuse.
- **Accessibility/robustness:** successful execution by relevant ability/experience groups; missing equipment, water/power interruption and unavailable-ingredient recovery.
- **Work environment (commercial track):** pollutant time series by occupied zone, thermal exposure, ventilation state, sensor coverage and staff-relevant outcomes; preserve high-exposure windows and use locally reviewed thresholds.
- **Evidence quality:** exact source/manifest hashes, data classes, missingness, uncertainty, denominators and replay determinism.

Do not hide subgroup failures in averages. No safety, allergy or cultural-rights gate is exchangeable for a faster schedule.

## Proposed stretch targets (hypotheses, not results)

- 100% of schedules presented as feasible pass an independent verifier; zero silent hard-constraint violations.
- Zero "safe" recommendations when required allergen/food-handling evidence is unknown.
- At least 10% lower makespan than serial execution on frozen multitask cases, with no increase in temporal/resource-constraint violations.
- At least 90% end-to-end executable-plan success in physical pilot cases with the declared ingredients/equipment; report per-scenario and subgroup results.
- At least 15% lower directly weighed avoidable edible waste versus a suitable control over a 6–8 week pilot, with a later persistence follow-up; this is not claimable from a short demo.
- No meaningful reduction in blind/controlled sensory acceptance against a declared baseline, using a justified design and uncertainty intervals.
- 100% of consequential substitutions preserve required allergen, unit, quantity, source and critical process constraints or are rejected.
- No meaningful decrease in cook agency, voluntary acceptance or independent skill after assistance is removed; agree non-inferiority margins with participating cooks before trials.
- Every headline result includes paired uncertainty, denominators, trade-offs and any missing/abstained cases. No universal SOTA claim from synthetic tests.

### How to establish "better than SOTA"

First reproduce a suitable public baseline (including Recipe2Plan for the scheduling slice) on aligned cases. Then preregister a separate held-out evaluation. Beat a published baseline only on the exact shared metric/task, with equal inputs and a compatible protocol. Use physical/participant data for claims about successful cooking, liking, waste reduction or skill. Use confidence/uncertainty intervals and practical-effect thresholds; do not use a larger model or aggregate score as substitute evidence.

## Scenario contract

The authoritative machine-readable scenario set is `CULINARY_EXPERIENCE_SCENARIOS_V1.json` (13 scenarios, including a commercial air/heat exposure scenario); the metric catalogue is `CULINARY_EXPERIENCE_METRICS_V1.json` (28 metrics).

Every scenario declares its required metrics, non-compensatory gates, evidence requirements, adversarial controls and claim limit. A blocked/unknown result must be explicit. A synthetic record can test the software contract but never qualify a product, recipe, safety or taste claim.

## Execution sequence

1. Freeze recipe/plan manifests, baselines, task boundaries and outcome definitions.
2. Write cross-artifact compatibility/split-integrity checks; reuse the existing schedule verifier and culinary evidence contracts.
3. Run synthetic positive and negative controls; describe only contract correctness.
4. Conduct recipe-and-equipment execution pilots with a small representative range of cooks and declared conditions.
5. Run controlled sensory tests and direct waste/resource measurements with consent, study design and uncertainty plan.
6. Run a longer pilot to test persistence, access inequity, cultural appropriateness and whether skill is retained after assistance is withdrawn.
7. Publish per-scenario results, non-wins and data exclusions; update only when a new manifest/version is recorded.

## Safety, consent and boundaries

This programme is not a food-safety authority, medical or dietetics service, allergen guarantee, certification process or replacement for local food handling requirements. Food-composition sources must be selected for the relevant foods/region; USDA FoodData Central is a useful U.S. data resource, not universal authority for South African foods: https://fdc.nal.usda.gov/api-guide/. User-study participation, personal data, practitioner work traces and culturally governed knowledge require purpose limitation, consent and access/retention rules.

**Current status:** benchmark artefacts are proposals until a versioned implementation, reproducible software tests, and empirical cooking/user trials provide the relevant evidence.
