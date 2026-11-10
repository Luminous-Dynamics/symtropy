# Scan-to-Engineering: model readiness × validation maturity (Luminous proposal v0.1)

**Status:** Proposed benchmark rubric. Not an achieved qualification and not a claim that Luminous invented the underlying EMRL/VM framing.  
**Programme:** [#4471 Representation Continuity](https://github.com/Luminous-Dynamics/symtropy/issues/4471)

## 1. Why two axes are necessary

Li, Liu and Fan's October 2026 review of 123 Scan-to-Engineering studies distinguishes three output pathways—Scan-to-Geometry, Scan-to-BIM and Scan-to-FEM—and introduces two complementary lenses:

- Engineering Model Readiness Levels (EMRL-1 to EMRL-4): geometry-, information-, analysis- and decision-ready model capability.
- Validation Maturity (VM-0 to VM-5): evidence strength, separate from model content.

The review reports progress on geometry-, information- and selected analysis-ready workflows, while decision-ready workflows remain rare. Its central methodological implication is important: a model's content/readiness and the strength of evidence supporting its use are different properties.

Source: Li, Liu & Fan, *Scan-to-engineering: From reality capture to digital models in civil engineering*, Automation in Construction 190 (2026), 107127. DOI: https://doi.org/10.1016/j.autcon.2026.107127

This document adopts the EMRL labels and proposes a Luminous operationalization of evidence maturity. It does not reproduce the paper's full VM-0…VM-5 criteria, which must be read from the paper itself before asserting equivalence. The Luminous validation ladder below is explicitly project-defined.

## 2. Axis A — Engineering Model Readiness

A model is assigned the highest level whose required content and task-specific constraints it actually satisfies. Higher readiness does not automatically imply higher validation maturity.

| Level | Readiness | Proposed minimum evidence/content contract |
|---|---|---|
| EMRL-1 | Geometry-ready | Metric geometry supports a named measurement task (dimensions, sections, deviation, deformation). Coordinate frame, units, observed/inferred/unknown regions, measurement support and uncertainty are explicit. |
| EMRL-2 | Information-ready | Stable element identity, declared semantic classes/properties, spatial/topological relations and exchange structure are present. Measured/inferred/assumed origins and representation lineage survive export. |
| EMRL-3 | Analysis-ready | Analysis representation is explicit: idealization, mesh/element formulation, material/constitutive assumptions, loads, boundary conditions, solver/version and verification/validation evidence. Model-form uncertainty is not conflated with scan uncertainty. |
| EMRL-4 | Decision-ready | A specific operational/engineering decision is named. Requirements, consequence model, authority, uncertainty, robust acceptance conditions and response to plausible unknowns are explicit. Decision performance is validated against independent outcomes. |

These are proposed operational criteria for Luminous, grounded in the paper's four readiness categories. A model can be EMRL-1 for a dimensional survey task and not ready for structural analysis. EMRL is always task-conditional; there is no universal "EMRL of a building" without naming the task.

### Non-implications

- IFC schema validity does not establish EMRL-1 physical accuracy.
- A complete IFC object graph does not establish EMRL-2 evidence continuity.
- A valid FEM mesh does not establish EMRL-3 model validity.
- A dashboard, digital twin or AI recommendation does not establish EMRL-4 decision readiness.
- Higher LOD/LOIN or richer object count is not a proxy for higher EMRL.

## 3. Axis B — Luminous Validation Maturity (LVM)

The following is a **Luminous-defined** six-stage validation ladder inspired by the need to separate model content from evidence strength. These labels are not asserted to be the exact VM definitions in Li et al.

| Level | Evidence class | Minimum qualification |
|---|---|---|
| LVM-0 | Unvalidated | Pipeline completion, visual plausibility, self-reported confidence or schema parse only. No independent accuracy claim. |
| LVM-1 | Analytic/synthetic | Frozen known-answer fixtures, negative controls, independent small oracle, deterministic receipts and exact-head reproducibility. Establishes contract semantics only. |
| LVM-2 | Published benchmark replay | Source-faithful reproduction on a named, rights-cleared or access-authorized benchmark; exact metric definitions, thresholds, data splits, baselines and independent evaluator recorded. |
| LVM-3 | External metrology | Held-out independent survey/checkpoint evidence, calibration analysis, building/site-level holdouts and uncertainty coverage/risk evaluation. Registration controls are not reused as independent checkpoints. |
| LVM-4 | Independent interoperability/replication | At least two independent toolchains validate the declared model profile; controlled import/edit/export preserves required geometry, identity, semantics and evidence crosswalk; another team can reproduce the protocol. |
| LVM-5 | Longitudinal operational validation | Repeated physical inspections/updates over time; drift, stale claims, causal change attribution and re-verification are evaluated against physical outcomes and decision consequences in an operational setting. |

A qualification is always bound to:
- exact model, evidence, software, schema and configuration digests;
- task/requirement profile and use class;
- named dataset/building/site split and date/time window;
- independent evaluator, checkpoints and validation methods;
- limitations, unknowns and claim ceiling.

LVM is not a universal ranking of software. A model may have LVM-3 for envelope dimensions, but only LVM-1 for MEP topology or LVM-0 for structural capacity.

## 4. The score is a pair, not a composite

Report **(EMRL, LVM)** per task, building, asset class and representation revision.

| Example | Interpretation |
|---|---|
| (EMRL-4, LVM-0) | Sophisticated decision-oriented model content, but no independent evidence supporting the decision claim. Not qualified for real decisions. |
| (EMRL-1, LVM-4) | Narrow geometry model independently tested across tools; useful and trustworthy for its limited measurement purpose. |
| (EMRL-2, LVM-2) | Information-ready BIM reproduced on a named published benchmark; no external metrology or operational evidence yet. |
| (EMRL-3, LVM-3) | Analysis-ready model with independently checked geometry/material/calibration assumptions for a defined engineering task. Still not automatically safe for every decision. |

Never average EMRL and LVM. Never allow a high value on one axis to compensate for a hard failure on the other. A weighted overall score may be published only as a secondary summary and must not override provenance, identity, uncertainty, topology, safety or acceptance blockers.

## 5. Benchmark suite

Keep each track separate and report per-building outcomes.

| Track | Metrics | Principal controls |
|---|---|---|
| Geometry / Scan-to-Geometry | Surface distance median/p95/p99; completeness/correctness curves; area overlap; dimensions; section and deformation error | Exact sampling, coordinate semantics, source-faithful metric reproduction, independent checkpoints |
| Information / Scan-to-BIM | Element precision/recall; opening dimensions/IoU; topology and relation F1; IDS requirements; IFC schema validity; edit/round-trip retention | Stable identity; explicit observed/inferred state; crosswalk and independent validators |
| Analysis / Scan-to-FEM | Mesh/idealization residual; modal/response discrepancy; parameter sensitivity; solver verification; model-form uncertainty | Independent physical measurements and task-specific validation; do not infer analysis readiness from IFC |
| Epistemic / observability | Observable vs hidden area; unknown/abstention rate; calibrated uncertainty coverage; risk-coverage curves | Evaluator isolation; full-reference masks; no reference leakage |
| Temporal / lifecycle | Identity continuity; split/merge/replacement accuracy; stale receipt rejection; change-attribution accuracy; accumulated error envelopes | Revision-bound evidence and longitudinal inspections |
| Decision / operations | False-admission rate; rework/review minutes; cost/time per accepted element; maintenance outcome; value of information | Matched baselines, blind evaluation, independent acceptance and consequence-aware utility |

### Statistical design rules

1. Split by whole building/site and, where relevant, by acquisition campaign. Never split points or rooms from one building across development and evaluation.
2. Treat buildings/sites—not points—as the principal independent sampling unit. Use building-clustered confidence intervals or a hierarchical model; millions of points do not create millions of independent buildings.
3. Pre-register primary metrics, thresholds, exclusions, baselines and acceptable failure rates before the blind run.
4. Report per-building tables, macro averages, worst-case and tail behavior. Do not let a large easy building conceal a small difficult building.
5. Use paired comparisons on identical scans, coordinate frames, task definitions and acceptance profiles.
6. Keep human intervention visible: expert review minutes, manual corrections and unresolved elements are part of operational performance.
7. Publish all negative controls and failures. No benchmark score can override a hard evidence or safety gate.

## 6. 2026 competitive baselines — matched tracks, not headline comparisons

The following are relevant comparators; the numbers below are specific to each study and cannot be ranked as though they were measured on a common benchmark.

- **Ersöz & Bosché (2026), geometric matching confidence:** coverage, distance and distribution indices; 46 point-cloud/BIM pairs and 4,000+ IFC elements. Reproduce the method and visibility population before extensions. DOI: https://doi.org/10.1016/j.autcon.2025.106636
- **Jayasinghe & Brilakis (September 2026), completion leakage:** bounding-box leakage in point-cloud completion and a corrected evaluation protocol; reported study-specific EMD/CD reductions. Candidate inference must not see complete-object bounds. DOI: https://doi.org/10.1016/j.autcon.2026.107052
- **Pan et al. (August 2026), agentic Scan-to-BIM/Scan-to-Graph:** multi-agent IFC/graph generation with topology-aware connections and user-directed detail. TUMCMS reported 4.70% and 5.10% room-area mean relative deviations for two configurations. Compare element/topology/identity and expert correction effort, not room area alone. DOI: https://doi.org/10.1016/j.autcon.2026.107003
- **Trinh, Hou & Wu (November 2026), structural reconstruction from incomplete scans:** boundary correction, Heaviside slicing and Greedy Snake Slice Completion; study reports >97% area, 98% perimeter and 92% moment-of-inertia accuracy, LOA30 and selected elements approaching LOA40, with approximately 11 minutes reported. This is a structural-element workflow, not a universal building score. DOI: https://doi.org/10.1016/j.autcon.2026.107161
- **DR-GANet (October 2026):** graph-attention segmentation and knowledge-driven structural/M&E completion; reports 93.60% mIoU and 12.8 mm mean surface deviation for primary structural components in an illustrative real-world scene. Dataset available on request; require access and matched evaluation before comparison. DOI: https://doi.org/10.1016/j.cacaie.2026.100146
- **Li, Liu & Fan (October 2026), Scan-to-Engineering review:** 123 core studies; separates Scan-to-Geometry, Scan-to-BIM, Scan-to-FEM and the independent dimensions of model readiness and validation maturity. DOI: https://doi.org/10.1016/j.autcon.2026.107127

Commercial tool comparison must also be task-matched:
- Leica Cyclone 3DR: cloud/model comparison, tolerance checks, structural and pipe/steel extraction and IFC workflows.
- Autodesk ReCap / Revit: reality capture, mesh processing, IFC/Revit interoperability.
- Trimble RealWorks: registration, point-cloud engineering and model comparison.

The target is not feature-count superiority. It is lower false-admission risk, better evidence continuity and less rework at the same accepted output profile.

## 7. Proposed Luminous targets

These are goals, not achieved results.

- Reproduce published metrics and their exact evaluation populations before claiming improvement.
- Pareto-improve at least one geometry/topology/identity trade-off on untouched whole-building groups without degrading difficult-scene performance.
- Zero unsupported measured claims, zero silent uncertainty shrinkage from representation-only transforms, and zero false admissions in frozen negative controls.
- Nominal 95% geometric uncertainty intervals should demonstrate empirical coverage near 95% on independent held-out sites, with confidence intervals and calibration curves.
- 100% detection of seeded stale digest, wrong transform, identity collision, inferred-to-measured promotion, unknown-to-zero promotion and relation-loss controls.
- Demonstrate IFC/graph round-trip retention using two independent toolchains for the declared profile.
- Show improved accepted elements per expert-review minute and total capture-to-accepted-model cost at matched requirements.
- Reach EMRL-4/LVM-5 only for named decision tasks after longitudinal evidence exists; do not label an entire building "decision ready" from geometry and IFC quality alone.

## 8. Relationship to Luminous workstreams

- #1607 owns survey pose, frames, calibration and covariance. Consume its receipts.
- #3331 owns persistent physical/component identity. Consume its correspondence receipts.
- #4086 owns reconstruction, topology, causal change attribution and mutation admission.
- #4116 owns blind replay, evaluator separation and benchmark receipts.
- #4207 and #1892 own decision-sufficient acquisition and value-of-information logic.
- #4271 owns independent IFC/IDS and edit/round-trip validation.
- #4292 owns evaluator-only theoretical visibility.
- #4456 owns independent ray-query engine comparison.
- #4471 owns the representation-continuity audit across these outputs.

No duplicate generic metrology, component registry, IFC parser, VOI engine or evidence ledger is introduced here.

## Claim ceiling

The scorecard is a research rubric. A completed rubric entry is specific to an exact task, representation, use class, dataset, building population, software/toolchain versions and evidence window. It does not certify structural adequacy, building-code compliance, safety, legal authority or universal commercial superiority.
