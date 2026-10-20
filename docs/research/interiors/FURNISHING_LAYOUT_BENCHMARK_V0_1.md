# Furnishing Layout Benchmark v0.1

**Status:** versioned synthetic benchmark contract; the v0.1 independent Rust oracle has a focused hosted pass, but no real-room/product or human-outcome qualification is claimed.  
**Workstream:** [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766).  
**Complementary prototype:** [occupant-led architectural review PR #1993](https://github.com/Luminous-Dynamics/symtropy/pull/1993).  
**Scope:** Symtropy geometry/constraint substrate with a future Symthaea alternative-generation interface.

## Paradigm shift

Move from "place attractive furniture in a rendered room" to **design a usable, adaptable interior whose functional claims can be checked independently**.

A furniture item is more than a mesh and a bounding box. In a future qualified model, it may expose separate, versioned geometry and evidence for:

- physical footprint and permitted orientations;
- approach, transfer, reach, and task-operation envelopes;
- maintenance, cleaning, repair, and replacement access;
- relationships to outlets, windows, lighting, airflow, acoustic/privacy zones, circulation and other furnishings;
- material, repair, reuse, disassembly, local fabrication, and lifecycle-cost evidence;
- resident-authored preferences and the provenance/authority of each preference.

These are distinct facets. A visual fit is not proof of approach access, code compliance, thermal comfort, acoustic performance, or maintainability.

## What v0.1 does—and does not—model

The fixture contract is deliberately small: room-local floor coordinates, metres, axis-aligned 2D rectangles, and 0/90/180/270-degree rotations. Four synthetic scenarios cover a blocked circulation route, blocked maintenance access, competing resident preferences, and unknown real-site geometry.

The frozen zones and expected outcomes are **synthetic known-truth oracles**, not legal clearances. There is no qualified door-swing, wheelchair turning, reach-range, egress, structural, electrical, daylight, thermal, ventilation, acoustic, fire, or cost model in this version. The fixture's preference scores are synthetic test inputs, not user research or measured comfort. The nominal retrofit dimensions must not be treated as a measured site.

Files:
- `fixtures/interiors/furnishing_layout_scenarios_v0_1.json`
- `fixtures/interiors/furnishing_layout_scenarios_v0_1.schema.json`

The schema checks record shape. A future independent validator still needs to check cross-record identity references, duplicate IDs, placement bounds after rotation, rectangle intersections, oracle agreement, and blocked-promotion rules.

## System responsibilities

- **Symtropy:** own deterministic geometry, geometric relationships, scenario execution, and the separation between a geometry result and a domain-qualified engineering claim.
- **Symthaea:** propose candidate arrangements and explain trade-offs from explicitly provided tasks and preferences; preserve conflicts and missing inputs; abstain instead of inventing dimensions or inferring preferences from demographic proxies.
- **Mycelix (future adapter):** preserve household participation, consent, provenance, dissent, and decision rights. Governance records do not prove physical performance.
- **Sol-Atlas (future adapter):** provide site, climate, orientation, and solar context only from a qualified, versioned source with units, coverage, epoch, and uncertainty.

These are intended boundaries, not claims that the cross-repository adapters exist today. Reuse [ACCESS-HOME-001 #1687](https://github.com/Luminous-Dynamics/symtropy/issues/1687) for resident-led accessibility and task requirements; this benchmark must not create a competing accessibility authority.

## Benchmark gates

All items below are **proposed acceptance gates**, not measured results.

### 1. Integrity and epistemic correctness

- 100% of fixture IDs and references resolve, with no duplicate scenario, item, zone, requirement, or alternative IDs.
- 100% of placements are within the declared nominal room after applying rotation; report overlap separately from out-of-bounds geometry.
- Identical frozen inputs, configuration, and solver revision replay deterministically in the declared deterministic harness.
- Any missing/stale geometry, unsupported model facet, unresolved mandatory requirement, or conflicting hard constraint remains unknown/blocked—not silently assumed satisfied.
- A render, generated layout, or confidence score never upgrades a candidate to verified engineering or regulatory status.
- Independent checking recomputes results from the exact fixture and implementation revision rather than trusting solver-provided summary booleans.

### 2. Constraint-oracle correctness

Compare each solver's decisions with the frozen synthetic oracle. Report a full confusion matrix by check type and fixture; do not combine hard constraints and preferences into a single score.

- **Safety-critical false-safe target:** zero oracle-known mandatory failures classified as pass.
- **Unknown-input target:** zero candidates promoted beyond blocked/candidate-only status when a required production input is unknown.
- **Negative controls:** move a furnishing into a keep-clear zone; remove a catalog item; duplicate an ID; change geometry without changing the revision; place a rectangle just outside the room; rotate a non-square object; tamper with an expected outcome. Each mutation must be detected or the run must be explicitly invalidated.
- Record false positives and over-blocking too. A system that abstains on everything is not useful; correct abstention and correct resolution must both be measured.

### 3. Multi-objective design quality

Keep hard feasibility as a gate, then report each objective independently. Candidate dimensions can include task completion, circulation quality, reach/access, privacy, resident preference fulfillment, acoustic/daylight/thermal outcomes when qualified models exist, cleaning and maintenance access, materials, lifecycle cost, reversibility and adaptation effort.

- Publish Pareto relations and the underlying objective vector. Do not hide contested preferences behind one default weighted "best room" score.
- Missing or non-comparable values remain missing/incomparable; no zero-filling or imputation.
- Keep disagreements visible by household member, task and context. An average must not erase a severe cohort-specific failure.
- Compare scenario-wise regret only against an explicitly defined reference policy and declared preference inputs.
- Require each objective observation to identify its source, model/version, units, and uncertainty; unavailable models are not "passed" by visual inspection.

### 4. Performance and scaling

Establish results only after an implementation and a pinned run harness exist. Suggested scene tiers are 10, 25, 50, 100 and 500 furnishing objects, with separate counts for zones, requirements and candidate alternatives. Report p50/p95 latency, peak memory, candidate throughput, hard-constraint accuracy, Pareto-frontier size, and completeness/truncation state on named hardware.

Do not announce a speedup until the baseline and candidate use identical fixtures, hardware, stopping criteria and correctness gates. Search caps must produce an explicit incomplete result; a truncated search cannot claim the global best or exhaustive Pareto frontier.

## Baselines and SOTA comparison plan

A fair initial comparison should separate these tasks instead of declaring one product "better":

1. **Feasibility baseline:** naive rectangle placement/packing, with no task reasoning.
2. **Rule-based baseline:** deterministic hard-constraint-first placement and a documented greedy task-priority policy.
3. **Optimization candidate:** constrained search or multi-objective optimization with independent verification and explicit search caps.
4. **Human reference:** layouts created or reviewed by qualified designers and paid co-design participants, with review protocol and disagreement retained.
5. **Industry workflow:** where legal access and export formats permit, use equivalent room geometry and task/constraint inputs in established CAD/BIM and interior-planning workflows. Record software/version, features available, operator time, manual edits and export loss. Do not equate a rendering-quality comparison with a feasibility comparison.

### Academic reference: SceneEval (WACV 2026)

[SceneEval](https://openaccess.thecvf.com/content/WACV2026/html/Tam_SceneEval_Evaluating_Semantic_Coherence_in_Text-Conditioned_3D_Indoor_Scene_Synthesis_WACV_2026_paper.html) provides a useful adjacent benchmark for text-conditioned 3D indoor scene synthesis. Its SceneEval-500 set has 500 annotated prompts and evaluates explicit requirements (object counts, attributes, spatial relations) plus implicit plausibility (support, collisions, navigability). It is especially relevant because realism-only metrics can overlook whether the scene meets a stated task.

Do **not** compare our v0.1 2D synthetic oracle scores directly against SceneEval scores: the tasks, scene representations, annotations and metrics are different. Instead, use its evaluation dimensions to structure a later matched-prompt track: (1) requirement fidelity, (2) geometric plausibility, (3) task access/navigability, and (4) evidence/currentness. Preserve the task-matched, frozen-input and independent-oracle requirements above.

### Interoperability reference: IFC + IDS

[buildingSMART IDS v1.0](https://www.buildingsmart.org/information-delivery-specification-ids-v1-0-is-approved-as-a-final-standard/) became an approved final standard in June 2024. It describes machine-interpretable requirements for IFC object classification, properties, values and units. Treat it as a baseline for product/asset data exchange and lossless schema validation—not as a furniture-layout solver or a truth source for seller claims. Record which vendor exports can be obtained and how much information is lost.

The first result should answer: **does the system reliably catch known circulation and maintenance conflicts while preserving unknowns and household trade-offs?** Only after passing that gate should we make comparative claims about usability, design quality, speed, resource use or adaptation cost.

## Execution sequence

1. Add a Rust-first independent fixture auditor: schema shape, reference integrity, rotated bounds, rectangle/zone intersections, deterministic canonical output, expected-oracle agreement, and mutation/negative-control tests.
2. Add a minimal Symtropy adapter that consumes a versioned furnishing catalog plus room/zone/task requirements. Keep it renderer-independent.
3. Add the first deterministic candidate generator; compare it to the rule-based baseline and keep hard feasibility outside preference optimization.
4. Connect occupant-reviewed variants to the existing architectural review and owner-led Pareto contracts; require a revision-pinned feedback-to-change trail.
5. Introduce 3D and specialized performance models one at a time, with separate validation and explicit applicability envelopes.
6. Conduct consent-based, compensated co-design before making real-world claims about comfort, accessibility, cultural fit, or preference quality.

## Current validation boundary

The v0.1 independent Rust oracle now checks fixture IDs/references, rotated room bounds, footprint and zone intersections, expected-outcome agreement, unknown-site abstention, and blocked promotion. Its focused pinned workflow passed at exact head `c39ae230044299ae25a383a2f1961eabd8bfb710` in [run #39395023546](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39395023546). The stacked v0.2 front-functional-envelope oracle later passed 18 tests at exact head `4841222c03a26ac90be4ab18aa86273e0bf0c264` in [run #39445299953](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39445299953).

These are synthetic oracle qualification results only. No measured-room accuracy, live Marketplace fit, physical-performance model, accessibility/code compliance, or compensated usability study is established. This remains a benchmark and constraint substrate—not a production furniture optimizer or qualified interior-design product.


## Marketplace composition

The furnishing benchmark now has a proposed, effects-disabled offer-profile companion in [Mycelix PR #5251](https://github.com/Luminous-Dynamics/mycelix/pull/5251). Keep the concerns separated:

- Symtropy exports a revision-pinned design context, geometry/zone constraints, units, evidence state, and which geometric checks were actually performed.
- Mycelix Marketplace returns existing listing references and source-attributed item/offer profiles. The profile must not duplicate authoritative price, availability, seller, order or transaction state.
- Symthaea can propose alternatives and explain trade-offs, but a missing or unlicensed geometry asset cannot be treated as a verified fit.
- Reuse, repair and refurbished candidates belong in the shortlist, but seller assertions about reclaimed material, repairability or sustainability remain unverified unless sourced evidence justifies a stronger claim.
- Exploration is read-only and effects-disabled. No buyer authorization, checkout, reservation, payment or public floor-plan disclosure is implied by a compatibility check.

The current Marketplace implementation is pre-alpha; the new profile is a contract proposal, not a functioning adapter. Do not change the production Marketplace transaction path until the profile is mapped field-by-field to its existing wire contracts and its independent checks pass.


## Independent Rust fixture oracle

The isolated `tools/furnishing-layout-audit/` crate now independently audits the synthetic fixture and its expected outcomes. It checks scenario/catalog/zone/requirement identities, reference integrity, nominal room bounds after rotation, rectangle collisions between furnishings, expected keep-clear-zone outcomes, unknown-site abstention, mandatory-requirement blocking, and the synthetic preference Pareto relation.

Run it with:

```sh
cargo fmt --manifest-path tools/furnishing-layout-audit/Cargo.toml -- --check
cargo test --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml
cargo clippy --locked --manifest-path tools/furnishing-layout-audit/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/furnishing-layout-audit/Cargo.toml
```

The checker is an independent **synthetic 2D rectangle oracle**, not an architectural solver. Its tests prove only that these fixture inputs, cross-references and expected outcomes are internally consistent. The fixture coordinates and clearances are not building rules; successful validation does not establish accessibility, egress/code compliance, physical performance, manufacturability, or suitability of a real dwelling. The dedicated exact-head workflow checks formatting, mutation tests and Clippy before this helper is treated as a maintained benchmark gate.

The independent audit crate includes a committed `Cargo.lock`; the dedicated workflow uses `--locked` for test and Clippy runs.
