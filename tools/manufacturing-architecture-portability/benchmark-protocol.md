# FoundryBench: fixed-client semantic portability and grounded-decision protocol

**Status:** proposed protocol, not an executed benchmark  
**Issue:** [FOUNDRY-BENCH-001 #6613](https://github.com/Luminous-Dynamics/symtropy/issues/6613)  
**Programme:** [MFG-ARCH-PORTABILITY-001 #6579](https://github.com/Luminous-Dynamics/symtropy/issues/6579)

## 1. Research question

Does an industrial information architecture let an independently implemented client reconstruct the same manufacturing meaning and make better-grounded decisions from the same source evidence, with lower integration and lifecycle cost?

This is not a protocol popularity contest. A rich API, large knowledge graph or high language-model score does not win if it loses source identity, changes units, misattributes a quality result, fabricates calibration confidence or cannot identify the evidence supporting a decision.

Keep four scoreboards separate:
1. **Protocol/API conformance:** required service and capability behavior.
2. **Information-model compatibility:** namespace, NodeSet, address-space and type/reference compatibility.
3. **Manufacturing semantic fidelity:** source identities and relationships preserved without critical loss.
4. **Grounded task/operational value:** correct decisions, appropriate abstention, resource use and lifecycle cost.

No result in one scoreboard substitutes for another.

## 2. Frozen baseline arms

All arms receive identical raw source artifacts, task questions, observable cutoffs, independent truth and hardware limits.

| Arm | Representation | Comparator |
|---|---|---|
| Native source | MTConnect/QIF documents and source-native identities | source-grounded reference answers and parser/schema tests |
| UNS | MQTT event transport plus pinned typed payload schemas | broker/client behavior, schema and delivery controls |
| i3X | i3X 1.0 contextual API | upstream 1.0 conformance suite with `--no-writes` |
| OPC UA | Machinery/Robotics companion models and address spaces | OPC Foundation UACTT Lite for NodeSet/address-space gap analysis; full CTT for applicable service/security/core/PubSub testing |
| Hybrid | Explicit native + standards + typed context mapping | same fixed-client evaluator |

The same client logic must run against each arm. Any transport shim is separately versioned, public and measured for implementation effort. Do not give one arm a better model, prompt, tool budget, data access or human support.

### OPC UA comparator boundary

UACTT Lite is an external comparator, not a tool Foundry claims to replace. The OPC Foundation describes it as a free NodeSet-versus-server-address-space analysis checking expected node presence, references, attributes, data types, type definitions and mandatory/optional nodes. It does not test the full functional behavior of a companion specification. Full OPC UA CTT covers broader services, security, core functionality, client and PubSub interfaces.

Foundry adds reproducible NodeSet provenance, RequiredModel dependency-skew reporting, NamespaceArray/NodeId identity normalization and manufacturing-specific part/lot/operation/QIF/metrology semantics. It does not claim UACTT Lite/CTT certification.

Any endpoint experiment must be read-only, network-isolated and egress-restricted. No method invocation, write, command, reset, setpoint, PLC actuation or product-release operation is in scope.

## 3. Dataset strata — never pool unlike tasks

**Discrete machine evidence.** MTConnect Sample, Event and Condition observations; top-level Asset objects remain separate. Test device/data-item/sequence identity, source digest, schema, source/reference timestamps, units, clock mapping/uncertainty, source locator and opaque extensions.

**Quality and digital thread.** QIF inspection results linked to the exact cycle, part, lot, operation, characteristic, feature, measurement plan, instrument, specification and revision. Nearest timestamp, matching display name or shared part number alone is not a valid join. Calibration/uncertainty references require exact subject, scope, unit, validity interval and clock identity; reference presence is not authentication.

**OPC UA information models.** Exact NodeSet artifact and RequiredModels admission; namespace/model metadata separation; NamespaceArray and canonical NodeId comparison; mandatory/optional/subset/unsupported behavior distinguished.

**Continuous-process intelligence: Tennessee Eastman Process (TEP).** TEP is a fault detection/diagnosis stratum, not CNC/QIF product acceptance. NIST's SI-for-Manufacturing programme names TEP as its planned initial case and lists UNS/MQTT + knowledge graphs/type models, i3X and OPC UA as alternative architectures. The NIST benchmark environment/harness/toolkit are currently planned FY28 deliverables; Foundry must not claim to have run that future harness.

**Physical equipment reference data.** NIST's Smart Manufacturing Systems Test Bed is a candidate: CAx covers CAD/CAM/CAI/PDM/V&V; its Manufacturing Lab includes CNC milling/turning and inspection equipment, with manufacturing data collected via MTConnect. Admit external data only after exact revision, schema, rights, source digests and measurement provenance review.

Keep synthetic, recorded/replay, live-read-only, laboratory-physical and production results in separate denominators.

## 4. TEP publication-bias and leakage controls

A 2026 systematic review/meta-analysis of TEP fault detection identifies selective reporting of easy scenarios as a comparison threat; IDV 3, 9 and 15 recur as difficult cases.

Mandatory protocol:
- Report all 21 scenarios (IDV 1–21), with 3, 9 and 15 explicitly visible.
- Use simulation-run-level or chronological splits. Never random-split samples/windows from one run across train/test; no windows cross split boundaries.
- Freeze run IDs, fault onset and test labels before tuning. Choose thresholds on training/validation only.
- Separate binary fault detection from fault classification/diagnosis.
- Report every scenario's numerator, denominator, recall/detection rate, false-alarm rate on normal test data and first-detection delay.
- Give each baseline the same partition and comparable tuning budget.
- Report mean and standard deviation over at least five seeds.
- Report alarm post-processing/voting/filtering separately from the model.
- Extend with lower-severity faults, additional operating modes and longer runs without replacing the canonical 21-case result.

Metric definitions:
- Detection recall per fault = detected fault runs / all fault runs for that scenario.
- False-alarm rate = false alarm events or alarmed normal windows / the predeclared normal-operation denominator; state which.
- Detection delay = time from source fault onset to first qualifying alarm.
- Diagnosis quality is measured after detection, and states whether classes were seen in training.
- Abstention reports task coverage, selective accuracy and correctness of abstention on ambiguous/out-of-domain tasks.

No headline average without all 21 rows and the exact aggregation rule.

## 5. Semantic portability tasks and negative controls

| Task | Evidence that must survive | Negative controls |
|---|---|---|
| Observation | source digest, machine/data item, sequence, kind, unit, clocks, locator, extension digest | wrong machine; missing sequence; unproven unit conversion; altered source digest; clock swap; dropped extension |
| Asset | asset ID/type, device, source document, raw asset digest, locator | Asset coerced into stream Sample/Event/Condition |
| Quality join | exact cycle/QIF result, part/lot/operation/characteristic and source digests | wrong part/lot/operation; expired association; wrong characteristic |
| Time/metrology | reference clock, mapping digest, uncertainty, instrument/calibration/uncertainty applicability | mismatched clock; stale calibration; wrong instrument/characteristic/unit; understated uncertainty |
| OPC UA | pinned NodeSet, namespace table, RequiredModels, metadata, canonical NodeId | reordered NamespaceArray; unknown index; wrong model/date; duplicate or unresolved alias; wrong reference target |
| Grounded QA | answer plus exact evidence references and observable cutoff | plausible but ungrounded answer; stale/contradictory source; post-cutoff leakage |

Critical relationship errors are scored by accepted false joins, not hidden by average field accuracy.

## 6. Scoreboards and definitions

**Protocol/API conformance:** every applicable MUST behavior, declared optional capability, access-control and negative-write test. Skipped update/write tests are untested, not passed. i3X read-only runs use upstream `--no-writes`.

**Information-model compatibility:** NodeSet provenance, model/version/date, RequiredModel skew, mandatory/optional node coverage, reference integrity, attributes/data types/type definitions and subset honesty. Report UACTT Lite/CTT separately; do not invent equivalent certification.

**Critical semantic preservation:**
- Critical-field preservation = exact required critical source fields preserved / total required critical source fields.
- False-join rate = incorrect accepted relationships / all accepted relationships.
- Unsupported-field rate = fields neither preserved nor explicitly rejected/marked unsupported / source fields in scope.
- Evidence coverage = material answer claims with resolvable evidence / all material claims.
- Grounding precision = supported material claims / all material claims returned.
- Correct abstention = ambiguous/stale/missing/contradictory tasks correctly marked inconclusive / all such tasks.

Report numerator/denominator, backend, data stratum and evidence class.

**Grounded decision quality:** task success, fault recall/precision, false-alarm burden, delay, diagnosis quality, abstention quality, evidence coverage, confidence calibration, expected decision loss and operator escalation/override burden.

**Operational value:** p50/p95/p99 latency, throughput, CPU, peak memory, network bytes, energy if measured, availability, recovery, operator burden, integration/maintenance hours and cost per accepted correct decision.

**Physical production (only with admitted physical evidence):** first-pass yield, accepted units/hour, dimensional conformance, rework/scrap, kWh/material/human-hours per accepted unit, process capability and lifecycle cost.

## 7. Experimental controls

Freeze and hash source artifacts/rights, dataset revisions/schema bundles, partition run IDs, tasks/labels/cutoffs, independent truth, adapters/profiles, client/model/prompts, evaluator, hardware/software/network and tuning budget.

Hold out entire simulation/production runs, machine families, lots, fault families or model versions where appropriate. Use separate development, validation and blind test sets. The evaluator truth store must not be queryable by the candidate. Any post-test change creates a new benchmark revision.

For context ablations compare: raw source; source + typed schemas; source + information model; source + graph/context relations; source + temporal/causal context; complete admitted evidence bundle. Matched client, compute and tuning budget. Measure whether context improves grounded decision value, not answer length.

## 8. Proposed target envelope

These are programme targets, not achieved results:
- 100% exact provenance admission for pinned NodeSet artifacts.
- Zero silent critical identity, unit, clock, source-document or part/lot/operation/characteristic corruption.
- Zero wrong accepted critical joins in the declared corpus; ambiguous cases reject/abstain.
- 100% of canonical TEP scenarios reported; no run leakage; at least five seeds for stochastic systems.
- No promotion of metadata/NodeSet presence into functional, cybersecurity, safety, metrology or physical qualification.
- Pareto improvement over the strongest same-task baseline in at least one material operational dimension without regression in critical semantic/authority invariants.

Do not collapse results into one weighted score. Publish Pareto frontiers and failure categories. Lower latency with wrong part/lot association is not an improvement.

## 9. Claim tiers

1. **Synthetic fixture:** schema and finite known-answer tests only.
2. **Recorded replay:** independently admitted, rights-reviewed real records replayed from immutable inputs.
3. **Live read-only:** isolated endpoint interrogation with zero write/method/command calls and probe receipts.
4. **Physical lab:** measured equipment/product evidence with independent metrology/acceptance.
5. **Production:** sustained plant operation, independent safety/quality authority and lifecycle outcomes.

Name the tier on every claim. Higher tiers are never inferred from lower-tier passes.
