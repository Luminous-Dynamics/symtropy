# Machine Capability Compiler v0.1

**Status:** architecture/specification and synthetic known-answer corpus. No production route selector or physical manufacturing route is qualified.
**Owner:** [MACHINE-CAPABILITY-COMPILER-001 #6786](https://github.com/Luminous-Dynamics/symtropy/issues/6786)
**Parent:** [PRODUCTIVE-MACHINE-PLATFORM-001 #6762](https://github.com/Luminous-Dynamics/symtropy/issues/6762)

## 1. Problem

A person should not have to choose a machine from a catalogue and then figure out whether it can safely manufacture a particular thing. The planning question is the inverse:

> Given an exact product/repair task, rights, part revision, failure consequence, operating environment, demand, deadline and available capabilities, which manufacturing or supply routes are admissible, evidence-supported and economical?

The compiler turns a task into candidate routes, checks the complete prerequisites, reports blockers and unknowns, and compares only routes that satisfy the hard constraints.

It does **not** generate arbitrary G-code, control physical machines, grant manufacturing rights, qualify a process, approve a safety case, replace a qualified engineer, or create another MES/inventory/dispatch/provenance authority.

## 2. Paradigm shift — intent to qualified production

The conceptual pipeline is:

`Intent → Rights & product authority → Process route → Machine/facility/skill capability → Inspection → Repair verification → Cost and lead-time comparison`

Each stage is a hard dependency. A later stage cannot compensate for a missing earlier authorization. A cheap or fast route cannot win if the part rights, exact revision, hazard class, material, process envelope or acceptance evidence are unknown or invalid.

### Input dimensions

- Exact output function, interfaces and part/product revision.
- Product-specific part-risk class and allowed production tiers.
- Design/manufacturing rights and redistribution terms.
- Material/feedstock identity, lot, conditioning and exposure requirements.
- Process family, machine envelope, tool/fixture, firmware and recipe.
- Site/facility, operator competence, utilities and safe operating conditions.
- Capacity, queue, setup, processing, inspection, transport and deadline.
- Measurement method, instrument calibration, uncertainty and acceptance criteria.
- Installation competence and post-repair verification.
- Complete route economics and the supply dependencies behind the machine itself.

### Output dispositions

- `Feasible`: at least one route satisfies every required hard constraint with current evidence.
- `Infeasible`: every candidate route is blocked by at least one known hard constraint.
- `Unknown`: no route is currently feasible and at least one could only be resolved by missing/stale/ambiguous evidence.
- `Unsupported`: the request requires semantics, process types or decision authority outside the implemented model.

Do not collapse these into a boolean. In particular, unknown is not a weak form of feasible.

## 3. Route evaluation semantics

A production route contains an AND-set of prerequisites. Candidate routes form OR alternatives.

A route can be accepted only if:
1. Rights are verified and not revoked.
2. Exact product revision and effectivity match.
3. Part risk is permitted at the route's tier.
4. Process family and machine envelope are compatible.
5. Machine identity/configuration is current.
6. Safety controls and operating domain are qualified for the route.
7. Material/feedstock is approved and traceable.
8. Operator competence is valid.
9. Required calibration is current.
10. Capacity meets the quantity and deadline.
11. Required inspection capability exists and is current.
12. Required installation/post-repair verification is present.

Known hard failures block that route. Unknown evidence holds the route in an unknown disposition unless a separate known hard blocker already makes it infeasible. A second OR route may still be feasible even when the first route is infeasible.

A route selector may rank only feasible routes. It must never optimize around a P3 restriction, missing rights, incompatible revision, failed safety requirement or missing required inspection.

## 4. Evidence classes and freshness

Every claim should retain its evidence class and source:
- `assumed`
- `synthetic`
- `vendor_declared`
- `measured`
- `independently_verified`
- `qualified`
- `revoked`
- `unknown`

A schema-valid record is not necessarily true. A machine self-report does not independently establish machine condition. A file digest proves byte identity, not rights, part quality or process qualification.

Evidence freshness is requirement-specific. Calibration expiry, firmware changes, tool wear, material lot changes, facility changes, operator competence expiry, product revision changes and safety modifications can invalidate different parts of a prior qualification. No global TTL should silently replace an owner-native revocation or expiry contract.

## 5. Machine dependency closure

Open CAD is not the same as local manufacturing capability. Each machine's critical BOM and operating requirements should expand into:
- material stock and structural components;
- motion parts, bearings, belts, fasteners and drive components;
- controllers, sensors, wiring, power conversion and firmware;
- tools, fixtures, metrology and calibration;
- energy, extraction/ventilation and environmental controls;
- operator skills, maintenance and safety verification;
- manufacturing processes needed to make, repair or replace those dependencies.

For each component/process dependency, report:
- locally manufactured;
- locally serviced or assembled from imported inputs;
- imported but locally stocked;
- imported with long or unknown lead time;
- unknown.

These states are not synonyms. A local distributor is not a local manufacturer. Local assembly does not imply local manufacture. Imported equipment with local repair may be maintainable but not reproducible.

The evaluator now constructs the **transitive prerequisite closure rooted at every critical machine dependency**. Each `requires` edge is an AND prerequisite: a calibration fixture, service tool, metrology reference, upstream machine, or other child dependency participates in the closure even if its own inventory label is `important` or `noncritical`. Classification is not limited to the first BOM tier.

Only dependencies reachable from critical roots influence the required closure; unrelated spare inventory is reported separately and does not make a machine appear less reproducible. Missing references, duplicate identities, or any cycle in the declared dependency graph return Unknown rather than proving closure.

The evaluator reports both direct-critical metrics and full required-closure metrics:
- direct critical item count and no-alternate / not-repairable counts;
- transitive required dependency count and no-alternate / not-repairable counts;
- closure disposition using the full transitive set.

Report separate dimensions:
- share of critical BOM roots with verified local manufacturing route;
- share of their transitive prerequisites with verified local manufacturing route;
- share with verified local service/repair route;
- direct and transitive dependency nodes without alternate sources;
- imported/unknown critical exposure and replacement lead-time distribution;
- skills/process/tool dependencies that prevent independent reproduction.

Do not produce a single self-sufficiency scalar. Do not allow a cyclic dependency graph to prove its own closure. The generic AND/OR closure theorem remains with Symthaea CIV-BOOT #5774/#5781; this project supplies a bounded machine-subject evaluator and evidence fixtures, not a competing general theorem.

**Adversarial reference:** MCC-022 has a locally serviced printer whose critical frame requires a locally manufactured calibration fixture; that fixture in turn requires a noncritical-labelled imported long-lead calibration reference. The overall route can still be feasible, but the machine is classified `import_dependent`. A separate test confirms that an unreferenced imported spare does not affect the required closure. The CLI exposes direct-critical counts and transitive required-closure counts separately.

## 6. Whole-route economics

Rank only routes that pass every hard requirement. The current v0.1 evaluator calculates synthetic cost per accepted output and a delivery lead-time measure, then reports the Pareto frontier across those two objectives. It deliberately does not collapse them into a weighted score.

`C_accepted = (capital allocation + material + energy + labour + setup + calibration + inspection + finishing + transport + maintenance + failed attempts + rework + warranty reserve) / accepted outputs`

Every cost category is required in the input, but an unmeasured category is represented as JSON null, never as zero. Physical route feasibility is assessed independently of economic completeness: a route can remain feasible while its cost is unknown. In that case the evaluator returns no cost-per-accepted-output value and excludes the route from the economic Pareto frontier until the missing category is evidenced. This prevents an incomplete route from appearing artificially cheap.

The Pareto frontier retains economically evaluable feasible routes for which no other such route is both no more expensive and no slower, with at least one strict improvement. A low-cost route that is slower may remain alongside a faster but more expensive route. Infeasible, unknown and unsupported routes are excluded from the frontier regardless of cost.

Also report:
- request-to-part P50/P90;
- queue, fabrication, inspection and transport time separately;
- expected and observed yield with denominator;
- scrap and rework;
- energy/material per accepted output;
- spare/alternate supplier risk;
- uncertainty and missing cost categories.

Do not compare an OEM quoted part price against a local print's filament cost. Missing cost components remain unknown, not zero. The current corpus uses synthetic cost inputs; no route price ranking is a measured procurement recommendation.

## 7. External standards and benchmark position

The compiler is an evidence/decision layer. It should interoperate with industrial equipment and digital-thread standards rather than replace them.

### Industry and SOTA comparison

- **NIST AM-Bench** is a strong reference for controlled additive-manufacturing measurements and blind model challenges. Its 2025 programme covers eight metals-AM measurement/challenge sets and one polymer vat-photopolymerization set; its 2022 polymer programme included material extrusion of polycarbonate. NIST describes a higher-TRL industrial-component focus for AM-Bench 2028. This compiler is not a substitute for AM-Bench and must not claim to outperform its metal-process physics challenges. The complementary Luminous target is independently reproducible desktop MEX geometry/capability, route-feasibility, repairability and cost per conforming output. See [NIST AM-Bench](https://www.nist.gov/ambench).
- **ISO 23247-1:2021** gives the manufacturing digital-twin framework; **ISO 23247-5:2026** addresses the digital thread; **ISO 23247-6:2026** addresses federated/integrated/unified digital-twin composition. The compiler should consume and produce versioned owner-native references at these boundaries, not create one universal twin. See [Part 1](https://www.iso.org/standard/75066.html), [Part 5](https://www.iso.org/standard/87425.html), and [Part 6](https://www.iso.org/standard/87426.html).
- **MTConnect / OPC UA** already provide manufacturing equipment information-model interoperability. The compiler must not invent another telemetry or control protocol. It consumes signed/attested owner-specific observation and profile references through adapters; observation interfaces do not themselves authorize actuation. See [MTConnect documentation](https://mtconnect.squarespace.com/documentation) and the [OPC Foundation MTConnect collaboration](https://opcfoundation.org/markets-collaboration/mtconnect/).

### Compose benchmark owners; do not duplicate them

| Benchmark / evidence question | Owner | Compiler relationship |
|---|---|---|
| FDM dimensional/feature quality and held-out geometric measurement | Symthaea #7960/#7961 | Consume the exact artifact, machine/process profile, metrology and feature-level outcome |
| Spatial build-volume and location-dependent capability | Symthaea #7998 | Treat capability as position/orientation/configuration-specific; never collapse into one printer-wide accuracy scalar |
| Production-value and fair conventional comparison | Symthaea MFG-VALUE-001 #7608 | Use cost per conforming part, first-pass yield, critical quality guardrails, uncertainty and paired campaign/lot comparisons |
| Canonical process/recipe/capability semantics | Symthaea MFG-PROC-000 #5686 | Consume canonical process definitions; do not redefine them in this compiler |
| Typed process references and resource offers | Mycelix #6531/#6533 | Consume exact owner references and offer states only after schema/version and external-object resolution |
| Route feasibility / dependency closure / Pareto analysis | Symtropy #6786 | This reference compiler implementation; it remains synthetic until independent review and physical evidence |

### Benchmark ladder and target metrics

Do not chase one aggregate "smart factory" number. Report the dimensions independently:

1. **Semantic and evidence conformance:** every known-answer synthetic case has the expected disposition; every malformed, stale, revoked, rights-invalid or missing hard prerequisite remains blocked/unknown; every exact input produces a deterministic replay receipt.
2. **Route feasibility:** 0 false-feasible decisions in frozen rights/revision/safety/material/calibration/operator/inspection negative controls. Count false-unknown and false-infeasible separately.
3. **Geometry and quality:** report feature-level dimensional error by position/orientation, uncertainty and instrument profile; no aggregate average may hide a local out-of-tolerance feature.
4. **Production economics:** use MFG-VALUE #7608 matched workloads and strongest conventional baselines; primary candidate objective is fully burdened cost per conforming, serviceable part. Its proposed Stage-2 target is at least 10% improvement at matched hard quality-escape risk, with a paired lot/campaign-level 95% interval excluding zero improvement and no guardrail regression. This is a target, not a present result.
5. **Repair and recovery:** time-to-diagnose, time-to-restore, first-time repair success, required tooling/skills, repeat failure and post-repair qualification.
6. **Productive closure:** count transitive critical dependencies that are independently serviceable, repairable and reproducible; expose single-source critical dependencies and replacement-generation needs. Do not call a machine locally reproducible merely because its frame can be fabricated.
7. **Federated capacity:** keep declared offer, owner-object resolution, qualified capability, calendar availability, bookable capacity, reservation and executed output as separate measured states.

Use preregistered policies, frozen holdouts, exact input/output digests and independent evaluators. Do not allow a gain in speed, machine utilization or nominal output to compensate for quality/safety failure. Compare the baseline and candidate on identical work, machine configuration, staffing, material lot, inspection method, time horizon and cost accounting. If sample size or independent lot count is insufficient, report Inconclusive/NotEvaluable.

## 8. Owner boundaries

- Symtropy #6786: deterministic reference assessment, synthetic corpus, route diagnostics and benchmark receipts.
- Symthaea CIV-BOOT #5774/#5781: generic productive capability closure semantics.
- Symthaea MFG-PROC-000 #5686: process/material/capability/recipe qualification semantics.
- Symtropy #6735 / #6603: part admission and appliance-specific manufacturing routes.
- Mycelix: canonical identity, rights/commitment, material/lot, custody, inspection and production/repair provenance.
- Human/competent authorities: product design authority, hazard analysis, machine safety and statutory conformity.

This specification must adapt to owner-native contracts instead of creating duplicate authorities.

## 9. Frozen v0.1 synthetic corpus

The corpus is deliberately synthetic. It contains admissible, infeasible and unknown routes, OR alternatives, capacity contention, local/import distinction, cyclic dependency attempts and missing post-repair checks. Its expected outputs are reference labels for independent review, not physical evidence.

The schema is `schemas/manufacturing/machine_capability_case_v0_1.schema.json`.
The fixtures are `fixtures/manufacturing/machine_capability_cases_v0_1.json`.

## 10. Implementation sequence

1. Validate the schema and freeze reviewed known answers.
2. Implement a small isolated Rust reference evaluator with strict enum/unit validation and deterministic normalized outputs.
3. Add mutation tests: drop a safety qualification, revoke rights, change product revision, expire calibration, substitute material, create a dependency cycle, alter units, duplicate IDs and reorder semantically unordered data.
4. Independently review expected labels; the oracle must not reuse the candidate evaluator.
5. Add a result receipt binding exact evaluator commit, schema, corpus and inputs.
6. Only after synthetic tests pass, connect one exact P0 part and one real N1 capability using measured evidence.

## 11. Claim ceiling

A passing synthetic evaluator establishes only that the implementation agrees with the reviewed synthetic contract. It does not qualify a printer, part, material, operator, facility, repair or production route. It does not establish lower cost, shorter restoration time, local manufacture or safety. Those require physical measurements, rights confirmation, competent review and current supplier/qualification evidence.
