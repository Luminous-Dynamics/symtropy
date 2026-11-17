# Kitchen Action Ontology V1

This version introduces a hierarchical vocabulary for recipe actions based on six Action Core families: cutting, pouring, mixing, preparing, pick-and-place, and cook-and-cool. It is a planning ontology, not a robot-motion or appliance command language.

- `ActionOntologyBindingV1.schema.json`: binding schema for ontology version, action core/group, mapping provenance, typed parameters, tool capability references, precondition references and evidence class.
- `fixtures/action-ontology-v1.json`: deterministic rule-mapping matrix for every RecipeActionKindV1 value; consumed by the Rust tests in `src/action_ontology.rs`.

## Mapping rules and limitations

- A supported action kind maps by a deterministic table to one core and one group.
- `other` remains explicitly `unmapped`; no embedding/cosine/LLM guess silently becomes a group.
- A human-reviewed mapping can resolve an otherwise unmapped action only when it has a non-empty provenance reference and a core/group pair that belongs together.
- The action binding is optional on RecipeTaskNodeV1 to preserve older RecipeTaskGraphV1 documents.
- Parameters are typed fixed-point values. Missing values stay missing; tool and precondition references are descriptive capabilities/conditions, not device commands.
- The evidence class remains declared, modelled or simulated. A taxonomy mapping does not promote evidence.
- The action ontology cannot validate food safety, allergens, microbial lethality, hand-clearance, or robot safety, and it cannot command an appliance or robot.

## Research alignment

The 2025 Frontiers action-knowledge-graph study proposes six Action Cores and reports 76.5% verb coverage on Recipe1M+ after its lexical grouping method. That percentage is an external paper result under its corpus and protocol, not a Symtropy result or a precision measure. Our benchmark must separately report core coverage, group precision/recall, unmapped rate, human correction, and downstream graph feasibility.

Kitchen-R (2026) separates planning, low-level control and integrated performance across more than 500 instructions. Symtropy's ontology contributes to planning/grounding only; low-level control and integrated physical success remain separate qualification tracks.
