# Causal Legibility Benchmark V0

Status: normative Living World validation contract. Documentation only.

## Purpose

Measure whether visible organism/ecosystem state carries recoverable information about the canonical causes that produced it.

The benchmark targets the Living World goal of **causal photorealism**: appearance should not merely vary; important variation should be explainable by development, physiology, damage, ecology, and history.

## Core principle

> Controlled changes in canonical history should produce stable, directionally characteristic changes in canonical phenotype/structure and, where intended, in presentation.

## First flora benchmark

Use one fixed hereditary/species baseline and four controlled histories:

1. control;
2. persistent drought;
3. persistent directional crosswind;
4. canopy competition / directional shade.

Keep unrelated inputs fixed.

### Canonical evidence first

Before rendering, compare appropriate canonical outputs such as:

- growth allocation trace;
- developmental exposure state;
- structural graph statistics;
- height/crown/root allocation proxies;
- asymmetry/directional growth summaries;
- organ/branch age distributions;
- damage/deadwood where applicable.

The benchmark fails causality if rendered trees differ while canonical biology does not.

### Presentation evidence second

For a fixed render scheme/profile, collect fixed-camera/fixed-path captures and derived visual metrics.

Candidate measurements include:

- silhouette/crown asymmetry;
- branch orientation distribution;
- foliage density/placement;
- material-state differences;
- trunk/branch taper and structural cues;
- persistence of historical scars/deadwood.

Raster identity is not required; the renderer's own non-authoritative stochastic/detail policy must be versioned and controlled where comparison depends on it.

## Fauna benchmark families

Future fauna causal-legibility fixtures should include controlled differences such as:

- healthy vs injured locomotor state;
- rested vs fatigued physiology;
- dry vs wet coat/material state;
- detected vs undetected threat history;
- learned-danger vs naive memory state;
- group alarm vs calm state.

Evaluate both canonical behavior and presentation cues without expecting every internal state to be visually obvious.

## History classification

A diagnostic classifier or rule-based observer may attempt to infer which controlled history generated an organism from canonical phenotype summaries or presentation-derived features.

If used, report:

- chance baseline;
- held-out scenarios/seeds;
- confusion matrix;
- accuracy/calibration;
- which features were available;
- whether classification used canonical state or pixels.

A classifier result supports causal separability; it does **not** prove human-perceived realism or artistic quality.

## Counterfactual pairs

Use paired scenarios differing in one controlled cause where practical.

Examples:

- same tree, add drought only;
- same tree, add one pruning event only;
- same animal, add hind-limb injury only;
- same habitat, add predator pressure only.

This improves attribution over comparing unrelated random organisms.

## Causal persistence

After a transient cause ends, distinguish reversible vs persistent effects.

For R2/R3 developmental/structural traits, removal of the current stressor should not erase already-grown historical form. For R0 transient physiology, recovery may appropriately remove the cue.

The benchmark should test the persistence class claimed by each trait.

## Anti-cheat tests

A benchmark fails if causal separability is achieved by hidden presentation labels unrelated to biology—for example assigning one texture variant by scenario name.

Perturbing/removing the scenario label while preserving canonical state must not erase the intended causal cue.

Likewise, changing renderer seed must not change which canonical history the organism actually experienced.

## Qualification direction

Initial pass criteria should be explicit but conservative. Examples:

- exact replay of canonical phenotype summaries for identical history;
- measurable directional divergence between intervention and control on declared metrics;
- R2/R3 differences persist after stress removal;
- fixed canonical state -> stable presentation-level causal class under allowed microvariation;
- simple observer exceeds chance on controlled history discrimination without access to scenario labels;
- human review confirms cues are plausible/readable rather than merely machine-separable.

Do not claim "photorealism proven" from these gates alone.

## Non-goals

This benchmark does not establish universal biological accuracy, human aesthetic preference, or scientific validity of every species parameter. It tests whether the implemented causal chain behaves and presents itself consistently with its declared model.