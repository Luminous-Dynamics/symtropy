# RecipeTaskGraphV1

Status: proposed, schema and Rust mapper on the recipe-graph branch. The graph maps into the existing KitchenPlanV1 scheduler; it is not a second planner.

- `RecipeTaskGraphV1.schema.json`: typed recipe, rights, ingredient quantity and action-graph envelope.
- `fixtures/recipe-task-graph-v1.json`: synthetic positive and adversarial fixtures, consumed by the Rust unit-test suite via `include_str!` and `serde_json`.
- `src/recipe_graph.rs`: Rust types, validator and mapper to KitchenPlanV1.

## Important boundaries

- Original recipe text is not embedded. Store a source pointer, digest and attribution/permission references. Respect the rights status; `unknown` or `restricted` sources are local-processing only. Redistribution or model-training uses require explicit permission evidence. Attribution alone is not permission.
- Ingredient quantities use an integer coefficient, decimal scale and explicit unit. No implicit gram-to-millilitre conversion occurs without a separate density/method record.
- The mapper copies KitchenTaskV1 fields (task IDs, duration, resources, dependencies and timing bounds) exactly. It does not generate missing timing or resource assumptions.
- A graph rejected by human review cannot be mapped. A reviewed graph is still not a food-safety or culinary-quality validation.
- The planner is advisory. It does not actuate appliances or override local safety systems.
- Evidence class remains declared/modelled/simulated; human review does not promote modelled evidence into measured evidence.
- A structurally valid graph can still fail runtime validation for unknown ingredient references, duplicate IDs, invalid schedule DAGs or rights-scope mismatch.

The planner test target deserializes every fixture into the Rust types and checks each runtime-validity expectation. JSON Schema shape and Rust cross-record invariants remain separate layers; the Rust tests are the executable authority for task/reference semantics.

Run through the existing planner test target:
`cargo test -p symtropy-kitchen-planner --all-targets`

The branch layers on the power-aware planner PR: https://github.com/Luminous-Dynamics/symtropy/pull/2230


## Optional action ontology enrichment

Each recipe task may optionally carry an `ActionOntologyBindingV1`:
- six Action Core families and explicit Action Groups;
- mapping method (`rule_based`, `human_reviewed`, or `unmapped`);
- provenance reference for mapped actions;
- optional typed fixed-point parameters, tool capability references and precondition references;
- declared/modelled/simulated evidence class.

The field is optional for backward compatibility. A missing binding means no ontology enrichment was performed. `Other` stays unmapped by default. Human-reviewed mapping requires its own provenance and a valid core/group pair. Parameters and capability/precondition references are descriptive only; no hardware command or robot trajectory can be emitted.

The action taxonomy is aligned with the six-core actionable-knowledge-graph research, but Symtropy's mapping table is a deterministic V1 subset. Do not compare raw coverage to the paper's 76.5% until the same corpus, normalization and similarity protocol are reproduced.


## Hierarchical action ontology integration

Each task node has an optional `ontology_binding`. When present, the recipe graph validator checks the ontology version, mapping provenance, action core/group coherence and typed references. A malformed binding blocks mapping to `KitchenPlanV1`. Existing graphs without the optional binding remain valid for backwards compatibility.

The current action vocabulary has 38 variants. The canonical Rust list and fixture test require exactly one fixture mapping per verb. Unsupported `other` actions remain unmapped by default. A human-reviewed mapping requires a provenance reference. This taxonomy does not authorize robot or appliance control.
