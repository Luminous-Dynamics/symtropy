---
title: Space Economy Reference Benchmark Suite
version: 0.1
status: proposed-reference-campaign
date: 2026-10-16
scope: cross-stack, deterministic economic and infrastructure decision benchmarks
evidence_class: synthetic-fixtures-until-replaced-or-augmented-by-qualified-data
---

# Space Economy Reference Benchmark Suite v0.1

## Purpose

This suite turns the Symtropy space-economy architecture into falsifiable engineering work. It is a **benchmark specification and synthetic fixture corpus**, not evidence that the engine, a lunar process, a market, or a flight system has passed. Fixture values use synthetic cost units (SCU); they are not price estimates or forecasts.

The scope complements, rather than replaces, the economic ledger, interplanetary trade, deep-space logistics, space-bootstrap, and industrial-closure specifications. The benchmark asks whether those systems can jointly make decisions that are physically consistent, economically transparent, resilient to disruption, and honest about evidence.

## Paradigm shift

**Optimize delivered, qualified capability over its lifecycle—not launches, payload mass, production volume, or token throughput in isolation.**

The economic object is a *service delivered under constraints*. A communications service, delivered kilogram, repairable asset-year, watt-hour at a lunar worksite, or kilogram of qualified product has a cost, reliability, location, delivery window, provenance, uncertainty, and end-of-life consequence.

This implies six architectural shifts:

1. **Mission → service network.** Compare interoperable services and infrastructure portfolios, not only individually optimized spacecraft or missions.
2. **Import-versus-make → lifecycle trade space.** Local production can reduce Earth-launched mass while increasing cost, energy demand, maintenance, or risk. Never declare ISRU economical from mass closure alone.
3. **Cheapest plan → feasible Pareto frontier.** Apply physical, contractual, safety, and sustainability gates first; only then compare cost, mass, availability, recovery, and resilience. Preserve alternatives instead of hiding value judgments in one arbitrary weighted score.
4. **Perfect information → delay-aware decisions.** Prices, inventories, offers, and operating states are local observations with source and age. A financial claim cannot create physical inventory; stale or expired information cannot silently become current truth.
5. **One expected case → robust operation.** Evaluate outages, launch slips, degraded equipment, energy scarcity, corrupted evidence, and tail costs. A cheap nominal plan that fails a hard service floor is infeasible.
6. **Simulation output → evidence-bearing claim.** Every recommendation must bind exact scenario input, assumptions, units, evidence class, model version, source commit, and verifier output. Estimated, simulated, observed, measured, and independently qualified values must remain distinguishable.

## System responsibilities

- **Sol-Atlas:** spatial/time context, asset and infrastructure topology, terrain/illumination/coverage layers, and links to their source datasets. It must expose the frame, epoch, resolution, validity region, and provenance of spatial inputs.
- **Symtropy:** deterministic scenario execution, causal inventories and energy ledgers, resource/process accounting, logistics scheduling, constrained optimization, and Pareto analysis.
- **Symthaea:** bounded hypothesis generation and plan comparison. It may propose actions or explain trade-offs; it does not supply missing evidence, turn unknowns into zero, or grant execution authority.
- **Mycelix:** provenance, versioned claims, permissioned sharing, custody/contract attestations, and append-only correction/retraction. A record attests that a claim was made with specified evidence; it does not make the claim physically true.
- **Independent verifier:** checks fixture hashes, arithmetic invariants, negative controls, candidate output shape, and exact source/tested revision. No component may certify its own result solely from its own narrative.

No live spacecraft command path is in scope for this benchmark.

## Benchmark protocol

### Input contract

Every run binds:

- `suite_id`, `suite_version`, and scenario ID;
- exact scenario-manifest digest and fixture class;
- units and any conversion profile;
- candidate implementation, exact source commit, dependency/toolchain profile, and configuration;
- input evidence classes and validity bounds;
- baseline implementation/version;
- deterministic seed, where randomness is used.

Missing or incompatible required fields produce an explicit `indeterminate` or `rejected` result, not a guessed default. The same input and exact implementation must replay to the same decision and same normalized result. Floating-point tolerances, if later needed, must be explicit per metric and must never be used to relax conservation or hard policy gates.

### Result contract

Each candidate returns:

- feasibility status and each gate's result;
- the chosen option **and** the remaining Pareto-nondominated options;
- normalized lifecycle cost and cost per qualified service unit;
- Earth-launched mass and local energy/material use where applicable;
- service coverage/availability and outage/recovery outcomes;
- resource, environmental, and evidence uncertainties;
- rejected alternatives with machine-readable reasons;
- the input digest, source revision, assumptions, and verifier receipt.

A lone scalar score is insufficient. Reports must show the metric vector and disclose any policy weights used for a final choice.

### Required order of operations

1. Parse and type-check units, frame/time profiles, evidence classes, and applicability.
2. Verify physical, custody, authority, and hard policy invariants.
3. Mark infeasible options and emit precise rejection reasons.
4. Evaluate lifecycle economics and service outcomes for remaining options.
5. Produce Pareto alternatives and any explicitly policy-conditioned recommendation.
6. Replay and independently verify the decision receipt.

## v0.1 scenario families

The frozen JSON corpus contains six small synthetic cases:

| ID | Purpose | Required result |
|---|---|---|
| `LIFECYCLE_MASS_COST_TRADEOFF` | Prevent launch-mass reduction from being mistaken for economic superiority | Report the mass/cost trade-off; no universal winner |
| `CISLUNAR_RELAY_N_MINUS_ONE` | Evaluate a critical service under a single-provider outage | Reject a plan that misses demand; a feasible dual-provider option survives one provider loss |
| `DELAYED_MARKET_AND_DOUBLE_SALE` | Exercise stale offers, acceptance after expiry, duplicate sale, and replay | Reject stale/expired and conflicting transactions; idempotent replay cannot duplicate custody or settlement |
| `PROCESS_MASS_BALANCE_AND_DUPLICATE_EVENT` | Check process input/output/waste balance and event idempotency | Preserve mass; reject the over-mass process and duplicate import |
| `DEBRIS_GATE_BEFORE_COST` | Prevent an economically cheap but policy-infeasible Earth-orbit option from winning | Reject the non-compliant option before economic ranking |
| `ROBUST_COST_AND_SERVICE_FLOOR` | Compare nominally cheap fragile supply with a robust provider | Enforce service floor in every required stress case before comparing cost |

These are **contract tests**, not calibrated engineering models. The corpus intentionally uses small, transparent numbers so that expected properties can be checked independently by inspection.

## Proposed benchmark targets

Targets below are gates to pursue, not measured results.

### Tier 0 — integrity (release blockers)

- 100% of frozen negative controls are rejected for the expected reason.
- Zero mass creation/loss outside an explicitly modeled source, sink, or uncertainty interval.
- Zero duplicate custody transfer, double sale, or duplicate settlement under replay.
- Zero hard-invariant violations accepted as a feasible plan.
- 100% of results bind an exact input digest, source revision, scenario ID, and verifier result.
- Exact deterministic replay for the deterministic corpus.
- No claimed pass inferred from a queued, cancelled, stale-head, skipped, or unrelated workflow.

### Tier 1 — decision quality

- For each small scenario with an enumerable solution, match the independent oracle on feasibility and expected ordering.
- Correctly preserve Pareto-incomparable choices; do not force a single answer when objectives genuinely trade off.
- Reduce lifecycle cost per qualified service unit by a target of **10–20%** against a documented, strong baseline in at least three representative scenario families, without breaking any hard constraints. This is a research target; accept it only when reproduced under equal assumptions.
- Report median, p95, worst-case, and sensitivity results; do not publish only the best seed or mean cost.

### Tier 2 — resilience and uncertainty

- Detect 100% of the frozen single-fault cases and reject invalid recovery plans.
- Demonstrate service demand coverage after the required single-provider failure where the scenario claims N-1 capability.
- Keep uncertainty and evidence class attached through transformations; never promote simulated or estimated resources into accepted inventory without an explicit acceptance event.
- On a future measured corpus, show calibration and tail-risk performance against a baseline before using probabilistic claims operationally.

### Tier 3 — scale and external validity

- Measure runtime and memory against increasing asset, provider, route, event, and uncertainty-set counts.
- Use independently sourced ephemerides, environmental measurements, costs, process yields, and availability data when available; record data license, publication/retrieval date, resolution, and limitations.
- Compare with independent tools or a separately implemented oracle. A model and checker sharing the same bug do not constitute independent verification.
- Run shadow-mode replay with domain review before any operational use; flight qualification remains a separate program.

## Metamorphic and adversarial tests

The implementation should grow beyond fixed expected answers. At minimum:

- Raising an option's cost while all else remains equal must not improve its cost ranking.
- Removing a required provider or lowering usable capacity cannot make a previously infeasible demand feasible.
- Duplicating an idempotent event cannot change inventory, custody, or settlement.
- Reordering independent events must preserve results; reordering causally dependent events must either preserve defined causal semantics or produce an explicit conflict.
- Increasing a local-production energy price must not make that option look cheaper.
- Introducing an expired offer or unqualified resource must not expand the feasible action set.
- Tightening a hard disposal/safety constraint cannot enlarge the feasible set.
- Replacing measured evidence with a lower evidence class must not silently increase decision authority.
- A model version, ephemeris, frame transformation, process recipe, or cost assumption change must change the input/derivation identity where it affects results.

## Data and standards boundary

This suite is informed by:

- [NASA's LunaNet Interoperability Specification](https://www.nasa.gov/directorates/somd/space-communications-navigation-program/lunanet-interoperability-specification/), Version 5, published 7 February 2025. It defines a framework for interoperable lunar networking, communications, navigation/PNT, and information services.
- [CCSDS](https://ccsds.org/), which develops space data-system standards for interoperability and cross-support across participating agencies.
- [ISO 24113:2023](https://www.iso.org/standard/83494.html), which defines debris-mitigation requirements for objects launched into or passing through near-Earth space. Applicability must be evaluated for the particular mission; this is not a blanket lunar-surface or off-Earth factory standard.
- [ESA's debris-mitigation verification guidance](https://technology.esa.int/page/space-debris-mitigation), including its 2025 revision of requirements/guidance.
- [NASA's 29 June 2026 lunar enabling-infrastructure announcement](https://www.nasa.gov/directorates/stmd/nasa-seeks-industry-input-to-accelerate-lunar-surface-technologies/), which identifies surface power, ISRU, advanced manufacturing, and nanomaterials among the integrated capabilities needing development.
- [Gao et al., *Lunar surface energy infrastructure* (2 September 2026)](https://www.nature.com/articles/s44453-026-00053-w), a review of lunar power architectures and first-order integrated cost-benefit considerations.

These references motivate interoperability, sustainability, and integrated-resource scenarios. They do not provide validated cost inputs for the synthetic fixtures, and citing a standard does not establish compliance.

## Implementation sequence

1. Keep the v0.1 manifest as a frozen, digest-pinned research artifact.
2. Add a standalone validator for its schema and invariants before connecting it to runtime code.
3. Implement the simple deterministic cases in a small Rust reference harness with no live-service dependencies.
4. Add fault-injection controls and an independent verifier with machine-readable receipts.
5. Connect the benchmark adapters to Symtropy, Symthaea, Sol-Atlas, and Mycelix only through explicit typed contracts.
6. Add externally sourced datasets and realistic uncertainty only when their provenance and interpretation can be independently reviewed.

The first milestone is successful rejection of incorrect claims, not a grand-economy visualization. The suite succeeds when it can tell us that a plan is infeasible, a proposed saving is illusory, or the available evidence is insufficient—and can show another reviewer exactly why.

 
## Active reference extension — correlated failure domains

The standalone reference suite now has a follow-up deterministic fault-set comparison for cislunar relay topologies: [BENCH-SPACE-003](https://github.com/Luminous-Dynamics/symtropy/issues/1749). Two providers can pass provider-level N-1 checks while sharing an upstream power/controller dependency whose failure disables both.

The executable example `tools/space-economy-bench/examples/common_cause_reference.rs` makes this distinction explicit. It compares a 65-SCU synthetic shared-bus option with an 85-SCU synthetic separate-controller option under demand 60 and two 70-capacity relays. The first topology passes either provider outage but fails its shared-bus outage; the second survives each individual provider/controller fault represented in its own topology.

This is a separate extension and does **not** alter or silently add a seventh scenario to the frozen v0.1 corpus. The example verifies named, deterministic component-dependency cases. It does not assign failure probabilities, claim statistical independence, or qualify actual cislunar infrastructure. A future probability model requires FMEA/hazard evidence and calibration.

 
## Active reference extension — supplier concentration

The standalone example `tools/space-economy-bench/examples/supplier_concentration_reference.rs` and [BENCH-SPACE-004](https://github.com/Luminous-Dynamics/symtropy/issues/1750) add a deterministic supplier-portfolio comparison inspired by the OECD's 2026 findings on launch-supply concentration and critical-input dependencies. The fixture is synthetic and is not an estimate of real supplier market shares.

For capacity shares 50/20/15/15, HHI is 3,350 and the loss of the 50-unit supplier leaves 50 units against demand 70. For shares 25/25/25/25, HHI is 2,500 and any single provider loss leaves 75 units. The second portfolio costs 18% more in this synthetic fixture, but it is the only one that passes the mandatory N-1 service gate.

This supplements, rather than changes, the frozen six-scenario v0.1 corpus. HHI is reported as a descriptive concentration metric for the stated portfolio—not proof of market power or an outage probability. Deterministic fault coverage is not a probability model.

 
## Active reference extension — orbital sustainability as common-pool infrastructure

The standalone example `tools/space-economy-bench/examples/orbital_externality_reference.rs` adds an explicit, deterministic test of whether lifecycle decisions externalize orbital burden. It is a separate extension; the original v0.1 six-scenario JSON corpus remains frozen.

### Motivation from current evidence

ESA's [Space Environment Report 2026](https://www.esa.int/Space_Safety/Space_Debris/ESA_Space_Environment_Report_2026), published 14 September 2026 and based on data through the end of 2025, reports a sharp deterioration in its long-term Space Environment Health Index and concludes that end-of-life disposal and prevention need to be complemented by active debris removal. Its health index is an ESA-defined model, not a unit used by this benchmark. OECD's [Space Economy at a Glance 2026, chapter 4](https://www.oecd.org/en/publications/the-space-economy-at-a-glance-2026_cbf9b240-en/full-report/managing-vulnerabilities-dependencies-and-sustainability-risks-of-the-space-economy_4d97ef6d.html) separately identifies orbital sustainability, launch concentration, supply-chain dependencies, and wider externalities as systemic risks.

That motivates a paradigm shift from treating orbital occupancy and end-of-life consequences as free side effects to treating them as constraints on shared infrastructure. It does **not** justify treating debris as one universal scalar or assuming the same model applies to low Earth orbit, lunar orbit, and the lunar surface.

### Executable synthetic contract

The example uses five declared options, a 90-unit qualified-service floor and a synthetic residual-burden budget of 15 units:

| Option | Accounted lifecycle cost | Residual synthetic burden | Contract outcome |
|---|---:|---:|---|
| Cheap disposable | 60 SCU | 32 SOBU | Rejected: burden budget exceeded |
| Design for removal | 82 SCU | 13 SOBU | Feasible |
| Service and reuse | 88 SCU | 8 SOBU | Feasible |
| Unverified cleanup claim | 63 SCU | 32 SOBU | Rejected: claimed removal receives zero credit |
| Under service floor | 60 SCU | 8 SOBU | Rejected: qualified-service floor missed |

The executable oracle must preserve both `design_for_removal` and `service_and_reuse` on the feasible Pareto frontier: one is cheaper; the other leaves less residual burden. It must not select the cheap disposable option just because that option has the lowest accounting cost. An end-of-life provision is included in lifecycle cost rather than left out of the accounting boundary.

Prevention credits and active-removal credits are represented separately. A claimed but unverified prevention or removal amount contributes zero to the accepted credit. Verified credits cannot exceed the remaining synthetic burden. The receipt includes a per-option gate result, rejection reason codes, a source commit, source digest, and a strict claim ceiling.

### Required future benchmarks

- **Tier 0 integrity:** 100% rejection of the frozen unverified-removal and burden-budget negative controls; 0 unverified burden credits; 0 accepted options below the qualified-service floor; cost includes the explicitly declared end-of-life provision; exact input/source provenance in the receipt.
- **Tier 1 decision quality:** preserve all feasible cost/burden/service Pareto alternatives; reject sustainability-gate violations before cost ranking; demonstrate cost and burden sensitivity with independently recomputable arithmetic rather than an opaque weighted scalar.
- **Tier 2 external validity:** replace SOBU fixture inputs only with regime-specific, independently sourced quantities and a reviewed conversion model. Publish the orbit regime, epoch, time horizon, uncertainty, collision/debris model, data provenance/licence, and calibration corpus. Compare predictions with held-out observations or an independently maintained model.
- **Tier 3 operations:** test end-of-life disposal success, passivation, servicing/removal interface availability, conjunction-avoidance burden, and verified removal outcomes as distinct measures. Report these separately; do not collapse them into one number without an explicit model and sensitivity study.

### Limitations

SOBU means *synthetic orbital burden unit* only. It is not a physical quantity, collision probability, validated debris-risk proxy, ESA Space Environment Health Index, ISO threshold, or proof of compliance. The example is not an orbital-dynamics simulation. It must not be used to assert real Earth-orbit economics, nor transferred to cislunar orbit or surface operations without a separate system boundary and validated physical basis. The benchmark verifies a decision contract; it does not certify a mission or policy.

Follow-on calibration and data research is tracked in [BENCH-SPACE-005](https://github.com/Luminous-Dynamics/symtropy/issues/1759).

 
## Active reference extension — verified industrial capability closure

The standalone example `tools/space-economy-bench/examples/industrial_closure_reference.rs` tests a further paradigm shift: evaluate the closure of a *capability dependency graph*, not the headline fraction of local material or a label saying "ISRU available." This is a separate synthetic extension and does not change the frozen six-scenario v0.1 JSON corpus.

The direction is consistent with NASA's 2026 Lunar Enabling Infrastructure Accelerator, whose stated gaps include connected surface power, ISRU, advanced manufacturing and nanomaterials systems ([NASA announcement](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/); [infrastructure context](https://www.nasa.gov/directorates/stmd/nasa-seeks-industry-input-to-accelerate-lunar-surface-technologies/)). It is also consistent with ESA's longer-term circular-space-economy framing around servicing, maintenance, repair, reuse and recycling ([ESA Space Environment Report 2026](https://www.esa.int/Space_Safety/Space_Debris/ESA_Space_Environment_Report_2026)). These sources motivate connected capability modeling; they do not validate the fixture graph or its costs.

### Distinct metrics

The receipt separately exposes:
- Claimed local-content percentage.
- Reported current operating status.
- Whether the operational claim has verified evidence.
- Whether the target is locally closed under the declared graph.
- Explicit blocker IDs and reason codes.

A target is locally closed in this synthetic contract only if it is operational, evidence-verified, locally operated, has no declared recurring import dependencies, and every dependency recursively meets the same requirements. Missing dependencies and dependency cycles fail closed. Cycle detection is conservative; this graph walk reports blockers and does not claim to compute globally minimal cut sets.

### Synthetic oracle

| Plan | Claimed local content | Lifecycle cost | Target operating now | Target locally closed |
|---|---:|---:|---|---|
| High local content | 92% | 95 SCU | Yes | **No** — unverified metal refining and imported recurring sensor module |
| Closure first | 78% | 120 SCU | Yes | Yes under the declared synthetic graph |

The point is not that local electronics manufacturing is an established or near-term lunar capability. That node is a synthetic fixture. Rather, the benchmark must make a high-local-content claim insufficient to override one unverified production dependency or an imported recurring bottleneck. The closure-first plan has a higher synthetic lifecycle cost, but passes the specified dependency-closure gate.

### Proposed targets

- 100% rejection of missing dependency, unverified process, recurring-import, nonlocal-operation, and dependency-cycle negative controls for the expected reason.
- Zero locally-closed target claims if any mandatory dependency is unverified, nonlocal, or dependent on recurring external supply.
- Operational-now and locally-closed results remain separate fields; the former cannot imply the latter.
- Every graph input and source revision has a deterministic digest/receipt; unknown capability claims remain blockers.
- For any empirical version, publish the service horizon, stockpile policy, wear-out/replacement rates, energy and material budgets, process yields, evidence classes, and dependency provenance. Evaluate the same service target under stress-tested alternatives rather than comparing arbitrary local-content percentages.

Research tracking: [BENCH-SPACE-006 — verified industrial capability closure and bootstrap bottlenecks](https://github.com/Luminous-Dynamics/symtropy/issues/1765).

 
## Active reference extension — time-horizon service viability under resupply shocks

The example `tools/space-economy-bench/examples/temporal_resilience_reference.rs` extends static capability closure into a discrete 12-month operating horizon. It tests whether declared service demand remains available through recurring component failures and resupply delays. This is a separate synthetic extension; the frozen v0.1 six-scenario JSON corpus remains unchanged.

NASA's [Lunar Surface Technology overview](https://www.nasa.gov/lunar-surface-technology/) describes interdependent needs including reliable power, local resource extraction, landing-site construction, dust mitigation and autonomous robotics. ESA's [Space Environment Report 2026](https://www.esa.int/Space_Safety/Space_Debris/ESA_Space_Environment_Report_2026) frames servicing, repair, reuse and recycling as part of a circular space economy. These motivate service-horizon and repairability benchmarks but do not calibrate the synthetic failure events or repair durations.

### Synthetic reference results

| Case | Service months / 12 | Availability | Maximum consecutive outage | Outcome at 75% floor |
|---|---:|---:|---:|---|
| Imported spares, nominal arrivals | 9/12 | 75.00% | 1 month | Pass |
| Imported spares, resupply delayed | 7/12 | 58.33% | 2 months | Fail |
| Verified local repair, nominal | 9/12 | 75.00% | 1 month | Pass |
| Verified local repair, resupply shock | 9/12 | 75.00% | 1 month | Pass |
| Unverified local-repair claim | 1/12 | 8.33% | 11 months | Fail |

The fixture uses a 12-month horizon, failure months 2, 6 and 10, and a synthetic 75% qualified-service availability floor. For import-based supply, the nominal arrival months are 4, 8 and 11; the resupply-delay scenario arrives at months 7 and 11. A repair or replacement occurring during a period does not restore service until the following period. The receipt reports service/outage months, the longest outage, repair credits, externally sourced spares consumed, deliveries and residual stock.

A local-repair assertion only counts if local repair evidence is verified. The architecture is eligible for cost ranking only if every scenario declared for that architecture passes the service floor. The imported-spares architecture fails its delay shock. The verified local-repair architecture passes both declared contexts and is therefore selected despite its higher synthetic lifecycle cost.

### Proposed targets

- Exact matching against an independently recomputable month-by-month oracle.
- A resupply-delay transformation must never improve availability compared with the corresponding nominal schedule.
- An unverified repair capability contributes zero credited repair events.
- 100% rejection of invalid event months, duplicate event definitions, and source-commit/digest violations.
- Robust-feasible architecture selection requires passing every declared mandatory stress scenario before cost comparison.
- Publish total service loss, worst consecutive outage, repair turnaround, inventory drawdown, external resupply dependence and remaining inventory; do not report only average availability.

### Limitations and calibration plan

Failure dates and repair delays are deterministic synthetic events, not measured failure rates. The reference does not optimize inventory, model simultaneous failures, or prove a locally repairable system can be built. Before empirical use, specify the service boundary and required service level, stockpile policy, wear-out/replacement process, lead-time distributions, repair capacity, energy/material constraints, common-cause failures, data provenance, uncertainty and held-out validation. Keep empirical and synthetic evidence classes distinct.

Follow-up: [BENCH-SPACE-007 — time-to-failure, resupply delays and service viability runway](https://github.com/Luminous-Dynamics/symtropy/issues/1792).

 
## Active reference extension — resource evidence to production chains

The example `tools/space-economy-bench/examples/resource_chain_reference.rs` adds a small mass-balanced pathway network. It is a separate extension; the original v0.1 six-scenario corpus is unchanged. Its purpose is to prevent a material catalogue from being mistaken for a productive economy.

### Research motivation

NASA's [ISRU overview](https://www.nasa.gov/overview-in-situ-resource-utilization/) notes that volatile location, form, concentration, distribution and accessibility remain incompletely characterized and that many demonstrations to date involved analog materials under Earth conditions for limited durations. USGS's [Assessment of Lunar Resource Exploration in 2022](https://www.usgs.gov/publications/assessment-lunar-resource-exploration-2022) provides a resource-assessment framework that treats resource type, certainty and recoverability distinctly. NASA's [Lunar Enabling Infrastructure Accelerator](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/) now explicitly connects power generation, oxygen extraction from regolith, advanced manufacturing and nanomaterials. These priorities reinforce a chain-level model; they do not validate the numerical fixture used here.

### Candidate production-chain families

1. **Bulk regolith to shielding/berm aggregate:** excavate, screen and move a large mass stream. Begin with products whose value does not require high-purity feedstock—such as bulk shielding or terrain works—while keeping structural qualification as a distinct gate.
2. **Regolith to oxygen:** extract oxygen chemically bound in mineral phases; track energy, residue and process lifetime. Treat Fe/Al/Ti/Si-bearing residue as potential feedstock, not refined metal inventory, until separation yield and product quality are verified.
3. **Polar volatile-bearing feed to water:** prospect the site, excavate/heat, collect vapour, purify and condition water. The concentration and accessibility assumptions are site-specific and must remain unknown until measured.
4. **Polar water to hydrogen and oxygen:** allocate recovered water to electrolysis and account for hydrogen/oxygen storage, conditioning, losses and delivery. Water delivered for life support and water consumed as electrolysis feed are alternative allocations; do not count the same recovered water twice.
5. **Recycling loop:** recover water, selected metals, polymers, and repairable modules. Model filter/catalyst/seal replacement, contamination, degradation, energy and recovery efficiency; a recycled stream is not automatically equivalent to virgin-grade product.

### Synthetic fixture oracle

| Route | Feed batch | Conditional product streams | Residuals | Evidence-gated output |
|---|---:|---|---:|---|
| Bulk shielding aggregate | 1,000 kg regolith | 900 kg aggregate | 100 kg screening rejects | 900 kg within the explicitly passing synthetic gate |
| Regolith oxygen | 1,000 kg regolith | 200 kg oxygen | 800 kg residue | 0 kg: process/product gates fail and assumed energy demand (6,000 kWh) exceeds the synthetic 5,000 kWh budget |
| Polar feed to water | 100 kg volatile-bearing material | 54 kg purified water | 46 kg residuals | 0 kg: site-feed, process and product-quality gates are unverified |
| Same polar batch to electrolysis | Same 100 kg batch | 48 kg oxygen + 6 kg hydrogen | 46 kg residuals | 0 kg for same evidence gates |

Every alternative closes mass balance exactly. The polar-water-to-water and polar-water-to-electrolysis pathways share an allocation-group ID and cannot be summed. The 54 kg purified water is an intermediate consumed by the electrolysis route, not an additional output alongside the oxygen and hydrogen. The water electrolysis split uses stoichiometric mass balance only; energy, purity, storage loss and attainable production rates remain uncalibrated.

The regolith oxygen fixture assumes 400 kg oxygen inventory in a 1,000 kg batch and 50% recovery; all batch sizes, fractions, yields, energy demands and evidence-gate flags in this reference are synthetic. They are not lunar grade estimates, reserves, production forecasts, measured energy intensities or technology qualification. The positive bulk-aggregate scenario is only a gate-logic test and does not assert that an actual lunar site has been sampled or a process has been space-qualified.

### Proposed targets

- Zero unexplained mass-balance delta on every pathway.
- 100% rejection of resource supply when a required site/access, process qualification, declared-use product quality or power gate fails.
- Zero double counting of intermediate material or alternative routes consuming the same batch.
- Zero refined-metal credit from an unqualified mixed residue.
- Distinct fields for resource occurrence, measured site feed, recoverable amount, process output, qualified product and delivered supply.
- Each empirical input includes source, site/body, time/epoch, spatial resolution, data license, uncertainty, sampling/measurement method and model version.
- Reports include yield intervals/sensitivity, kWh/kg qualified product, tailings/residuals, consumables and wear-part needs, throughput, repair downtime, purity, and cost per delivered qualified kilogram.

Follow-on: [BENCH-SPACE-008 — resource evidence to auditable production chains and coproducts](https://github.com/Luminous-Dynamics/symtropy/issues/1803). See the broader [Lunar Resource-to-Production Chain Map v0.1](SPACE_RESOURCE_PRODUCTION_CHAIN_MAP_V0_1.md) for resource evidence tiers, proposed first industries, process chains, and staged benchmark gates.

 
## Active reference extension — resource-survey selection by decision value

The example `tools/space-economy-bench/examples/resource_assay_selection_reference.rs` closes a loop between site uncertainty and production-route decisions. It is separate from the frozen six-scenario v0.1 corpus. Rather than reward more observations in the abstract, it compares the best action without a survey against the action policy after each possible survey result, then subtracts the survey cost.

### Synthetic exact oracle

Prior probability of a rich site is 50%. Building local oxygen-production capability yields +200 decision units at a rich site and -100 at a poor site; importing oxygen is a zero-payoff fallback.

- Best value without survey: 50 synthetic units.
- Expected value of perfect information (EVPI): 50 units.
- Verified 90% sensitivity/specificity direct assay costing 10 units: EVSI 35, net incremental value +25. Build after a positive result; import after a negative result.
- Verified 60% sensitivity/specificity lower-resolution assay costing 2 units: EVSI zero because the optimal action remains the same after either result; don't buy it. The receipt shows the posterior rich-site probabilities as 60% after positive and 40% after negative.
- Uninformative 50%/50% assay costing 1: EVSI zero and don't buy.
- Unverified claim of 90%/90% accuracy: the accepted model uses an uninformative 50%/50% likelihood, grants zero information credit, and doesn't buy it.

The values are exact integer/rational calculations under the synthetic model. They are not lunar prior probabilities, real survey sensitivity, geological confidence, dollars, or a project investment recommendation.

### Proposed targets

- EVSI is computed against the optimal baseline action; it cannot be negative when the information can be ignored and the best action retained.
- EVSI must never exceed EVPI under the same action/payoff model.
- A survey is purchased only if net incremental value is positive; all non-positive-value positive-cost surveys are rejected.
- Unsupported assay-performance claims contribute zero information credit.
- Increasing assay cost while leaving its likelihood model unchanged cannot improve net survey value.
- In later multi-stage versions, choose the next measurement only after assessing whether it can change the preferred route, reduce decision loss, or unblock a critical evidence gate. The model must represent survey fatigue, mission/lander time, sample limits and destructive sampling where relevant.

Research motivation is grounded in the [USGS lunar resource assessment](https://www.usgs.gov/publications/assessment-lunar-resource-exploration-2022) and [NASA's ISRU overview](https://www.nasa.gov/overview-in-situ-resource-utilization/), which highlight the gap between possible occurrence, site-specific characteristics, recoverability and qualified commodity production. Sources motivate the benchmark but do not calibrate these synthetic assay values.

Tracking issue: [BENCH-SPACE-009 — choose resource surveys by decision value, not data volume](https://github.com/Luminous-Dynamics/symtropy/issues/1813).

## Active reference extension — adaptive sequential resource exploration

Follow-up to [BENCH-SPACE-009](https://github.com/Luminous-Dynamics/symtropy/issues/1813): [BENCH-SPACE-010 — adaptive sequential resource exploration and conditional EVSI](https://github.com/Luminous-Dynamics/symtropy/issues/1823) adds `tools/space-economy-bench/examples/sequential_resource_exploration_reference.rs`. It extends the one-shot assay contract without modifying the frozen six-scenario v0.1 corpus.

The fixture models a named irreversible choice: build local oxygen production (+200 synthetic decision units for a rich site, -100 for a poor site) or use imports (0 in the model). Starting from a 50% prior:
- A low-cost screen with 70% sensitivity/specificity costs 1 unit; its exact posteriors are 70% after a positive and 30% after a negative.
- A 90%/90% follow-up assay costs 20 units. Conditional EVSI is 13 units on the positive branch, below cost, and 47 on the negative branch, above cost. Therefore the policy stops after a positive and runs the follow-up after a negative.
- Expected measurement spend is 11 units. Gross expected decision value is 78.5; net expected value is 67.5, compared with 65 for buying the high-resolution assay directly and 50 for taking no measurement. The adaptive policy's net advantage over the direct assay is 2.5 synthetic units in this fixture.
- A cost-60 negative control verifies that neither branch buys the follow-up.

All calculations use a common fixed integer joint-probability denominator. The implementation rejects an unexpected analytical result, reports the branch-conditional EVSI and decision posteriors, and emits a deterministic receipt tied to the exact source commit and code digest. The focused workflow now runs the example's unit tests, asserts key receipt invariants, and includes the new receipt in lock/toolchain provenance and artifact upload.

### Targets for the next benchmark generation

1. **Policy optimality:** on bounded finite-state fixtures, the selected adaptive policy must match an exhaustive enumeration oracle; regret target is exactly zero under the declared model.
2. **Arithmetic and mass integrity:** exact or bounded-rational probability mass conservation; branch probabilities sum to one; EVSI is non-negative when the decision maker may ignore information; EVSI never exceeds EVPI.
3. **Selective measurement:** zero follow-up purchases when branch-conditional EVSI is no greater than total incremental cost; no unverified performance claim may affect accepted likelihoods.
4. **Model robustness:** add correlated assays, spatially biased samples, missing observations, and misspecified likelihood negative controls. Independent evidence must not be treated as independent merely because it comes from separate instruments.
5. **Mission constraints:** add sample consumption, destructive sampling, elapsed time, power, access geometry and a hard exploration budget. Compare expected value per constrained resource only after satisfying mission and evidence gates.
6. **Calibration:** before any physical claims, validate predicted probabilities on held-out, independently ground-truthed data and report calibration error and decision regret against observed outcomes.

This is a synthetic algorithm contract, not a calibrated lunar exploration strategy. Its numbers are not geological priors, field-assay sensitivity, reserve estimates, actual costs, or mission investment recommendations. The USGS framework motivates treating resource certainty and recoverability as distinct states; it does not calibrate this fixture: [USGS lunar resource assessment](https://www.usgs.gov/publications/assessment-lunar-resource-exploration-2022).

## Active reference extension — correlation-aware resource evidence

[BENCH-SPACE-011](https://github.com/Luminous-Dynamics/symtropy/issues/1824) adds `tools/space-economy-bench/examples/correlated_assay_evidence_reference.rs`. This extends the sequential-exploration work without changing the frozen six-scenario v0.1 corpus.

The model has two hidden states that can invalidate naïve evidence fusion: (1) a shared calibration mode in which two assays reuse the same latent measurement result, and (2) a 20% chance that the sampled patch is not representative of the site-wide resource. Outcomes in the unrepresentative state are uninformative about the site-wide rich/poor state. Conditional on a representative patch, the A/B pair has a 50% shared-calibration mode; the emitted joint probabilities are exact fixed-denominator likelihoods.

| Portfolio/model | Gross value | EVSI | Cost | Net value |
|---|---:|---:|---:|---:|
| No measurement | 50 | 0 | 0 | 50 |
| A only | 61 | 11 | 4 | 57 |
| A+B with shared-mode likelihood | 66.7 | 16.7 | 10 | 56.7 |
| A+B under a false independence assumption | 70.62 | 20.62 | 10 | 60.62, rejected from selection |
| A+C with a separate, verified fixture failure domain | 75.66 | 25.66 | 9 | 66.66 |

The naïve model estimates the double-positive posterior at 89.01%, versus 81.08% under the joint likelihood that represents shared calibration and sample representativeness. Its gross expected value is 3.92 synthetic units higher than the correctly specified correlated model. The receipt retains the naïve calculation as a diagnostic negative control, but rejects it for selection because its independence claim conflicts with its provenance.

The complementary candidate uses a different calibration domain and a separately modeled sampling patch. The fixture assumes conditional independence only after those likelihood and domain contracts are explicitly given; merely naming two distinct instruments would not prove independence.

### Constraints and acceptance targets

- Exhaustive enumeration of this finite candidate set must select `complementary_assay_a_c` with exactly zero regret under the declared model.
- Every likelihood row must sum to 10,000 basis points and the joint observation masses must sum to the global probability denominator.
- Enforce hard budgets: at most two assays, two sample units, four power units and two elapsed hours. A three-assay overrun is rejected.
- Missing dependence metadata, uncalibrated measurements, unmodeled spatial representativeness, missing results and false shared-domain independence claims receive zero incremental evidence credit.
- Preserve (0 \le EVSI \le EVPI); probability calculations and policy scoring must not use floating-point rounding.
- For empirical claims, require independently ground-truthed site/epoch samples, calibration records, conditional-dependence estimates, held-out probability calibration and reported decision regret before setting acceptance thresholds.

The fixture is motivated by metrology practice that treats correlated uncertainty explicitly; see [NIST's correlation review](https://www.nist.gov/publications/correlation-uncertainty-measurement-discussion-state-art-techniques), [NIST covariance propagation guidance](https://www.itl.nist.gov/div898/handbook/mpc/section5/mpc55.htm), and this [review of Bayesian fusion with unknown correlation](https://www.sciencedirect.com/science/article/pii/S1566253517302890). These sources do not calibrate the numerical fixture. No result here establishes lunar resource recoverability, field-sample representativeness, assay performance, economic viability or mission readiness.
