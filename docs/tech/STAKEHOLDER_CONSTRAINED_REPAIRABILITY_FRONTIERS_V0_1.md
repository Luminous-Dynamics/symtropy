# Stakeholder-Constrained Repairability Frontiers v0.1

**Status:** research and decision-contract proposal. Stakeholder profiles are not yet validated with participants; no model is selected and no physical product is assessed.  
**Tracker:** [REPAIR-PROFILES-001 #1872](https://github.com/Luminous-Dynamics/symtropy/issues/1872)  
**Decision engine:** [REPAIR-DECISION-001 #1862](https://github.com/Luminous-Dynamics/symtropy/issues/1862) and [PR #1827](https://github.com/Luminous-Dynamics/symtropy/pull/1827).

## 1. The next paradigm shift: there is no single universal repairability score

A product can be easy for a manufacturer-trained technician with a stocked parts warehouse and poor for a household without specialist tools, a rural owner far from a service centre, or an independent repair shop that cannot access diagnostics or compatible parts. A single weighted repairability score hides who benefits, whose constraints were optimized, and who bears the remaining cost or risk.

A June 2026 open-access study introduces the RIUP approach—repairability according to user profiles—and reports differences between design-team, consumer and maintenance-team perspectives in an illustrative heat-sealing-machine case. That result motivates stakeholder-separated analysis; it does not establish the correct appliance profiles or thresholds for this project. See [A stakeholder-centered repairability index supporting circular product design](https://doi.org/10.1016/j.cesys.2026.100419).

**Our extension:** keep universal hard gates and evidence eligibility, then evaluate each stakeholder profile as an explicit set of constraints and a view over the Pareto frontier. Do not create a hidden average consumer, universal scalar score, or invented numeric thresholds.

## 2. Five initial stakeholder lenses

These are research hypotheses to be validated through interviews/usability work; they are not claims about every person in each group.

### P1 — Household owner and intended user

**Question:** Can ordinary maintenance be understood and safely completed without specialist tools, account access or unnecessary downtime?

Focus: approved user-maintenance task success, understandable instructions, total owner cash cost, outage burden, consumables and clear referral points. Only intended user-level operations belong in this profile. Internal mains, heater/drive, heavy rotating assembly, stored-energy and hazardous pressure/refrigerant tasks must never be reclassified as DIY.

### P2 — Qualified independent repair technician

**Question:** Can the fault be discriminated, repaired and verified efficiently without unnecessary assembly replacement or proprietary lockout?

Focus: diagnostic discrimination, access/disassembly time, manuals and test points, exact part compatibility, tool/fixture needs, first-visit verified restoration, rework and follow-up recurrence. The profile does not relax qualification, isolation or post-repair test obligations.

### P3 — Parts supplier and local repair network

**Question:** Can a lawful, compatible part be identified, sourced, stocked and delivered at a sustainable cost?

Focus: exact-fit catalogue quality, supersession mapping, source independence, local delivery/fulfilment, inventory obsolescence, packaging, returns and stock economics. A generic public listing is not proof of fit, independent supply or delivery.

### P4 — Lifecycle and material steward

**Question:** Does repair retain useful function with lower whole-life resource and material burden?

Focus: discarded mass per verified repair, repair-versus-replacement lifecycle cost, energy/water per successful service, durability, refurbishment/reuse and end-of-life recovery. Avoid assuming that a lighter module or a lower part count implies better environmental performance.

### P5 — Remote or institutional operator

**Question:** Can a machine serving a home, clinic, school, farm, shelter or remote site be safely restored when service access and spare-part logistics are poor?

Focus: elapsed downtime, availability of high-consequence parts, maintainability with locally available tools, offline documentation, diagnosability, safe degraded operation where the product supports it, and planned service intervals. Do not assume remote users have technician skills or should open hazardous assemblies.

## 3. Decision mechanics: hard gates -> profile constraints -> Pareto frontier

Each candidate is evaluated in the same order.

1. **Universal hard gates:** safety, core function, durability/post-repair verification and authorization for the planned test. A failed gate rejects the candidate; an unknown gate blocks it. No stakeholder preference can waive these gates.
2. **Evidence eligibility:** every required metric needs a frozen definition, valid unit, physical/independent evidence class where physical ranking is intended, traceable source/method identity and valid uncertainty bounds.
3. **Profile requirements:** the profile owner and research team preregister measurable requirements (e.g., maximum owner cost, minimum locally delivered part coverage, or maximum outage) from participant input and local context. Until thresholds are agreed, their state is `requirements_not_frozen`; the profile cannot declare a satisfied/selected design.
4. **Profile-specific frontier:** among candidates that pass the universal gates and the profile's frozen constraints, report robust Pareto non-dominated options for the declared criteria. A candidate may suit one profile and fail another. Report this explicitly rather than averaging away disagreement.
5. **Unresolved choices:** when candidates are incomparable or uncertainty overlaps, select the next measurement using the existing Symthaea EIG/EVSI kernel, conditioned on test safety and decision relevance. Do not duplicate the scorer.

Thresholds may differ by market and profile, but must be established before looking at candidate comparative results. If profiles disagree, the result is an exposed design trade-off that needs an explicit decision or another experiment—not a reason to tune the weights until everyone appears to agree.

## 4. Requirements are not scores

For each profile requirement, store:
- requirement ID and criterion ID;
- stakeholder rationale and source (interview, service policy, legal requirement, budget scenario, or explicit project constraint);
- direction and unit;
- threshold (nullable until agreed);
- market and intended participant/use context;
- gate/constraint vs descriptive preference;
- owner, version and approval state;
- date frozen relative to candidate measurements.

A missing threshold remains unknown. Do not choose arbitrary numbers merely to make the ranking algorithm produce an answer. “90% of common tasks in 10 minutes” may be a research hypothesis, but it is not a requirement until the task set, user population, task risk, timing boundary and acceptance rule have been agreed and preregistered.

Some constraints are hard and non-negotiable (applicable safety requirements). Others describe preferences (repair cost, downtime, material burden). Do not transform a preference into a product-safety claim.

## 5. Benchmarks by stakeholder profile

Use metrics as evidence-bearing outcomes, not as an opaque universal score.

| Profile | Candidate benchmark dimensions | Required evidence |
|---|---|---|
| Household owner/user | Approved-task success, time, errors, assistance needed, safe stopping/referral, out-of-pocket costs | Representative usability study, validated task scripts, consent and safety boundaries |
| Independent technician | Fault isolation accuracy, hands-on time, first-visit verified restoration, post-check completeness, rework and recurrence | Controlled service-task protocol, participant competence declared, exact parts/tools/procedure |
| Parts/service network | Exact-fit evidence, dual-source coverage, order fulfilment, delivered lead time, stock obsolescence | Dated written quotes/orders, source identity/independence, exact model/revision compatibility |
| Lifecycle/material steward | Lifecycle cost range, energy/water per successful output, discarded mass, material recovery, useful lifetime | Declared lifecycle model, measured resource use, mass balance and follow-up/end-of-life assumptions |
| Remote/institutional operator | P90 outage, availability of critical parts, offline documentation access, restore success within declared capability | Named remote service scenario, logistics test, task/role boundaries, safe operational limits |

Universal hard gates remain above this table. Where a result depends on assumptions, publish sensitivity and uncertainty rather than implying precision.

## 6. Research and validation plan

1. **Desk synthesis:** map stakeholder types, product-system barriers and product-service-system barriers. The 2025 scoping review on household appliance repair barriers identifies access/understanding/disassembly, obsolescence and safety alongside economic factors, time and access to information. See [Rovira-Menaya et al., PLATE 2025](https://doi.org/10.54337/plate2025-10288).
2. **Short interviews:** seek input from household users, independent repair technicians and local parts suppliers in the selected launch market. Ask participants to identify unacceptable failure modes, realistic tool access, travel/lead-time constraints and what counts as restored function. Do not ask interviewees to endorse a product design that has not been built.
3. **Preregister profile requirements:** publish the interview protocol, recruitment limits, questions, coded themes, disagreement and thresholds before candidate results.
4. **Test profile mechanics synthetically:** unit tests demonstrate that unfrozen constraints remain unresolved, constraints cannot bypass hard gates, and profile views do not create a universal winner.
5. **Validate physically:** after control and methods are frozen, compare architecture candidates under the same task/function profile. Include stakeholder-specific usability/service trials and report the separate frontiers.

Do not infer South African household or technician needs from European repair-index findings; use the paper as a method reference, then validate with people in the actual launch context.

## 7. Data boundaries and evidence owners

- **Symtropy:** architecture, benchmark definitions, stakeholder profiles and lifecycle trade-offs.
- **Symthaea:** advisory experiment prioritization using existing EIG/EVSI; no safety authority.
- **Mycelix:** existing artifact/unit/repair/material/post-check/recurrence evidence owners; do not create a second repair ledger.
- **Physical appliance:** deterministic protection and qualified procedures remain authoritative.
- **Research participants:** their preferences and reported barriers are evidence with context and limitations, not a substitute for physical tests or legal requirements.

## 8. Claim ceiling

This artifact proposes five stakeholder lenses and a validation method. It does not establish that these profiles are exhaustive or representative, that any thresholds are agreed, or that a washer is more repairable. No profile-specific selection is valid until universal gates pass, profile requirements and methods are frozen, and the required evidence is collected.
