# RecipeTaskGraphV1

Status: proposed, schema and Rust mapper on the recipe-graph branch. The graph maps into the existing KitchenPlanV1 scheduler; it is not a second planner.

- `RecipeTaskGraphV1.schema.json`: typed recipe, rights, ingredient quantity and action-graph envelope.
- `fixtures/recipe-task-graph-v1.json`: synthetic positive and adversarial fixtures.
- `src/recipe_graph.rs`: Rust types, validator and mapper to KitchenPlanV1.

## Important boundaries

- Original recipe text is not embedded. Store a source pointer, digest and attribution/permission references. Respect the rights status; `unknown` or `restricted` sources are local-processing only. Redistribution or model-training uses require explicit permission evidence. Attribution alone is not permission.
- Ingredient quantities use an integer coefficient, decimal scale and explicit unit. No implicit gram-to-millilitre conversion occurs without a separate density/method record.
- The mapper copies KitchenTaskV1 fields (task IDs, duration, resources, dependencies and timing bounds) exactly. It does not generate missing timing or resource assumptions.
- A graph rejected by human review cannot be mapped. A reviewed graph is still not a food-safety or culinary-quality validation.
- The planner is advisory. It does not actuate appliances or override local safety systems.
- Evidence class remains declared/modelled/simulated; human review does not promote modelled evidence into measured evidence.
- A structurally valid graph can still fail runtime validation for unknown ingredient references, duplicate IDs, invalid schedule DAGs or rights-scope mismatch.

Run through the existing planner test target:
`cargo test -p symtropy-kitchen-planner --all-targets`

The branch layers on the power-aware planner PR: https://github.com/Luminous-Dynamics/symtropy/pull/2230
