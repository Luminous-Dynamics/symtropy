# Careframe: Industry and Evidence Baseline v0

**Research cut-off: 23 October 2026.** This is a scoping review for a research proposal, not an exhaustive systematic review, standards-compliance determination, or medical-device regulatory opinion. Vendor metrics below are attributed to the vendor; they have not been independently verified here.

## 1. Main finding

A general-purpose “humanoid hospital assistant” is not itself a novel claim. Healthcare logistics robots are already deployed, and an assistive wheeled humanoid is already in a clinical pilot. The strongest defensible research contribution is instead a **portable, evidence-bound qualification system for bodies and tools**: the same task/capability contract, hazard gates, revision invalidation, simulation corpus, and auditable evidence ladder follow the tool across fixed-base, wheeled-manipulator, wheeled humanoid and (only where justified) biped morphologies.

The other important finding is that robot availability and technical task success must not be confused with clinical outcome improvement. A 2026 systematic review/meta-analysis of care-robot RCTs identified 34 studies with 2,476 participants. Most assessed outcome domains did not show statistically significant pooled improvements, prediction intervals crossed the null, and authors reported low/very-low certainty for outcomes. This supports narrow, workflow-specific claims and well-powered human-centred evaluations, not sweeping promises that robots improve care.

## 2. Relevant platforms and what they establish

### Diligent Robotics — Moxi / Moxi 2.0

Official sources:
- [Moxi company page](https://www.diligentrobots.com/)
- [Moxi 2.0 rollout announcement, 17 August 2026](https://www.diligentrobots.com/blog/diligent-robotics-a-serve-robotics-company-begins-rolling-out-moxi-20)

Moxi targets hospital logistics: fetching medications, lab samples and supplies. Diligent's August 2026 announcement says Moxi 2.0 is rolling out to health-system customers, draws on operations across more than 25 hospitals, and is designed for longer service windows, faster charging and better edge-case recovery. These are vendor-stated claims, not independent comparative benchmark results in this review.

**Lesson for Careframe:** logistics has the closest near-term deployment value. We should benchmark against carts and established mobile-manipulation workflows first—not only against other humanoids. A world model or autonomy claim is not a moat unless it lowers interventions and failure-recovery cost on a preregistered task suite.

### ALTER-EGO — wheeled humanoid in assistive care research

Official research sources:
- [University of Pisa / IRCCS Maugeri announcement, 20 May 2026](https://www.unipi.it/en/news/alter-ego-bring-robotics-into-hospital-wards/)
- [Peer-reviewed 2026 co-design study in Frontiers in Robotics and AI](https://www.frontiersin.org/journals/robotics-and-ai/articles/10.3389/frobt.2026.1944029/full)

The announcement describes a roughly 120 cm two-wheeled humanoid, teleoperated/semi-autonomous/autonomous modes, compliant arms and assistive tasks, with a pilot focused on care for people with ALS. The 2026 co-design paper describes multidisciplinary selection of rehabilitation use cases and an exploratory pilot; it is preliminary evidence about needs, usability and study design, not proof of broad clinical effectiveness or comparative superiority.

**Lesson for Careframe:** include stakeholder co-design and task selection *before* freezing the embodiment. Human-height form, wheels, compliant actuation and teleoperation already exist as design directions. Novelty cannot rest on those features alone.

### Clinical outcomes and care robots

- [Journal of Medical Internet Research, 2026 systematic review and meta-analysis](https://www.jmir.org/2026/1/e95232/)

The review included 34 randomized controlled studies (2,476 participants across 17 countries); 20 contributed to at least one meta-analysis. The pooled neuropsychiatric-symptom domain showed a nominally significant result, but the reported result was fragile in sensitivity analyses; the remaining six domains were not statistically significant in the pooled analyses and prediction intervals encompassed the null. The authors rated certainty low or very low and concluded that evidence does not yet justify routine broad adoption based on clinical outcome claims.

**Lesson for Careframe:** report task completion, staff time returned, intervention rates and usability as operational endpoints separately from patient outcomes. Any patient-outcome claim needs its own appropriate protocol, endpoint, sample-size plan, independent oversight and evidence; it cannot be inferred from simulator success or staff preference.

## 3. Standards and regulatory anchors to re-check for intended use

- **ISO 14971:2019 — medical-device risk management.** ISO's catalogue states this edition was reviewed and confirmed current in 2025. [Official listing](https://www.iso.org/standard/72704.html). Use it as a risk-management process reference, not as a checklist that by itself makes a device safe.
- **ISO/TS 24971-2:2026 — guidance on applying medical-device risk management to machine learning in AI.** The ISO/TC 210 catalogue lists this technical specification as published in 2026. [Official committee catalogue](https://www.iso.org/committee/54892/x/catalogue/). Check applicability with the safety/regulatory team and use it to make model/data shift, intended-purpose boundaries and evidence uncertainty explicit.
- **ISO 13485:2016 — medical-device quality management.** Establish applicable design-control, traceability, supplier and lifecycle obligations based on the intended market and product classification. Confirm the current status and any applicable amendments at project gates.
- **IEC 62304 and IEC 62366-1 — software lifecycle and usability engineering.** ISO/TC 210's current catalogue lists IEC 62304:2006 + AMD1:2015 and IEC 62366-1:2015 + A1:2020 as published references. [Catalogue](https://www.iso.org/committee/54892/x/catalogue/). Qualified specialists must decide actual applicability and software safety classification.
- **IEC 80601-2-77:2019 + AMD1:2023** is scoped to robotically assisted surgical equipment/systems. A second edition, IEC/DIS 80601-2-77, is under development. [ISO current draft listing](https://www.iso.org/standard/90452.html). Careframe V0 is not surgical; this is a watch item only if intended use changes.
- **FDA cybersecurity guidance, February 2026.** FDA's current final guidance addresses device cybersecurity design, labeling and premarket-submission documentation and supersedes its June 2025 version. [FDA guidance](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/cybersecurity-medical-devices-quality-management-system-considerations-and-content-premarket).
- **FDA RASD draft guidance, September 2026.** It is a draft for certain robotically-assisted surgical devices, not a generic guide to all care robots; comment deadline listed as 24 November 2026. [FDA draft guidance](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/robotically-assisted-surgical-devices-premarket-submissions).
- **South Africa / SAHPRA.** SAHPRA published MD08-2025/2026 on AI/ML-enabled medical devices. Its published communication stated the regulatory status as it stood at its publication date; verify current requirements, licensing, registration, classification and clinical-study permissions with qualified local specialists before any human research or deployment. [SAHPRA communication](https://www.sahpra.org.za/wp-content/uploads/2025/08/MD08-20252026_-SAHPRA-Communication-to-Industry-AI-Medical-devices-pdf.pdf).

This baseline deliberately avoids asserting that satisfying one standard, guideline, or synthetic test is equivalent to regulatory approval.

## 4. Differentiation hypothesis

| Capability | Existing evidence / incumbent strength | What Careframe must demonstrate |
|---|---|---|
| Hospital logistics | Moxi shows that focused delivery and retrieval can be deployed; incumbent has field feedback. | Complete a named logistics task with lower total operator-intervention and recovery burden than cart / current robot, under equal conditions. |
| Wheeled humanoid assistance | ALTER-EGO demonstrates an assistive wheeled humanoid and clinical co-design work. | Evidence-bound tools and authority that transfer across embodiments, with task-specific utility verified rather than presumed. |
| General-purpose tool use | Many robots can accept end effectors; individual tools still have mechanical and process constraints. | Versioned manifest and capability envelope; unknown fields, out-of-scope tasks, identity changes and unsupported hygiene claims fail closed. |
| Learning from deployments | Moxi 2.0 describes a vendor “learning flywheel” built from deployed experience. | Privacy-preserving, attributable learning with holdouts, rollback, drift checks and independent release qualification; no learning update silently changes safety authority. |
| Safety and qualification | Standards and product-specific safety cases already exist in the field. | Lower integration and requalification cost with audit-ready evidence, without lowering the hazard bar or conflating simulation with physical evidence. |
| Clinical benefit | Systematic review evidence is mixed and often low certainty. | Claim clinical benefit only after appropriately designed and governed studies; keep operational and clinical endpoints distinct. |

## 5. Recommended near-term program

1. **Start with task scarcity/value, not body shape:** partner with nurses, porters, infection prevention, facilities, occupational therapy and people receiving care to select 2–3 repetitive, measurable non-clinical tasks.
2. **Benchmark cheapest competent baseline:** human-only, cart/static workstation, conventional AMR/mobile manipulator, fixed base, wheeled humanoid; add biped only if access evidence requires it.
3. **Design a capability-and-tool compiler:** tool manifests constrain contact class, allowed task, payload/geometry, force envelope, cleaning claim state, power/interface, privacy and evidence stage. Tool designs are simulation proposals until separately reviewed.
4. **Make the robot prove changes are noticed:** mutate tool revision, calibration, authority, localization, network state, payload and consent after planning. Previously validated plans must invalidate and recover safely.
5. **Open evaluation protocols and failure cases:** frozen public corpus; separate held-out challenges; publish failures, withdrawals, uncertainty and denominators. Do not publish patient-identifying examples.
6. **Move through an evidence ladder:** synthetic contract → simulation → guarded bench/HIL → mock-ward workflow → governed human research only after independent review. Each transition requires explicit approval and a new claim ceiling.

**Proposed competitive thesis:** win first on qualification reuse, safe interruption/recovery, maintenance burden and transparent evidence—not on unsupported claims of superhuman dexterity, clinical benefit, or autonomous competence.
