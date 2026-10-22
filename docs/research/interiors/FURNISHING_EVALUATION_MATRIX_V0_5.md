# Furnishing co-design: cross-industry evaluation matrix v0.5

**Status:** proposed benchmark protocol. Targets here are acceptance goals, not measured results. The geometry oracle remains a synthetic axis-aligned 2D checker; this document does not certify accessibility, building code, physical safety or human outcomes.

## 1. Paradigm shift: test the whole claim chain, not one layout score

A single collision-free score is inadequate. A layout may have no furniture-footprint intersections while blocking a drawer, failing a resident's explicit request, leaving no usable approach, relying on unknown dimensions, or presenting a stale marketplace offer as current. Conversely, a realistic-looking scene can still violate the specification.

Evaluate separate layers with separate oracles and report their vector of results rather than collapsing them into one hidden weighted score:

1. **Intent fidelity:** required objects, counts, properties, spatial relations, resident-selected priorities and explicit exclusions.
2. **Geometric truth:** collisions, out-of-room footprints, task-specific envelopes, boundary/tolerance cases and deterministic output.
3. **Human task feasibility:** access, operation, turning, transfer, reach and circulation models whose inputs and criteria have been sourced and qualified.
4. **Evidence and uncertainty:** whether asserted dimensions, model identity, repair claim, geometry license, offer price or availability is traceable, scoped and current enough for the claim being made.
5. **Decision quality:** whether the system exposes feasible alternatives and real trade-offs without letting a universal score silently override the resident.
6. **End-to-end utility:** time to a feasible candidate, number of edits, usability, installation surprises, post-occupancy task success and user-reported fit. These require user studies or field data, not synthetic geometry alone.

Each layer gets an independent result and evidence receipt. Unknown mandatory evidence must remain indeterminate or blocked. Performance improvement never offsets a false-safe result.

## 2. What current research and standards contribute

### Scene generation and evaluation

[SceneEval (WACV 2026)](https://3dlg-hcvc.github.io/SceneEval/) explicitly evaluates textual requirement fidelity—such as object counts, attributes and spatial relations—alongside implicit plausibility such as collisions and navigability. Its released SceneEval-500 covers 500 descriptions over ten room types with easy, medium and hard descriptions. This is a useful evaluation model to adopt: **requirements fidelity and physical plausibility must remain separately observable**. It is not a direct runtime baseline for our rectangle checker, because SceneEval evaluates generated 3D scenes against annotated descriptions and properties.

[3D-FRONT (ICCV 2021)](https://openaccess.thecvf.com/content/ICCV2021/html/Fu_3D-FRONT_3D_Furnished_Rooms_With_layOuts_and_semaNTics_ICCV_2021_paper.html) provides a large synthetic furnished-room dataset. Its paper reports 18,968 rooms, 6,813 houses and 13,151 furniture objects. It can help build a realistic distribution of room and object sizes, categories and placements if dataset terms permit the intended use. It is not, by itself, ground truth for drawer clearance, accessibility or the correctness of a particular task envelope.

**Action:** define a local furnishing-evaluation corpus that links annotated intent, normalized product identity, dimension evidence, room geometry, expected hard-constraint outcomes and uncertainty labels. Keep source dataset, license, transformation script and digest with every derived case.

### BIM and product-information interoperability

[IFC 4.3.2.0 / ISO 16739-1:2024](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html) is the official IFC 4.3 publication listed by buildingSMART. Reuse IFC object concepts and established property-set semantics where they fit, rather than creating a parallel proprietary BIM vocabulary.

[buildingSMART IDS 1.0](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/page/4/) was approved as a final standard on 1 June 2024. IDS is useful for specifying and validating required IFC entities, classifications, properties, quantities, materials and values. buildingSMART explicitly notes that IDS does not describe geometry itself. Therefore, **use IDS for information requirements; use a separate qualified geometry/evidence contract for spatial proofs**. Neither an IFC classification nor a complete property set proves that a real item matches its model, that a clearance is usable, or that marketplace stock is current.

### Accessibility and task models

[ISO 21542:2021](https://www.iso.org/standard/71860.html) covers accessibility and usability aspects of the built environment, including access, circulation, egress and evacuation. Its published summary references wheelchair footprints in the context of related ISO wheelchair standards. Treat the exact edition, applicable jurisdiction, building type, users, measurement assumptions and clauses as explicit inputs to a rule pack; do not turn one nominal wheelchair rectangle into a complete accessibility model.

Door swings, drawers, pull-outs, reach, turning, transfers, accessible approach, egress, caregiver access and maintenance are different task models. Each needs a source, version, scope, geometry transformation, uncertainty bounds, test fixtures and a human/professional review path. The current 2D rectangle oracle is not an accessibility or code-compliance engine.

## 3. Benchmark suite

### A. Correctness and fail-closed behavior — hard gates

| Metric | Proposed gate | Evidence required |
|---|---:|---|
| Missed physical-overlap pairs | 0 | Exact comparison with an independent brute-force reference on frozen synthetic cases |
| Extra physical-overlap pairs | 0 | Same independent comparison; identical, explicit tolerance policy |
| False-safe outcomes in known-truth and adversarial cases | 0 | Mutations for overlaps, boundary contact, near-epsilon gaps, task-envelope intrusion, out-of-room cases and invalid inputs |
| Required unknown/malformed evidence promoted to pass | 0 | Mutation tests proving unknown, stale, missing, unsupported-rotation and contradictory evidence remains blocked |
| Output determinism | Exact | Same content and stable diagnostic order for repeated identical inputs and reordered source links |
| Revision/evidence identity loss | 0 | Every projected product, layout and offer claim preserves its exact source reference and version |
| Unauthorized effect during discovery | 0 | Static capability boundary plus contract tests showing discovery cannot place orders, reserve stock, contact sellers or send floor plans |

A high collision-recall figure is not enough; one false-safe in the declared known-truth corpus blocks qualification. Precision/recall may also be published for fuzzed or real-world annotated cases, but must not replace exact gates on deterministic cases whose truth is known.

### B. Performance and scaling — separate by workload density

Run the v0.2 brute-force implementation, the v0.3 BVH/indexed implementation, and at least one independent/reference broad phase on identical inputs and a named machine.

| Workload | Sizes | Report | Proposed target |
|---|---|---|---|
| Sparse, disjoint items | 25 / 100 / 1k / 10k / later 100k | p50, p95, p99, candidate pairs, memory peak | Initial target: p95 under 100 ms at 10k for local oracle only |
| Tight functional clearance, but no footprint overlap | Same | p50/p95 plus exact task fail counts | Same provisional 10k target, with all blocked requirements preserved |
| Dense physical overlap | 25 / 100 / 1k / 10k; increase only if safe | correct pair count, pairs/s, wall time, memory peak, unexpected findings | No fixed 100 ms gate when correct output is large; zero missed/extra pairs stays mandatory |
| Boundary and precision | Fixed adversarial micro-cases and generated sweeps around predicate tolerance | Exact expected decision at each offset; no result based on rendering pixels | Zero disagreement with the published numerical policy |
| Mixed/realistic distributions | Derived, licensed cases from furnished-room data and measured/annotated rooms | Per-category latency, case coverage, memory and correctness | Set SLA only after repeated reference-machine runs |

The provisional target under 100 ms at 10k refers only to a local, preconstructed JSON snapshot on a declared reference machine. Do not combine network, DHT convergence, IFC import, asset decoding, UI rendering or solver search into the same stopwatch. Record cold and warm runs separately.

The dense workload is inherently output-sensitive. Ten thousand rectangles have 49,995,000 possible unordered pairs; if many really intersect, enumerating all true collisions cannot be promised to finish under the sparse-case SLA. Publish true-positive count and throughput so the algorithm is not rewarded for silently dropping findings.

### C. Generation and user-value tests

A production-quality benchmark should eventually contain a frozen, versioned set of annotated requests and scenes rather than synthetic grids alone:

- Diverse room types, small/irregular rooms, unusual object shapes, multiple constraints and difficult/rephrased resident requests.
- Explicit requirement annotations for counts, attributes, adjacency, orientation, relations, exclusions and resident-defined priorities.
- Independent outcomes for physical collision, task clearance, navigation, and every accessibility/egress rule actually in scope.
- Dimensions labeled by evidence state (measured, manufacturer-specified, seller-estimated, nominal, unknown) with tolerances; never treat illustrative or unknown envelopes as known truth.
- Multiple baselines: v0.2 brute force, v0.3 BVH, uniform spatial hash for suitable near-uniform scenes, a mature R-tree/BVH implementation if dependencies and licenses permit, and a documented greedy/manual layout baseline.
- A separate blinded human review for task semantics, style/usefulness and usability. Geometry unit tests cannot stand in for this study.

Report feasibility rate, false-safe rate, requirement recall, time to first feasible layout, feasible alternatives per request, human edit count, Pareto-front coverage, and post-installation discrepancy where observations can be collected with consent. Do not use mean realism alone or one weighted score as a proxy for success.

## 4. How to demonstrate superiority fairly

1. Freeze input cases, expected outputs, model versions, licenses and SHA-256 corpus digests before tuning.
2. Run baselines and candidate code on identical machines, thread settings, memory limits, inputs and timing boundaries.
3. Compare a vector of correctness and utility metrics; show confidence intervals for sampled human/field outcomes and exact counts for deterministic geometry.
4. Publish disaggregated results by room size, object count, density, category, evidence completeness, task type and unsupported/unknown inputs.
5. Include dense, degenerate, adversarial and failure cases—not only attractive success examples.
6. Keep implementation and outcome claims separate: a faster broad phase is a kernel result; fewer planning revisions or better resident outcomes require end-to-end evidence.
7. Publish failures and limitations alongside the score, and rerun after every material algorithm or policy change.

A responsible initial claim would be: “The BVH implementation preserved exact collision-pair truth on the declared synthetic suites and improved measured local runtime on the named workloads.” Do not claim “better than SOTA” until the matched baseline, realistic held-out corpus and task/user evaluation demonstrate it.

## 5. Measured kernel evidence (20 October 2026)

The matched same-host run [#39481984586](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39481984586) ran the pre-BVH v0.2 oracle and the pinned v0.3 indexed oracle sequentially on the same Ubuntu 24.04 / Rust 1.96.0 / 4-vCPU AMD EPYC 9V45 runner, with identical benchmark source blob and generated inputs. It passed the baseline tests, formatting and strict Clippy; all 12 workload rows matched expected collision counts with zero unexpected findings.

| Workload | 1,000-item p95 ratio | 10,000-item single-sample ratio | Correctness |
|---|---:|---:|---|
| Sparse grid | 67.12× | 701.20× | 0 expected / observed collisions |
| Tight task clearance | 50.36× | 499.53× | 0 expected / observed collisions |
| Dense footprint overlap | 41.93× | 457.18× | 3,811/3,811 at 1k; 39,402/39,402 at 10k |

The full measurement record lives in [baseline PR #2392](https://github.com/Luminous-Dynamics/symtropy/pull/2392), file `FURNISHING_GEOMETRY_AB_RUN_39481984586.json`. The 10,000-item tiers use one sample per implementation and the listed ratios are *single-sample latency ratios*, not p95 estimates. The result is compelling evidence of a local kernel speedup for this specific synthetic distribution, not a production SLA or general SOTA victory. Peak memory, realistic/held-out rooms, 3D geometry and human/task outcomes are still unmeasured.

## 6. Indexed-only scale extension

The indexed-only sparse probe passed [run #39483177535](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39483177535) and follow-up [run #39483408682](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39483408682). Both emitted zero findings at 10k, 25k, 50k and 100k item/task tiers. The follow-up, on a 4-vCPU Intel Xeon Platinum 8573C runner, measured 10k p95 62.204 ms (3 samples), 25k p95 143.889 ms (5 samples), 50k 274.389 ms and 100k 566.416 ms (single samples). The earlier 100k sample on a 4-vCPU AMD EPYC 7763 runner was 760.021 ms. The full receipt with exact source SHAs and machine metadata is archived at [FURNISHING_GEOMETRY_SCALE_100K_CI_RUN_39483177535.json](https://github.com/Luminous-Dynamics/symtropy/blob/a55a43a15d4647ceb8a3d16a2e95c3e2a88f894f/docs/research/interiors/evidence/FURNISHING_GEOMETRY_SCALE_100K_CI_RUN_39483177535.json).

These two 100k values are single-sample point observations—not p95 or an SLA—and they come from different CPUs. The proposed p95-under-1-second target is not yet qualified, and peak memory is unmeasured. The brute-force comparison was only run through 10k.

## 7. Near-term execution order

1. Finish v0.4 sparse/tight/dense stress matrix with independent pair-count assertions.
2. Complete the matched v0.2 brute-force run using the identical stress generator; archive exact SHAs, runner info, output counts and timings.
3. Add a uniform-grid baseline and use measured candidate density to decide whether an adaptive broad phase is justified.
4. Expand the corpus with license-audited 3D-FRONT-derived distributions and a small manually annotated set; do not label derived synthetic truth as field truth.
5. Add IFC/IDS property-validation fixtures and keep geometry-rights, dimension evidence, task envelopes and marketplace revision receipts separate.
6. Add jurisdiction-specific accessibility/task-model rule packs only with explicit source/version, scope and review.
7. Start a consentful post-installation evaluation protocol for discrepancy and task success; never send detailed household layouts to a catalogue backend merely to compute geometry.

## References (checked 20 October 2026)

- Tam et al., [SceneEval: Evaluating Semantic Coherence in Text-Conditioned 3D Indoor Scene Synthesis, WACV 2026](https://3dlg-hcvc.github.io/SceneEval/).
- Fu et al., [3D-FRONT: 3D Furnished Rooms With layOuts and semaNTics, ICCV 2021](https://openaccess.thecvf.com/content/ICCV2021/html/Fu_3D-FRONT_3D_Furnished_Rooms_With_layOuts_and_semaNTics_ICCV_2021_paper.html).
- buildingSMART, [IFC 4.3.2.0 documentation](https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/index.html).
- buildingSMART, [IDS v1.0 final-standard announcement and documentation](https://www.buildingsmart.org/information-delivery-specification-ids-v1-0-is-approved-as-a-final-standard/).
- ISO, [ISO 21542:2021 — Accessibility and usability of the built environment](https://www.iso.org/standard/71860.html).
