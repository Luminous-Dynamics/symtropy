# Plant Material Causality V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define how plant visual material channels derive from biological state so bark, wood, leaves, wounds, wetness, senescence, disease, and dead tissue communicate causal history rather than arbitrary shader randomization.

## Core invariant

> A visible biological condition should be driven by canonical/qualified biological state whenever that condition is meant to communicate organism history or physiology.

Presentation noise may add microscopic variation. It may not invent future-bearing disease, damage, hydration, age, or phenological state.

## Candidate biological inputs

Material state may derive from qualified inputs such as:

- organ/tissue age;
- live/dead status;
- phenological phase;
- hydration/water status;
- chlorophyll/pigment proxy;
- senescence progress;
- wound/repair state;
- disease/pathogen load or lesion state;
- fire/heat damage;
- nutrient-stress response;
- surface wetness from environment;
- bark maturation / structural tissue class.

Each input remains owned by its biological/environmental authority layer.

## Material outputs

Derived channels may include:

- base reflectance / pigment coordinates;
- roughness;
- normal/displacement amplitude;
- subsurface/transmission proxy;
- leaf translucency/thickness response;
- wetness film amount;
- wound/scar masks;
- necrosis/senescence masks;
- fungal/lichen presentation masks when biologically supported;
- deadwood weathering state.

The exact PBR/shader representation is renderer-specific.

## Causal legibility

The goal is not merely variation. The visual should preserve directional evidence:

- drought-stressed foliage should differ for reasons traceable to hydration/development state;
- old bark should look older because tissue age/maturation differs;
- a healed wound should reflect the retained damage/repair history;
- senescing leaves should follow phenological/physiological state;
- wet surfaces should track qualified environmental wetness rather than random gloss pulses.

## Random microvariation boundary

Keyed deterministic microvariation is allowed for details whose exact microscopic cause is outside simulation scope, for example pores, tiny pigment noise, bark microcracks, or sub-leaf speckling.

Such variation must be:

- keyed/call-order independent;
- bounded by the biological material class;
- non-authoritative;
- incapable of changing ecology when renderer settings change.

## Temporal behavior

Material changes that represent biology follow authoritative state/cadence. Presentation may visually interpolate between canonical states to avoid popping.

Interpolation cannot advance the biological endpoint itself.

For example, a leaf may visually blend toward a newly committed senescence state over several render frames; the renderer cannot decide the plant has senesced because the blend completed.

## Scale consistency

Material cues should remain coherent across LOD. A diseased or scarred landmark plant must not become visually healthy simply because it switches to a distant representation if that cue remains perceptually relevant.

The LOD system may simplify detail, but important causal state should survive through aggregate masks/material classes or equivalent representations.

## Qualification direction

At minimum test/measure:

1. canonical material-state derivation is deterministic for a fixed source revision/version;
2. changing renderer FPS cannot change biological material inputs;
3. healthy vs drought/senescence/wound fixtures produce directionally distinct material-state outputs;
4. remeshing preserves stable wound/scar attachment semantics;
5. non-authoritative microvariation cannot alter canonical state;
6. important damage/disease/phenology cues persist across qualified LODs;
7. same current environment but different retained wound/development histories can produce different materials where history matters;
8. headless ecology remains identical to rendered ecology.

## Non-goals

This contract does not prescribe final textures, scan sources, shader models, artistic grading, or universal botanical optical parameters.