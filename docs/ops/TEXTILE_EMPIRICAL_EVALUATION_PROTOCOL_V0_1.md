# Textile Empirical Evaluation and Causal Evidence Protocol v0.1

Status: proposed preregistration contract. This document is a protocol, not an assertion that a factory trial has been run. Numeric targets are provisional decision thresholds to freeze before data collection.

## Paradigm shift: measure what happened, not what the optimizer predicted

A feasible schedule is not a finished garment. A plan to use stock is not a physical transfer. An item marked reusable is not evidence of another service. Simulated lower production is not proof of avoided virgin production or a life-cycle benefit.

Judge the system on an explicit ladder of claims:

1. **Plan validity:** hard gates, time windows, capabilities and resource constraints are satisfied by the plan.
2. **Execution fidelity:** the operation actually ran as assigned; deviations, downtime and overrides are logged.
3. **Physical outcome:** cut/sewn/finished/accepted/rejected quantities and fates reconcile to measured ledgers.
4. **Service outcome:** the required garment-specific service was delivered on time at the profile-specific quality threshold.
5. **Circular outcome:** repair, transfer, resale, reuse or recycling is substantiated by a downstream receipt and observed disposition—not inferred from eligibility or intent.
6. **Impact claim:** a separately specified life-cycle comparison demonstrates the avoided burden under stated substitution, transport, use-phase and end-of-life assumptions.

A result can pass one level and fail a later one. Reports must label the highest level supported by evidence.

## Study question and estimand

The primary question is whether a declared production policy, compared with a frozen baseline policy, reduces unsold newly manufactured output **without hiding lower service, quality loss, delay, or resource burden**.

For each garment profile and functional unit, report at minimum:

- **Surplus intensity:** unsold newly manufactured units at the end of a preregistered observation window per fulfilled service unit.
- **Service fill:** realized service demand fulfilled / realized service demand.
- **On-time service:** fulfilled service units delivered by the promised time / fulfilled service units.
- **Quality acceptance:** accepted new units / physically completed new units.
- **Material intensity:** measured material input per accepted-and-fulfilled functional unit, with scrap and all other fates reported.
- **Energy and water intensity:** metered energy and water per accepted-and-fulfilled functional unit, reported separately rather than as a composite green score.
- **Execution fidelity:** assigned policy followed without unplanned override / assigned cohorts.
- **Evidence coverage:** eligible records with valid source, timestamp, unit, method and verification metadata / all eligible records.

Predeclare one primary endpoint and a hierarchy of secondary endpoints. Do not choose whichever metric looks best after seeing the data. Show absolute values, deltas, relative deltas where denominators permit, uncertainty intervals, missingness and excluded-record counts.

A functional unit must represent the service actually requested. Examples include a workwear shift, performance run, commissioned event, bridal/ceremonial event, exhibition day, or conservator-defined service boundary. Do not compare a work shirt, a couture commission and a costume by raw wear count. Cultural, artistic, identity and conservation value remain explicit qualitative or multi-criteria considerations; they must not silently become a penalty for infrequent use.

## Design: estimate effects before optimizing the story

### Preferred designs

1. **Randomized blocked assignment** when orders/cohorts can safely receive different policies. Block by profile, garment complexity, order size, season/lead-time band and relevant material/process class, then randomize within blocks.
2. **Cluster crossover** where a production cell, team or time period must use one policy to avoid contamination. Randomize the order of policy periods where feasible; record wash-in periods and carryover risks.
3. **Matched observational comparison** only when randomization is impractical. Match on pre-treatment factors, preserve the matching rule, report balance and residual confounding risks, and label conclusions observational. Before/after alone is weak evidence when seasonality, supplier availability, mix or demand changes.

Use **intention-to-treat** as the main causal comparison: analyze cohorts by assigned policy even when the team overrides it. Report the as-executed view separately. Dropping difficult treatment cases can manufacture a false success.

Freeze the protocol, policy definitions, eligibility rules, observation window, exclusions, primary endpoint, safety margins and analysis plan before the first outcome is observed. Give the protocol a stable ID/version and SHA-256 digest. Preserve amendments with timestamps and reasons; never overwrite a preregistration.

### Sample size and inference

Do not choose a universal sample count for every garment type. Estimate baseline variance and cluster structure from a pilot or historical records, then preregister a power calculation for the smallest effect that justifies adoption. A proposed starting standard is at least 80% power at a two-sided 5% type-I error rate for a confirmatory primary endpoint, with cluster design effects and attrition included. If a rare-use couture, ceremonial or archive stratum cannot support that design, report estimation with uncertainty and case evidence—not a falsely precise confirmatory verdict.

Use cluster-aware confidence intervals or randomization inference appropriate to the assignment design. Report sample size at both garment/order and cluster levels. Predeclare non-inferiority margins for service and quality; a nonsignificant difference is not proof of non-inferiority.

## Evidence protocol

Every measured value should carry, directly or through a stable evidence reference:

- source system or instrument ID and source-record ID;
- event time in ISO 8601 with timezone, plus ingestion time;
- declared unit, measurement method and boundary;
- calibration/verification state and validity window for relevant meters;
- actor/role or automated process that recorded it;
- immutable file/content digest and schema/version;
- reviewer status, including pass, fail, or unknown, with a reason;
- links to upstream order, batch, material lot, garment/serial or anonymized service cohort.

A reference string by itself is not authentication. Missing, stale, out-of-calibration, contradictory or unqualified evidence is unknown until resolved. Never replace a missing measurement with zero. Keep raw readings and corrected readings distinct, including correction reason, method and reviewer.

Capture timestamps at the level needed to audit release times, operations, changeovers, gate decisions, finished-product QA, service delivery, returns and downstream disposition. A planned completion timestamp does not prove production completion or customer acceptance.

### Required ledgers and cross-checks

For each cohort, reconcile measured counts. At minimum:

- completed new units = accepted new units + rejected new units;
- accepted new units = new units fulfilled + unsold at window end + explicitly enumerated other fates;
- eligible existing inventory = physically issued to service + eligible stock remaining + enumerated other fates;
- fulfilled service = on-time fulfilled + late fulfilled;
- realized uncancelled service demand = fulfilled service + stockout/unfulfilled service units.

If a boundary does not close, label the cohort reconciliation-failed and exclude it from claims that require a closed ledger while keeping it in the flow diagram and missing-data report. Do not silently repair numbers. Material-mass balance across scraps, rework, returns and waste is separate from garment-unit counts.

Inventory reuse needs a physical issue/transfer receipt and an identified destination/service event. Repair requires repair completion and reacceptance evidence. Resale or donation requires a downstream hand-off record; a second-use claim additionally needs evidence that the item was actually used. Recycling requires receipt and outcome data from the recycler; collection alone is not recycling yield.

## Safety and guardrail decision rule

Adoption is a multi-condition decision, not a weighted score that can trade away a hard gate. A candidate policy may be promoted only when:

- no hard readiness, chemical, effluent, care, conservation or capability gate is bypassed;
- the primary endpoint meets the preregistered minimum useful effect, with uncertainty reported;
- service and quality satisfy preregistered non-inferiority/safety margins;
- stockouts, late delivery, returns, rework and rejection are all disclosed, including increases;
- measured energy, water and materials do not reveal hidden burden shifting;
- evidence coverage and ledger closure exceed preregistered minimums;
- results replicate across the intended profile range, or the claim is explicitly restricted to observed profiles.

Initial hypothesis for a standard operational pilot: at least 10% relative reduction in unsold newly manufactured output per fulfilled service unit, with no more than 1 percentage-point regression in on-time service. These are **proposed thresholds**, not literature constants. Freeze or replace them before data collection based on business relevance and baseline uncertainty. Rare-use or specialist profiles may need their own margins and cannot be dropped solely because they are harder to measure.

## Lifecycle and circularity: no green claims from a single meter

A 2024 review by Abagnato, Rigamonti and Grosso (45 LCA publications) emphasizes that textile reuse/recycling results depend on yield, material demand, use phase, assumed virgin product displacement, substitution factor and transportation. Accordingly:

- report electricity, water, materials, transport and waste separately;
- do not turn energy savings into carbon savings without a specified energy mix and boundary;
- do not turn reuse eligibility, inventory allocation or a transfer into avoided virgin production;
- run sensitivity analysis for replacement rate, lifetime extension, transport, cleaning/care, returns, recycling yield and end-of-life;
- use a recognized, versioned LCA method or qualified independent practitioner for life-cycle claims;
- show burden shifting across impact categories instead of hiding it in one weighted score.

Reference: Abagnato, S., Rigamonti, L., & Grosso, M. (2024), [Life cycle assessment applications to reuse, recycling and circular practices for textiles: A review](https://doi.org/10.1016/j.wasman.2024.04.016), *Waste Management*, 182, 74–90.

## Digital Product Passport interoperability boundary

The European Commission lists textile apparel as a priority product group under the ESPR Working Plan 2025–2030 and currently indicates a planned textile delegated act in Q4 2027; timing and exact information requirements may evolve. This benchmark should maintain product/profile identity, composition, care/repair guidance, source lineage and disposition links, but this draft protocol/schema is **not** an adopted EU DPP schema and is not a legal compliance claim.

Official reference: [European Commission — Textile Apparel and the Digital Product Passport](https://single-market-economy.ec.europa.eu/single-market/digital-product-passport/textile-apparel_en).

## Red-team negative controls

A benchmark implementation should reject or flag all of the following:

- treatment assignment leaks future realized demand into the production decision;
- a confirmed order is dropped by a forecast rule;
- purported physical inventory use lacks an issue/transfer receipt;
- production, fulfillment or material fates fail reconciliation;
- a value is accepted with incompatible units, missing evidence, stale calibration or a future timestamp outside allowed clock skew;
- an unavailable meter is converted into zero resource use;
- profile or functional-unit strata are pooled without declared defensible weighting;
- schedule completion is substituted for accepted finished-good evidence;
- policy overrides or treatment noncompliance disappear from the primary analysis;
- synthetic cases are presented as observed sample frequencies, causal evidence or factory savings;
- one environmental indicator is used to make a whole-life superiority claim.

## Reproducible report package

Each report should package: frozen protocol and analysis plan; exact software/repository commit; input-data manifest with SHA-256 digests; versioned schema; cohort inclusion/exclusion flow; assignment and deviation log; source evidence references; reconciled unit and material ledgers; metric code/version; uncertainty and sensitivity outputs; privacy-preserving records; and a machine-readable decision with unresolved/unknown claims explicit.

The schema and example fixture are contracts; the example is synthetic and proves no effect. A typed Rust loader and cross-field invariant checker are now drafted in `crates/domains/symtropy-textile-process/src/empirical.rs`. Its negative-control tests are authored but unexecuted. Independent JSON Schema Draft 2020-12 validation, exact-head Rust test/fmt/Clippy runs, redacted pilot data and independent analyst reproduction remain required before production use. The loader checks data shape, pairing and arithmetic, not cryptographic evidence authenticity or whether a randomized assignment actually occurred.
