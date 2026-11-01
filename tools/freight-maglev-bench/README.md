# symtropy-freight-maglev-bench

Standalone, dependency-free evaluator for a frozen synthetic port-to-hinterland freight corridor corpus.

This tool does not optimize routes, select sites, run geotechnical/railway/maglev physics, or establish commercial feasibility. It evaluates one declared alternative against one declared baseline under transparent synthetic arithmetic.

## Corpus format

The pipe-delimited file has 23 columns:

1. case ID
2. candidate mode family
3. cargo class
4. demand tonnes/day
5. candidate capacity tonnes/day
6. baseline capacity tonnes/day
7. candidate annualized fixed cost (integer minor units/year)
8. baseline annualized fixed cost (integer minor units/year)
9. candidate variable cost (integer minor units/tonne)
10. baseline variable cost (integer minor units/tonne)
11. candidate end-to-end transit minutes
12. baseline end-to-end transit minutes
13. candidate energy (Wh/tonne)
14. baseline energy (Wh/tonne)
15. candidate emissions (gCO2e/tonne)
16. baseline emissions (gCO2e/tonne)
17. rights gate
18. geotechnical gate
19. fire/egress gate
20. environmental gate
21. cargo compatibility gate
22. deterministic fault capacity remaining (%)
23. expected disposition

Gate values are pass, fail or unknown. A failed gate makes a candidate ineligible; an unknown gate yields needs-evidence; only after gates pass is capacity assessed. Cost cannot compensate for a failed or unknown hard gate.

All values in v0.1 are synthetic. Cost units are deliberately abstract, not South African rand, Swiss francs or any procurement currency. Transit time is end-to-end in the candidate service boundary, not maximum vehicle speed. Fault percentages are deterministic stress inputs, not inferred failure probabilities.

## Run

    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
