# Animal Kingdom Coverage and Cross-Taxon Qualification V0

Status: program-level design contract. Documentation only; it defines coverage and qualification requirements, not a claim that animal simulation is implemented or biologically validated.

## Purpose

Define how Symtropy and Symthaea can expand from a bounded set of fauna primitives toward broad animal-kingdom coverage without creating a separate simulator for every species, treating mammals as the default animal, or claiming realism from visual resemblance alone.

This document complements, and does not replace, the existing Living World contracts for canonical body plans, perception, homeostatic behavior, locomotion/contact, memory/social continuity, damage/recovery, signals, group dynamics, niche construction, species/developmental biology, and causal qualification.

## Paradigm shift: composable biology, not a zoo of bespoke agents

The target is a **capability-composed animal model**:

`taxon identity + body plan + physiology + sensory capabilities + life history + behavior policy + ecology`

Species-specific differences should be represented as qualified data and composition of shared mechanisms wherever those mechanisms are genuinely shared. They must not be reduced to one universal "animal brain" or one rig/controller with cosmetic variations.

The simulation must distinguish:

- **Lineage/taxonomy** — what organism the model represents, using a versioned external identifier and lineage path.
- **Capability** — a mechanism the organism can actually use: a body-support structure, movement mode, receptor, feeding strategy, developmental transition, or ecological effect.
- **Evidence** — the source, method, conditions, uncertainty, rights, and scope supporting each consequential capability or parameter.
- **Representation/fidelity** — how much detail is active now, without silently changing the organism's identity, known history, or future-bearing state.
- **Presentation** — meshes, rigging, animation, sound and effects derived from canonical state; presentation is never biological authority.

No finite Rust enum of phyla or species should be treated as a complete or timeless taxonomy. Taxonomy identifiers and lineage membership must be versioned and allow additions without changing canonical simulation semantics.

## Cross-taxon capability axes

A versioned Animal Capability Manifest should eventually express the axes below. V0 does not prescribe one giant struct: independent capability modules may be optional, staged, and owned by their relevant domain.

1. **Identity and provenance:** taxon ID and version; stable model/profile ID; common/scientific names when source-backed; developmental stage; profile revision; source rights; evidence records.
2. **Morphology and support:** bilateral/radial/asymmetric body organization where relevant; segmentation; body regions; soft tissue; hydrostatic support; exoskeleton; endoskeleton; shell/plates; appendages; body-size range; growth and regeneration capability.
3. **Movement:** sessile/anchored; ciliary; peristaltic; crawling; walking/running; hopping; climbing; burrowing; undulation; swimming; jet propulsion; gliding; powered flight; combinations and stage-dependent transitions.
4. **Sensation and signals:** vision and spectral range; hearing/acoustics; chemical sensing; touch/contact; vibration; temperature; flow/hydrodynamic sensing; electroreception; magnetoreception; echolocation and other specialized modalities when evidence supports them. Signal generation, propagation, receptor sampling, percept and belief remain distinct causal stages.
5. **Physiology and energy:** intake and digestion; respiration/gas exchange; circulation or transport; water/salt balance; thermal strategies; oxygen and temperature tolerances; fatigue, recovery, injury, disease susceptibility, growth and energetic budgets.
6. **Feeding and interaction:** filtering; grazing/browsing; predation; scavenging; detritivory; parasitism; nectar/pollen use; symbiosis; omnivory; food handling and species-specific constraints.
7. **Life history:** sexual/asexual reproduction where applicable; reproductive investment; eggs/embryos/larvae/nymphs/juveniles/adults; metamorphosis; parental care; dispersal/propagules; dormancy; migration; senescence; mortality and decomposition. Unsupported life-stage transitions must not be invented during load, promotion or catch-up.
8. **Learning and behavior:** reflexes; fixed action patterns; exploration; habituation; associative learning; spatial memory; social learning; planning or flexible problem-solving only where justified; bounded memory and explicit uncertainty.
9. **Social organization and communication:** solitary, pair, family, herd, flock, school, swarm, colony, and fission/fusion patterns as appropriate; calls, visual displays, chemical signals, touch, and environmental traces; individual-specific relationships only when supported by persistent identity/history.
10. **Ecological function:** resource acquisition; predation pressure; pollination; seed dispersal; filtering; grazing/browsing; burrows; nests; reef/soil/sediment modification; nutrient/waste deposition; scavenging; parasitism; symbiosis; and other habitat effects.
11. **Failure and recovery:** missing or impaired appendages, sensory loss, malnutrition, dehydration, temperature stress, injury, disease burden, failed reproduction, and recovery when the declared biology permits it.
12. **Scale and information sufficiency:** what can be represented as a population aggregate, what requires an active individual, and what requires high-fidelity embodiment. Changing representation must conserve counts, biomass/resource accounting, persistent identity and all future-bearing facts the coarse representation claims to retain.

A capability may be **present**, **absent**, **unknown**, **not measured**, **not applicable**, or **conflicted**. Unknown is not false; missing evidence is not permission to fill in a convenient default. Incompatible evidence must remain visible.

## Coverage portfolio

Coverage is measured along two separate dimensions:

- **Taxonomic breadth:** which registered lineages have at least one reviewed profile and explicit evidence status.
- **Functional breadth:** which body plans, locomotor modes, sensory strategies, life histories and ecological roles are exercised by adversarial scenarios.

Counting species alone is not an adequacy metric. A large catalog of similar terrestrial mammals would not cover the animal kingdom.

The initial cross-taxon portfolio must include (at minimum) synthetic archetypes for these distinct strategies before real-species calibration is claimed:

- soft-bodied, hydrostatic, crawling/undulating and burrowing animals;
- segmented bodies with peristaltic locomotion;
- shelled and muscular-foot locomotion, plus filter-feeding and jet-propelled variants;
- arthropods with multiple appendage types, molting/size discontinuities, and at least one flight-capable life stage;
- aquatic undulators with flow-sensitive perception;
- powered flight with air-relative dynamics, takeoff/landing and energy cost;
- radial or weakly cephalized body plans with distributed or stage-dependent sensing;
- sessile/anchored filter feeders whose environment changes through water/particle transport;
- animals with complex metamorphosis or major stage-specific body-plan change;
- solitary and collective organisms whose group behavior emerges from local, non-omniscient interactions;
- specialized sensory strategies (for example, echolocation, electroreception, or magnetic orientation) represented only with modality-specific evidence and declared limitations;
- predator, prey, scavenger, detritivore, parasite, mutualist and ecosystem-engineer relationships.

This is a minimum functional portfolio, not a claim that the listed synthetic profiles represent all living species or that their numerical parameters are measured biology. The implementation plan must then map every recognized animal lineage in a pinned taxonomy snapshot to `covered`, `profile_in_progress`, `unsupported`, or `not_reviewed`, with a reason and source/provenance state.

## Causal contract

Every active animal follows a causally inspectable loop, adapted to the organism's declared capabilities:

`world/ecological event -> signal or contact -> propagation/attenuation -> receptor sample -> percept -> belief/memory -> homeostatic appraisal -> intent -> controller/locomotion/contact -> resource and ecological settlement -> new world signals and traces`

A reduced model may collapse or approximate stages, but the profile and benchmark must declare which stages were omitted and which claims are therefore unsupported. The renderer, animation clock, player proximity, debug UI, and hidden global-state queries may not create canonical perception, decisions, movement, injury, reproduction, or habitat effects.

Fast controllers and physical constraints own immediate motor feasibility. Higher-level Symthaea/ANIMA cognition may set bounded goals or select among feasible actions, but may not bypass contact, energy, authority, or safety constraints.

## Qualification suites and benchmark aims

Use frozen, deterministic synthetic fixtures first. Add measured/reconstructed data only with method, units, conditions, sample limitations, species/stage match, source rights, and an explicit validation split.

| Suite | Required measurements | Hard gate / target |
|---|---|---|
| Profile integrity | ID/version stability, typed capabilities, unknown/conflicted values, evidence coverage | 100% of malformed profiles rejected; no silent capability inference |
| Taxonomic/functional coverage | lineage snapshot, covered capability axes, explicit gaps | 100% of registered lineages receive a disposition; no claim of complete coverage while gaps remain |
| Body/contact | joint/contact bounds, slip, penetration, terrain support, appendage damage | Zero impossible contact/teleport cases in the frozen adversarial fixtures; species-specific continuous-error bounds preregistered before validation |
| Perception | detection, missed detections, false alarms, range/orientation/occlusion sensitivity, source tracing | Zero false omniscience on the frozen negative-control suite |
| Behavior | action choice, task completion, energy/resource cost, abstention under uncertainty | Report per-task/per-species baselines and confidence intervals; no single aggregate score may hide a failed capability |
| Physiology/life history | energy and mass/stock balances, development transitions, survival/reproduction under declared scenarios | Exact conservation where integer/count authority applies; numeric tolerances established per model and units |
| Social/ecological dynamics | group membership, encounter rates, feeding/competition, population trajectories, habitat effects | Zero duplicate individuals or unaccounted stock transfer at representation boundaries; compare trajectories against held-out empirical reference where available |
| Persistence/fidelity | save/reload, offscreen catch-up, promotion/demotion, region transfer, CPU/GPU/backend parity | Exact equality for authority/identity/count invariants; stated numerical tolerance only for measured continuous outputs |
| Causal presentation | headless vs rendered, low/high render LOD, frame-rate perturbation | Zero renderer-driven changes to canonical decisions, physiology, identity, or ecological settlements |
| Generalization | held-out terrain, conditions, individuals and life stages | Report transfer failure and uncertainty explicitly; unseen species or contexts do not inherit validated status |
| Performance | entities, active individuals, population cells, p50/p95 tick time, memory, catch-up cost | Publish reproducible workload/hardware baseline; preserve quality and authority gates while optimizing. Throughput is not a substitute for validity. |

A known-answer synthetic corpus should contain at least 24 distinct profiles and 40 hostile cases spanning missing/conflicting/unsupported capabilities, species-stage mismatch, impossible body/controller combinations, hidden-signal leakage, stale memories, invalid stock transfer, interrupted life cycles, representation changes and renderer/FPS manipulation. Expected dispositions must be independently specified rather than generated by the implementation under test.

Targets of 100% apply to frozen logical invariants and known-answer cases, not to agreement with every real animal. Empirical accuracy must be assessed per capability and species against independently sourced held-out observations. Do not use a single universal "animal realism score."

## Data, scientific, and welfare boundaries

- Label records and results as **measured**, **reconstructed**, **modeled**, or **synthetic**; never relabel a simulation output as observation.
- Preserve uncertainty, contradictory findings, study conditions, individual variation, and limitations. Population averages cannot erase individual variation when it changes future behavior.
- Use explicit provenance and source-use rights. Unknown permission blocks corresponding redistribution/training use; attribution-required sources retain their attribution.
- Do not infer consciousness, sentience, emotion, pain, welfare, or subjective experience from movement resemblance, reward optimization, neural architecture, or a benchmark score. These require separate, carefully defined evidence and claims.
- In any application that informs animal care, wildlife management, veterinary decisions, conservation policy, or experiments on living animals, simulation output remains decision support and requires domain-expert review. This contract does not authorize physical trials or substitute for animal-welfare review.
- No benchmark may imply that a real species is safe to handle, suitable for captivity, ecologically interchangeable, or fully represented because one capability test passed.

## Delivery sequence

1. **Coverage contract and synthetic oracle:** freeze the capability vocabulary, extensible taxonomy IDs, coverage matrix, independent expected decisions, and negative controls.
2. **First cross-taxon executable core:** implement profile validation and deterministic coverage reporting in Rust, independent from rendering/Bevy. Do not start with a universal neural policy.
3. **Body/contact verticals:** qualify one representative of each distinct locomotor/support strategy, keeping canonical body state separate from rigs.
4. **Sensorimotor verticals:** integrate typed signals, receptor sampling, bounded perception and feasible action selection; use hidden-state and occlusion negative controls.
5. **Physiology and continuity:** connect energy, water/temperature stress, injury, development, reproduction, mortality and persistence through explicit owners.
6. **Multispecies ecology:** qualify food webs, social groups, parasites/symbioses, life-stage recruitment, habitat modification and coarse population settlement.
7. **Evidence-backed expansion:** add real profiles one capability at a time, preregister validation thresholds, retain held-out data, and publish known gaps for every taxon.

Every implementation change must identify the exact contract/corpus digest and Git HEAD it qualifies. Static review, an authored test, a queued workflow, and a passing exact-head test are four different evidence states. Missing, skipped, wrong-head, stale, or queued checks are not PASS.

## Relationship to existing work

- [Symtropy PR #244](https://github.com/Luminous-Dynamics/symtropy/pull/244): animal embodiment, perception, behavior, locomotion, signals, groups and ecological effects.
- [Symtropy PR #245](https://github.com/Luminous-Dynamics/symtropy/pull/245): causal realism and cross-fidelity qualification.
- [Symtropy PR #230](https://github.com/Luminous-Dynamics/symtropy/pull/230): species, heredity, development, reproduction and recruitment.
- [Symtropy PR #246](https://github.com/Luminous-Dynamics/symtropy/pull/246): first executable non-omniscient fauna perception oracle.
- [Symtropy PR #1207](https://github.com/Luminous-Dynamics/symtropy/pull/1207): dependency-neutral ANIMA canonical substrate.

These references are architectural inputs, not proof that their branches are merged, all gates pass, or the end-to-end animal simulator exists.

## Non-goals

No complete catalog of all species in this document; no invented real-species parameters; no claim of biological fidelity; no neural/LLM dependency requirement; no visual asset collection; no duplicate simulation authority; no automatic animal-care or conservation decision; no weakening of evidence, rights, or exact-head qualification requirements.
