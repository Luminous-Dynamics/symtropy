# Kitchen Action Ontology V1

This version introduces a hierarchical vocabulary for recipe actions based on six Action Core families: cutting, pouring, mixing, preparing, pick-and-place, and cook-and-cool. It is a planning ontology, not a robot-motion or appliance command language.

- `ActionOntologyBindingV1.schema.json`: ontology version, action core/group, mapping provenance, typed parameters, capability references, precondition references and evidence class.
- `fixtures/action-ontology-v1.json`: deterministic mapping matrix for all current RecipeActionKindV1 values; consumed by the Rust tests in `src/action_ontology.rs`.

## Mapping rules and limitations

- Supported action kinds map by a deterministic table to one core and group.
- `knead` is categorized as Preparing in alignment with the cited Actionable Knowledge Graph study, not as Mixing.
- `add`, `combine`, `whisk`, `drain`, `halve`, `cover`, `cook`, `heat` and `freeze` are represented as explicit groups. Ambiguous verbs such as `other` remain unmapped.
- `other` stays `unmapped`; no embedding/cosine/LLM guess silently becomes an action group. A human-reviewed mapping needs a non-empty provenance reference and a valid core/group pair.
- The binding is optional on RecipeTaskNodeV1 to preserve older RecipeTaskGraphV1 documents.
- Parameters are typed fixed-point values. Missing values stay missing; capability and precondition references are not device commands.
- Evidence class stays declared, modelled or simulated. A taxonomy mapping does not promote evidence.
- This ontology cannot validate food safety, allergens, microbial lethality, hand-clearance or robot safety; it cannot command an appliance or robot.

## Research alignment

The 2025 Frontiers Actionable Knowledge Graph study identifies six Action Cores and Action Groups, and reports 76.5% verb coverage on Recipe1M+ after direct matching plus cosine-similarity matching. Its study also reports limitations around less common verbs, simplified parameterization and the gap between simulation/selected demonstrations and real-world kitchen deployment. The 76.5% figure belongs to that paper and protocol, not Symtropy. It is coverage, not precision, task success or food-quality performance.

Our benchmark therefore reports core coverage, group precision/recall, unmapped-verb rate, human correction, unsupported mapping rate and downstream task-graph feasibility separately. Compare only on a matched licensed corpus and reproduced preprocessing/matching protocol.

Kitchen-R (2025 publication) provides 500+ language instructions and separates planning-only, low-level control and integrated evaluation in an Isaac Sim digital twin. Symtropy's ontology contributes only to recipe/action grounding. Low-level control, integrated robot success and physical food preparation remain separate qualification tracks.


## Executable benchmark evaluator

- `ActionOntologyBenchmarkV1.schema.json` defines labelled benchmark cases with an explicit `gold_group`, an ontology binding and a positive occurrence weight.
- `fixtures/action-ontology-evaluation-v1.json` is a synthetic weighted fixture consumed by the Rust unit tests.
- `evaluate_action_ontology_benchmark` validates every prediction before scoring and returns exact integer weighted counts for core coverage, micro group precision/recall, correct abstention, and per-group gold/predicted/true-positive counts. Invalid predictions, duplicate IDs, zero weights and arithmetic overflow fail the evaluation.
- Macro precision/recall are derived from the per-group counts under a predeclared class-inclusion convention; keep macro and micro values separate.

The published 76.5% verb coverage from the 2025 Actionable Knowledge Graph paper is only a corpus/protocol reference. Reproduce its normalization and split before claiming a direct comparison. Do not use its coverage figure as a precision target.
