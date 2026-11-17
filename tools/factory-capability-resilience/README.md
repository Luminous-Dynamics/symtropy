# Factory Capability Resilience v0.1

**Owner:** Symtropy FACTORY-CAPABILITY-RESILIENCE-001 [#5704](https://github.com/Luminous-Dynamics/symtropy/issues/5704)  
**Parent programme:** evidence-gated industrial intelligence  
**Fixture generation:** `pump-valve-topology-v0.1`  
**Observation epoch:** `2026-11-15T12:00:00Z`  
**Implementation status:** frozen synthetic corpus and dependency-free Rust evaluator authored; eight unit tests and a deterministic TSV replay are defined; exact-head CI qualification is pending.

## Research question

How many alternative routes remain **independently qualified** after a shared dependency, authority, utility, material, calibration, software, inspection, or partner failure?

The primary outcome for later dynamic extensions is time to first independently accepted output. v0.1 deliberately starts with a small, deterministic route-admissibility fixture so its logic can be checked by hand before the evaluator and larger factory routes are implemented.

## Paradigm shift

Machine count is not production resilience. A candidate route is an AND of mandatory prerequisites; a product operation can have OR alternatives among candidate routes. Two different machines do not imply two independent capabilities when they share a feeder, calibration source, material origin, evidence origin, authorization root, or transport corridor.

This fixture contains five nominal paths:

- `local_a` and `local_b` are distinct local machines but share five declared upstream fault domains: power, metrology, material origin, inspection facility, and measurement-evidence origin.
- `partner_c` and `partner_d` are distinct partner routes but share one partner authorization root.
- `independent_e` is an explicit positive-control alternative with no declared shared fault-domain identity with the other routes.

Consequently, five nominal paths provide only three simultaneously pairwise fault-domain-disjoint paths in the baseline fixture: one local path, one partner path, and the independent path. This is an explicit property of the synthetic topology, not an estimate of real-world reliability.

## Files

- `topology-v0.1.tsv`: the 43 mandatory dependency rows for the five candidate paths.
- `routes-v0.1.tsv`: expected mandatory dependency count and route class for each candidate path.
- `scenarios-v0.1.tsv`: 14 deterministic scenario rows, including shared-root failures, a single qualification revocation, an unknown dependency remainder, a compound failure, a no-route case, a failed first-article gate, and paired hidden-future controls.
- `policy-arms-v0.1.tsv`: baseline and oracle definitions.
- `Cargo.toml`, `src/lib.rs`, `src/main.rs`: dependency-free route evaluator, interval calculation, paired-future fingerprint check, and TSV CLI.
- `.github/workflows/factory-capability-resilience.yml`: focused Rust formatting, Clippy, unit-test and deterministic replay gate.

All IDs and values are synthetic. `fault_domain_id` is an owner-declared common-cause identity, not something inferred from a different or matching label. Distinct IDs alone do not establish physical independence. The future evaluator must consume dependency/fault-domain projections owned by Mycelix (#5762/#5696 and #1243); it must not create a competing generic graph.

## v0.1 semantics

For each candidate path:

1. All rows for the path are mandatory AND prerequisites.
2. A path is `AVAILABLE` only when its dependency inventory is complete, every referenced qualification is current and applicable at the decision epoch, and none of its fault domains is failed in the scenario.
3. A path with any declared unknown dependency remainder is `UNKNOWN`, not available and not silently dropped from the inventory.
4. A path with a failed fault domain or revoked qualification is `BLOCKED`.
5. A synthetic accepted output is possible iff at least one complete, current path remains available and the scenario's separate first-article/independent-inspection gate is `PASS`. A feasible route is not an accepted product.
6. Report an **independent-path interval**, not an unjustified point estimate, when route inventory is incomplete. The lower bound is the largest pairwise-disjoint subset of fully known `AVAILABLE` paths. The optimistic upper bound also includes `UNKNOWN` paths, assuming they are available and their unknown remainder introduces no additional conflicts. The upper bound is therefore optimistic; the lower bound is conservative. If both bounds are equal, the unknown path does not change this particular count, but it remains `UNKNOWN` and must not be admitted.
7. With a complete inventory, lower and upper bounds coincide and equal the maximum subset of available routes whose declared fault-domain sets are pairwise disjoint. It is computed by exhaustive subset enumeration on this tiny fixture. It is not a failure probability, reliability estimate, or generalized max-flow result.

For an available path `p` under scenario `s`:

`available(p,s) = complete(p) AND current_qualifications(p) AND (fault_domains(p) ∩ failed_domains(s) = ∅)`

The pairwise independence interval is:

- `lower = max_disjoint(AVAILABLE paths)`
- `upper = max_disjoint(AVAILABLE paths ∪ UNKNOWN paths with only their known dependency identities)`

The interval expresses uncertainty in capability inventory; it does not represent a probability interval. This conservative finite model intentionally avoids probability assumptions. A later implementation may support richer AND/OR hypergraph and fault semantics only when the upstream owner provides a complete, versioned projection and an independently checked interpretation.

## Expected hand-checkable results

| Scenario | Available paths | Maximum independent paths | Expected disposition |
|---|---|---:|---|
| S00 baseline | local_a, local_b, partner_c, partner_d, independent_e | 3 | synthetic route available |
| S01 local power failure | partner_c, partner_d, independent_e | 2 | local redundancy lost |
| S02 local metrology failure | partner_c, partner_d, independent_e | 2 | local redundancy lost |
| S03 local material-origin failure | partner_c, partner_d, independent_e | 2 | local redundancy lost |
| S04 local evidence-origin failure | partner_c, partner_d, independent_e | 2 | local evidence not independent |
| S05 partner authority revoked | local_a, local_b, independent_e | 2 | both partner routes blocked |
| S06 partner C transport failure | local_a, local_b, partner_d, independent_e | 3 | one partner alternative remains |
| S07 local A qualification revoked | local_b, partner_c, partner_d, independent_e | 3 | do not over-invalidate local B |
| S08 partner D dependency inventory incomplete | local_a, local_b, partner_c, independent_e; partner_d unknown | [3, 3] | unknown path is not admitted; upper bound uses optimistic known-domain-only remainder |
| S09 local + partner common cause | independent_e | 1 | independent positive control survives |
| S10 local + partner + independent power failures | none | 0 | no admissible route |
| S11/S12 paired hidden futures | different post-event path sets | [2, 2] each | decision before event must be identical |
| S13 first-article inspection rejected | all five paths available | [3, 3] | route exists but no product is accepted |

The machine-readable TSV is authoritative for scenario IDs, injected events, acceptance-gate state, expected route dispositions, lower/upper independence bounds and expected output disposition. The table is explanatory.

Run the focused evaluator with:

\`\`\`sh
cargo fmt --manifest-path tools/factory-capability-resilience/Cargo.toml -- --check
cargo clippy --manifest-path tools/factory-capability-resilience/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/factory-capability-resilience/Cargo.toml --all-targets
cargo run --quiet --manifest-path tools/factory-capability-resilience/Cargo.toml --bin factory-capability-resilience
\`\`\`

## Required evaluator behavior

- Reject unknown, stale, revoked, incomplete, or mismatched qualifications.
- Treat an absent dependency record as unknown/incomplete, not as evidence of independence.
- Preserve multi-valued dependency identities and explicit common-root overlap.
- When inventory is incomplete, emit lower/upper independent-path bounds; never promote the optimistic upper bound to a verified count.
- An unknown path must remain excluded from the admissible set, even when the lower and upper independence bounds happen to coincide.
- Distinguish `AVAILABLE`, `BLOCKED`, and `UNKNOWN`; do not collapse them into a boolean.
- For paired hidden-future scenarios S11/S12, require the same pre-event action/decision digest because the observable prefix is identical and the future event is not visible.
- Keep product inspection/acceptance and genealogy as separate hard gates. S13 proves that a set of valid routes can still produce no accepted output when first-article inspection fails.
- Never infer a probability from the 13 scenario rows.

## Baselines

See `policy-arms-v0.1.tsv`. The required progression is:
- count-based nominal redundancy;
- route diversity without qualification;
- qualification-aware route admission;
- dependency/fault-domain-aware path selection;
- adaptive substitution/requalification;
- exact tiny oracle;
- full reconfiguration;
- evaluator-only hindsight reference.

All deployable arms must receive identical observable evidence, decision epochs, budgets, and route requirements. Hindsight information is not an equal-information competitor.

## Metrics and hard gates

Report separately:

- nominal path count, available qualified path count, and lower/upper pairwise-independent path bounds;
- route survival by scenario and common-cause detection;
- false qualified-path admission and false independence count;
- unknown/incomplete inventory count;
- exact-oracle agreement;
- requalification actions and affected operation/resource footprint;
- time to first qualified route and, in dynamic multi-stage extensions, time to first independently accepted output;
- WIP/genealogy, inspection, and blocked-output disposition;
- runtime, memory, solver calls, and explicit `INCONCLUSIVE` outcomes.

Hard gates: zero invalid route admissions; zero false independence claims for declared shared roots; no incomplete inventory treated as complete; a failed or unknown first-article gate cannot yield `ACCEPTED`; exact agreement with the exhaustive oracle on all supported tiny fixtures; 100% non-anticipativity on the paired hidden-future controls; explicit disposition for every blocked or unknown path.

Performance targets are hypotheses only. Establish a baseline before preregistering percentage improvements. Do not combine correctness gates and performance metrics into one weighted score.

## Scope and claim boundary

This is a synthetic single-operation capability-route fixture, not a full production scheduler, multi-order capacity model, physical factory twin, safety case, or commissioning record. The bounded `ACCEPTED` label in this fixture is synthetic only and depends on the scenario's explicit first-article gate. It does not prove actual fault independence, failure rates, supplier reliability, throughput, production savings, product quality, or physical acceptance. The `independent_e` path is independent only under the declared synthetic fault-domain inventory. An identifier, hash, signature, or digital twin does not prove that a physical measurement occurred or that a conclusion is true.

## Reuse and ownership

- Symtropy #5432: matched pump/valve and repair route requirements and architecture profiles.
- Symtropy #5411: reconfiguration chronology through first accepted output.
- Symtropy #5520: selective repair and decision-valued inspection.
- Symtropy #5622: context ablation and grounding.
- Mycelix #5695/#5762: bounded assurance cut sets and multi-valued/transitive dependency preservation.
- Mycelix #1243: canonical evidence dependency/source-derivation relations.

Do not copy those systems into this crate. Consume their owner-issued, versioned interfaces when available.

## External measurement context

NIST's Super Intelligence for Manufacturing programme identifies human decision ownership, contextualized industrial data, integration benchmarks, and digital-twin VVUQ as parts of a trustworthy manufacturing intelligence architecture: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing

ISO 23247-6:2026 describes integrated, unified, and federated digital-twin composition: https://www.iso.org/standard/87426.html

These references motivate interoperability and independent evidence boundaries; neither establishes that this synthetic model is industrially validated.
