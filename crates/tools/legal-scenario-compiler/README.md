# Legal Scenario Compiler (synthetic-only v0.1)

This isolated Rust tool explores a diagnostic-testing paradigm for Symthaea/Mycelix/Symtropy/Sol-Atlas: generate legal-reasoning test cases from a typed symbolic rule set and compute answer keys instead of relying only on manually maintained static question/answer pairs.

**It models a fictional rulebook only.** It is not a legal ontology, source of legal authority, legal advice, or a representation of real South African, U.S., EU, or other law. No real legal text is ingested.

## Research basis

OpenExempt, published in Findings of ACL 2026, represents rules from U.S. bankruptcy statutes symbolically and generates machine-computable legal-reasoning tasks on demand. Its published benchmark contains 9,765 samples across nine diagnostic suites; its experiments report performance cliffs that emerge under longer reasoning paths and obfuscating statements. Reference: https://aclanthology.org/2026.findings-acl.1328/

This prototype adopts the general methodological idea—explicit rules, generated cases, and machine-checkable labels—but uses an original fictional ruleset, not OpenExempt's dataset or code.

COLIEE 2026 similarly emphasizes retrieving relevant provisions alongside legal entailment judgments, reinforcing that answer-only accuracy is insufficient. See https://coliee.org/COLIEE2026/tasks/task3.

## What it generates

The embedded ruleset contains:
- explicit scenario-to-fact mappings;
- rules with required facts and derived conclusions;
- bounded forward chaining;
- prioritized decision rules with explicit actions, claim codes, and citations;
- fail-closed abstention if equally prioritized decision mappings prescribe incompatible outputs.

For each of 48 semantic states, it generates two wording variants, producing **96 fictional cases**. The six modeled dimensions are source-operation permission (allowed, denied, unknown), authenticated competence, current-authority availability, material-fact availability, unresolved authority conflict, and irrelevant wording variation.

It also emits **54 metamorphic pairs**:
- 48 wording-invariance pairs: equivalent wording must preserve action, claim codes, and citations;
- 6 material-change pairs: changing permission, competence, currentness, fact sufficiency, or conflict state must change the action under the fictional rules.

The generator computes each expected result through the symbolic rule engine and cross-checks the action, claim codes, and citation set against a separately written imperative oracle. Full generated result metadata is also re-evaluated when validating a corpus.

## Run

Generate a corpus with fixed seed and validate it:

\`\`\`sh
cargo run --manifest-path crates/tools/legal-scenario-compiler/Cargo.toml -- generate 42 /tmp/legal-scenarios-a.json
cargo run --manifest-path crates/tools/legal-scenario-compiler/Cargo.toml -- generate 42 /tmp/legal-scenarios-b.json
cmp /tmp/legal-scenarios-a.json /tmp/legal-scenarios-b.json
cargo run --manifest-path crates/tools/legal-scenario-compiler/Cargo.toml -- validate /tmp/legal-scenarios-a.json
cargo test --manifest-path crates/tools/legal-scenario-compiler/Cargo.toml
\`\`\`

The output receipt includes the exact corpus SHA-256, ruleset SHA-256, seed, case/pair counts, and train/dev/test case counts. Generation and validation perform no network requests.

## Reproducibility and split rules

- Same generator version + exact ruleset bytes + seed must produce byte-identical JSON.
- Different seeds may reorder the same scenario set and change the seed-bound partition assignment; they must not change the set of case IDs or expected outputs.
- All variants connected by a metamorphic pair are assigned to the same partition to prevent direct pair leakage across train/dev/test.
- The current prototype has a **single fictional rule schema and scenario family**. Its split is useful for reproducible plumbing and pair safety, but it is not a genuine held-out-rule or cross-jurisdiction generalization benchmark. Additional independently reviewed synthetic rule families are required before making those claims.

## Proposed release gates

These are targets, not results claimed for a deployed legal system:
- 100% byte reproducibility for a fixed seed, generator version, and ruleset digest.
- 100% agreement between the symbolic evaluator and independent oracle on generated cases.
- 100% satisfaction of all declared metamorphic pairs.
- Zero duplicate IDs, broken pair references, mismatched ruleset digests, corrupted expected traces, or cross-partition pairs.
- Zero silent answers in explicitly underdetermined scenarios.
- Report results by permission state, temporal/currentness state, reasoning depth, fact sufficiency, conflict state, and wording variant—never just one aggregate score.
- On future held-out rule families, report degradation as inference depth/exception complexity increases.

A passing result only demonstrates conformance to the declared fictional ruleset. It does not establish real-world legal correctness, source permissions, professional competence, fairness of an actual institution, or authorization to provide legal services.

## Next step

Extend the ruleset into two or more independently authored fictional legal domains with exceptions, amendment intervals, conflicting authority hierarchies, and longer dependency chains. Have a second implementation independently validate generated labels; keep real-law datasets out of the generator until source-specific use rights and expert review are secured.
