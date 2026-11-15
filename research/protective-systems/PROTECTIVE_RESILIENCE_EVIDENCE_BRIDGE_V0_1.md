# Protective Resilience Evidence Bridge v0.1

**Snapshot:** 15 November 2026  
**Owner:** PROTECT-THREAT-PORTFOLIO-001 #5433  
**Parent:** PROTECTIVE-SYSTEMS-001 #5246  
**Status:** research contract and synthetic conformance fixtures; not yet an executable qualification gate

## 1. Purpose

Connect the defensive threat register and lifecycle investment decisions to the existing service-resilience, robustness, recovery and value-of-information work.

The unit of investment is a **capability portfolio that preserves a declared human or essential service over a stated lifecycle**. The unit of measurement is the service and beneficiary trajectory, not the number of protected assets or a composite threat score.

This bridge is an adapter and evidence contract. It must not create a second service-cascade simulator, resource ledger, uncertainty optimizer, recovery engine, materials model or certification authority.

## 2. Compose existing owners

| Owner | Owns | Bridge consumes |
|---|---|---|
| PROTECT-THREAT-PORTFOLIO #5433 | Hazard evidence, context mapping, lifecycle investment and cross-owner portfolio | Hazard-family IDs, admitted evidence state, asset/service context and disposition |
| CIVDYN-CASCADE #3894 | Time-dependent service dependencies and cascade consequences | Service graph scenario IDs, per-service trajectories, deficit and sustained recovery outputs |
| CIVDYN-RES #3621 | Cross-scale shock and recovery trajectories | Context, scale, shock sequence and distributional recovery outputs |
| CIVDYN-RES-002 #3739 | Separate service, buffer and ecological/physical recovery | Recovery-quality state families; never infer whole-system recovery from service restoration alone |
| PROTECT-ROBUST #5317 | Uncertainty sets, hard floors, regret and robust multi-objective comparison | Frozen uncertainty set, feasibility, tail/regret and abstention outputs |
| PROTECT-VOI #5268 | Experiment and observation value | Whether new evidence changes a feasible decision, and decision regret avoided per fully burdened observation cost |
| CIVBENCH-MAD #3731 | Measurement-as-distribution protocol | Unit of analysis, sampling/replication design, strata, holdouts and reproduction receipt |
| POWER-SYSTEM-RESILIENCE #3418 and EMSR #10775 | Power, electromagnetic and PNT domain qualification | Their domain-owned evidence and capability outcomes; no duplicate electrical or RF model |
| Mycelix lifecycle/provenance owners | Exact configuration, inspection, qualification and lifecycle evidence | Identity and lineage receipts; not physical truth or certification |
| Sol-Atlas | Rights-aware spatial/context projections | Only admitted, appropriately scoped spatial context |

A missing upstream result is not silently reconstructed by this bridge. It yields **ABSTAIN_MISSING_UPSTREAM_EVIDENCE**.

## 3. Contract: context before comparison

Every comparison must freeze:

- context identifier, jurisdiction and declared system boundary;
- applicable legal, regulatory, municipal, sector and standards requirements, including exact edition/amendment, effective status, scope, authority and applicability rationale;
- affected services and beneficiary strata;
- baseline and intervention portfolio identifiers;
- hazard-family references and scenario references;
- scenario time horizon, event sequence and bounded/unbounded status;
- service floors, tolerances, sustained-recovery windows and any mandatory human-safety constraints;
- physical, digital, organizational and supply dependencies admitted by the owning model;
- evidence revisions, method versions, configuration identity and input digests;
- lifecycle horizon and cost assumptions, including installation, energy, staff, inspection, calibration, maintenance, consumables, spares, downtime, replacement and decommissioning.

A scenario is not an event probability. Deterministic stress cases stay deterministic. Probability and expected annualized loss are permitted only when a source-backed, context-specific frequency model and consequence distribution are admitted by the relevant owner.

## 4. Outcome contract: keep state families separate

### 4.1 Service trajectory

For every service and beneficiary stratum, report in native units:

- minimum delivered service and declared floor;
- total time below floor;
- integrated service deficit over the declared horizon;
- onset and propagation time where supported;
- time to recovery using a predeclared sustained-service window;
- residual service deficit at the end of the horizon.

The benchmark must preserve the complete trajectory or a reproducible time-series receipt. A first threshold crossing is not sustained recovery.

### 4.2 Buffer and recovery condition

Report separately, where supported by the owning model:

- usable reserve / storage and remaining fraction;
- backup runtime or replenishment runway;
- redundancy and alternate-source condition;
- maintenance backlog and repair capacity;
- spares, consumables and operator availability;
- recovery-resource constraints.

A service can recover while its reserves are depleted or its maintenance backlog grows. This is a mixed result, not a whole-system recovery certificate.

### 4.3 Human, ecological and physical outcomes

Keep human safety, accessibility, health, ecological condition, physical stock and service outcomes as distinct dimensions. Do not sum incompatible units or monetize a mandatory life-safety floor.

No domain receives a pass from another domain's improvement. Unknown is not zero, safe, available or recovered.

## 5. Portfolio comparison and economics

At minimum, compare a matched context against:

1. persistence / no new intervention;
2. the strongest simple domain-appropriate control supported by current evidence;
3. a cross-cutting continuity and recovery portfolio;
4. a specialized control only where exposure and efficacy evidence justify it;
5. a bounded exact oracle on tiny synthetic cases, where tractable.

Each candidate must use the same context, service demand, scenario sequence, uncertainty set, evidence budget and evaluation horizon. Report Pareto outcomes rather than a universal resilience score.

Permitted economic outputs, when inputs are adequate:

- incremental present-value lifecycle cost and assumptions;
- cost per beneficiary-year protected;
- cost per critical-service-hour preserved;
- avoided service-deficit area and sustained-recovery improvement;
- uncertainty range and robust regret;
- inspection, maintenance, training and support burden;
- residual risk and repairability.

Do not add the same shared capability benefit once per hazard family. A shared benefit is counted once in a portfolio ledger and linked to every relevant service/hazard claim. Compare complete portfolios when interactions or common-cause dependencies are material; do not sum isolated intervention benefits as if independent.

If denominators, frequency evidence, service units or lifecycle scopes are incompatible, do not publish a cost-effectiveness ranking.

## 6. Decision and fail-closed semantics

- **FEASIBLE:** all mandatory human-safety and service constraints pass for the declared scope.
- **INFEASIBLE:** at least one mandatory constraint is violated; economic or unrelated performance gains cannot compensate.
- **ABSTAIN_MISSING_UPSTREAM_EVIDENCE:** a required service, uncertainty, recovery or provenance result is absent.
- **ABSTAIN_INCOMPARABLE_SCOPE:** candidates do not share the same boundary, horizon, demand or measurement basis.
- **ABSTAIN_UNSUPPORTED_PROBABILITY:** a numeric probability / expected loss is requested without admitted support.
- **ABSTAIN_DOUBLE_COUNTED_BENEFIT:** overlapping benefit claims have not been reconciled.
- **MIXED_SERVICE_BUFFER_RECOVERY:** service conditions pass but a required buffer/stock/recovery condition does not.
- **MONITOR_OR_PREPARE:** proportionate, reversible readiness is justified but specialized capital is not.
- **DEFER_NO_DEDICATED_INVESTMENT:** the fieldable pathway or relevant exposure is not established.

The bridge must not convert an abstention into a favorable default or silently drop an infeasible candidate.

## 7. Benchmark portfolio: synthetic conformance tasks

The companion fixture file defines six bounded, synthetic checks:

1. a small water-service context with power / communications continuity and recovery evidence;
2. service restored while reserve condition remains depleted;
3. one multi-hazard capability whose benefit must be counted once;
4. a new observation that changes uncertainty but not the feasible action;
5. a mandatory safety-floor violation with attractive economics;
6. unknown event frequency where expected annualized loss must remain unavailable.

The fixtures test semantic boundaries, not physical hazard effects, operational procedures or forecasts. They are not measurements of any real utility, hospital, city or installation.

## 8. External standards and evidence anchors

These are interoperability and research anchors, not declarations of certification or complete compliance.

- **NIST Community Resilience Planning Guide:** connects buildings and infrastructure performance to social/economic functions, interdependencies and desired recovery-time goals. Use its service-performance framing; retain the domain-specific model owner for calculations.  
  https://www.nist.gov/community-resilience/planning-guide  
  https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190GB-16.pdf
- **NIST SP 1339 (17 June 2026), OT Backup Quick Start Guide:** emphasizes integrating backups with change management, creating them regularly, testing them and reviewing them in recovery exercises. Treat tested restoration—not backup-file existence—as the recovery evidence.  
  https://csrc.nist.gov/pubs/sp/1339/final
- **NIST SP 1800-45 (24 June 2026), Water and Wastewater OT Remote Access:** provides three reference designs for secure remote access to water/wastewater OT using commercially available technology. This is a useful public reference context, not a universal utility architecture or a resilience score.  
  https://csrc.nist.gov/pubs/sp/1800/45/final
- **NIST SP 800-82 Rev. 4 Initial Public Draft (21 September 2026):** relevant to OT safety, reliability and security. As of this snapshot it is a draft, with comments due 30 November 2026; do not label it final or normative.  
  https://csrc.nist.gov/pubs/sp/800/82/r4/ipd
- **ISO 22301:2019 and Amendment 1:2024:** business continuity management-system requirements and climate-context amendment. ISO indicates a revision is in progress; preserve edition/amendment and jurisdiction applicability in any future standards register.  
  https://www.iso.org/standard/75106.html

## 9. Benchmark ladder and prospective targets

| Gate | Evidence required | Prospective target |
|---|---|---|
| B0 Contract conformance | Fixture and schema checks | 100% of mandatory fields and invariants; zero silent defaults |
| B1 Owner interoperability | Frozen upstream scenario and output receipts | 100% of consumed results trace to the declared owner, scenario and exact revision |
| B2 Decision reproducibility | Frozen context, portfolio, assumptions, code and evidence digests | 100% reproducibility of deterministic decisions |
| B3 Matched portfolio comparison | Equal scope, demand, horizon, information and lifecycle accounting | Demonstrable Pareto or cost/recovery improvement over a declared baseline |
| B4 Independent retrospective | Rights-cleared historical/public evidence and independent domain review | Reproduced baseline first; model discrepancy and unsupported strata disclosed |
| B5 Prospective/physical validation | Approved facilities, competent independent testing, preregistration and safety review | Real outcomes match the declared claim envelope within predeclared tolerance |

Primary endpoints:

- zero hard-safety-floor violations accepted;
- zero duplicate shared-benefit credit;
- zero invented event probabilities;
- zero whole-system recovery claims from service-only evidence;
- zero cost-effectiveness rankings across incomparable scopes;
- complete provenance for every investment recommendation;
- measured improvement in service-deficit area, sustained recovery or lifecycle cost against a matched baseline.

These are targets, not achieved results. Synthetic conformance is not field validation. No procurement, physical test, human-subject study or operational actuation is authorized by this contract.

## 10. Claim ceiling

The bridge may claim that it correctly connects declared evidence, scenario outputs and decision boundaries only after the relevant conformance and reproduction gates pass.

It may not claim:
- that a synthetic case predicts a real incident;
- that a design is certified, safe or universally resilient;
- that upstream model output proves physical truth;
- that a high score establishes protection against an emerging or exotic weapon;
- that a service recovery implies reserve, ecological or whole-system recovery;
- that an untested backup, sensor or fallback mode is operationally effective.

The intended advance is a reproducible chain:

**hazard evidence → context → service requirements → owner-qualified scenarios → service/buffer/safety trajectories → lifecycle portfolio comparison → independent evidence and bounded claims.**


## 11. Industry comparators and interoperability boundary

A fair comparison must recognize established work rather than relabel it as new.

### IN-CORE and NIST community resilience methods

IN-CORE is a serious external comparator, not a straw-man baseline. Its published architecture already connects community inventories, hazard and fragility information, physical infrastructure damage/functionality, repair/recovery/restoration and social/economic impacts. Its services include hazard, damage/functionality/repair/recovery/restoration (DFR3), data and geospatial functions. NIST's Community Resilience Planning Guide supplies a stakeholder-led planning method and performance goals expressed as recovery times for community functions.

The Luminous bridge must therefore **not** claim novelty for:
- infrastructure dependency modelling;
- hazard-to-damage-to-functionality chains;
- community recovery trajectories;
- social/economic consequence assessment; or
- recovery-time performance goals.

Instead, benchmark interoperability and the additional decision-assurance contract:
- Can a result from an authoritative external or domain model be imported with its scenario, boundary, data revision, method revision, uncertainty and claim ceiling intact?
- Can the bridge compare complete portfolios across several model owners without silently substituting missing values or treating model output as physical truth?
- Are mandatory human-safety/service constraints non-compensatory?
- Are service, reserve/buffer, maintenance and ecological/physical recovery kept distinct?
- Are shared benefits attributed once, with overlaps explicitly reconciled?
- Can all accepted recommendations be reproduced from frozen evidence and configuration receipts?

Where an IN-CORE testbed/data set is accessible and its scope is compatible, use the published baseline or platform output as a comparison. Do not claim a matched result until hazards, asset boundary, service demand, scenario, recovery assumptions, population and evaluation horizon have been aligned. If a compatible dataset cannot be obtained, report an interoperability gap, not a benchmark win.

References:
- NIST-funded IN-CORE measurement-science and model architecture: https://www.nist.gov/publications/interdependent-networked-community-resilience-modeling-environment-core
- NIST Center of Excellence and IN-CORE service architecture: https://www.nist.gov/community-resilience/center-excellence
- IN-CORE open platform and technical documentation: https://in-core.org/ and https://dev.in-core.org/doc/incore/introduction.html
- NIST community planning guide and performance goals: https://www.nist.gov/community-resilience/planning-guide and https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190GB-16.pdf

### ISO 22301 / SANS 22301

ISO 22301 is a business-continuity management-system standard, not a physical hazard simulator or a guarantee that a facility will never be disrupted. Treat it as an organizational capability / process evidence comparator: business-impact analysis, continuity objectives, roles, plans, exercises, monitoring, management review and continual improvement. A certificate's scope and audit evidence must be inspected; the existence of a certificate alone does not prove a particular service trajectory.

For South African deployments, the SABS catalogue lists SANS 22301 (version 2.00). The exact adopted edition, amendment, scope and applicable sector rules must be confirmed for each deployment rather than inferred from a generic ISO reference. The South African Disaster Management Act 57 of 2002 and the 2023 National Disaster Management Framework review establish a multi-level disaster-planning context; local/provincial/municipal applicability requires competent local review.

References:
- SABS standards catalogue: https://www.sabs.co.za/sabs-standards
- South African Disaster Management Act 57 of 2002: https://www.gov.za/documents/disaster-management-act
- National Disaster Management Framework review, Government Gazette 48874 (30 June 2023): https://www.gov.za/sites/default/files/gcis_document/202307/48874gon3635.pdf
- ISO 22301 overview: https://www.iso.org/standard/75106.html

### Operational technology implementation references

NIST SP 1339 makes backup testing and recovery exercise review part of OT recovery discipline. NIST SP 1800-45 supplies water/wastewater OT remote-access reference designs. These are implementation evidence and architecture comparators, not a substitute for physical service performance tests or local compliance review.

The standards crosswalk must always distinguish:
- published final standard / guidance;
- national adoption and edition;
- draft or public-comment document;
- sector-specific regulation;
- local operator policy;
- independent physical qualification.

As of this snapshot, NIST SP 800-82 Rev. 4 is an initial public draft, not final guidance.

## 12. Comparison protocol: how a superiority claim can be earned

The first comparison should be a frozen, synthetic, hand-solvable case that exercises a subset of the real owner interfaces. The next should be an accessible, rights-cleared retrospective case with an independent domain reviewer. Do not begin with a city-scale digital twin or an opaque AI-vs-AI leaderboard.

For every baseline, freeze:
- the same context/jurisdiction, boundary, service demand and beneficiary strata;
- the same admitted scenario(s), event order and time horizon;
- the same source data and source revisions;
- the same permitted observations and engineering assumptions;
- the same mandatory safety/service floors;
- the same lifecycle cost boundary and discount assumptions;
- the same compute / human-review / measurement budget where relevant.

Baselines must include:
1. no-change / persistence;
2. a simple competent domain rule or current operator baseline;
3. the applicable continuity-management plan / exercise evidence;
4. an applicable external model or tool such as IN-CORE only where a genuinely matched testbed is available;
5. existing Symtropy domain owners and their bounded exact oracles;
6. a portfolio bridge result.

Report native-unit outcomes and uncertainty intervals, not a universal rank. Required comparisons include service-deficit area, time below floor, sustained recovery time, reserve and maintenance condition, worst-served stratum, human-safety feasibility, whole-life cost and decision reproducibility.

Prospective stretch goals for a future independent case study—not current results:
- at least 20% lower fully burdened lifecycle cost while preserving every agreed safety and service floor; **or**
- at least 20% lower integrated critical-service deficit at equal fully burdened lifecycle budget;
- zero false-feasible decisions on the frozen negative-control set;
- 100% deterministic reproduction from frozen inputs and exact model/evidence revisions;
- zero unadjudicated duplicate benefit claims or unsupported probability claims.

The 20% thresholds are proposed materiality targets. Before evaluation, reviewers must verify that the sample, scenario severity and measurement precision can resolve an effect of that size. If the data cannot support a 20% estimate, report the confidence interval and the smallest detectable effect; do not present an underpowered null as proof of equivalence.

A superiority claim requires:
- baseline reproduction first;
- an independently reviewed protocol and frozen holdout;
- complete cost accounting;
- uncertainty and applicability limits;
- disclosure of cases where the Luminous bridge loses or abstains.

Synthetic case success demonstrates software contract conformance only. It is not evidence that a real protective system, facility or community is more resilient.
