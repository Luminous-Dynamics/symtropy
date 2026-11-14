# Holiday Experience UX v0.1
## Human-centered journey, interaction contract and evaluation plan

**Status:** research design / draft requirements. No implemented interface, user study, accessibility audit or field outcome is claimed by this document.

**Scope:** discovery and planning of holidays spanning resorts, theme parks, water parks, cruises, ski/mountain destinations, nature retreats, cultural trips and combinations. This work composes with JOY-WORLDS rather than creating another destination simulator or a general booking marketplace.

## 1. The goal: make time away better for people

A holiday is not just a transaction or a sequence of activities. It is a scarce block of time, money, energy and attention, often coordinated across several people and organizations. The useful design question is:

> Can people shape a trip that fits what matters to them, understand the compromises, enjoy more of their time away, and recover gracefully when reality diverges from the plan?

The product should optimize for informed choice, low avoidable friction, agency, inclusion, resilience and evidence of post-trip fit. It must not optimize click-through, time in app, upsell rate, itinerary density or spending as proxies for a good holiday.

A recommendation to stay home, take a shorter trip, choose a cheaper destination, book fewer activities, keep an afternoon free, or not book yet can be a correct outcome.

This programme does **not** assume that happiness is universal, objectively observable through a phone, or reducible to one number. Experience intents—rest, play, adventure, awe, connection, culture, learning, solitude, nature, celebration—are user-chosen prompts, not a psychometric model.

## 2. Paradigm shifts

| Old optimization target | Better design target | What changes in the system |
|---|---|---|
| Destination/category first | Human intent first | Ask what the traveller wants the time away to make possible, then find suitable places |
| Maximize activities per day | User-controlled rhythm | Show movement, effort, transfers, downtime and optionality; protect unscheduled time |
| “Accessible” badge | Evidence along the whole access chain | Distinguish verified detail, partially known detail, barrier and unknown; include transport, rooms, routes, toilets, activities and emergency information |
| Search rank as hidden score | Explainable, comparable trade-offs | Show why an option matches, what it costs, what is uncertain and who may find it burdensome |
| Booking conversion | Informed, reversible decisions | Reveal total cost, cancellation and material restrictions before commitment; never auto-book |
| Fixed itinerary | Resilient plan | Provide user-approved alternatives for delay, closures, weather and tiredness |
| Personalized by default | Private by default, user-controlled | Work without a mandatory account; minimize sensitive data; let users edit, export and delete what they save |
| Visitor volume and revenue | Visitor experience plus resident, worker and ecological boundaries | Keep safety, rights, access and known environmental limits as hard constraints |
| Post-trip star rating | Optional reflection against the traveller's own intent | Learn from voluntary feedback without pretending self-report establishes objective wellbeing |

These shifts are hypotheses to test, not proof that a particular UI will improve outcomes.

## 3. Experience contract: seven stages

### Stage A — Set the intent without a long quiz

Start with one plain-language prompt: **“What would make this time away feel worthwhile?”** Offer a small, skippable set of multi-select intents, such as rest, time with people, adventure, nature, food/culture, learning, novelty, celebration or simply getting away. Allow a free-text answer and a “not sure yet” path.

Do not force an account, full itinerary, precise location or sensitive disclosure before showing useful options. Ask questions progressively, only when the answer can change a recommendation. Separate **must-haves**, **preferences** and **unknowns**.

### Stage B — Make constraints legible

Capture only what is useful to the decision: date range and flexibility, party composition when relevant, total budget range and currency, likely departure area when needed, travel time tolerance, pace, transport preference, dietary requirements if volunteered, accessibility and sensory needs if the traveller chooses to share them, and any non-negotiables.

For group planning, private preference details remain private by default. Share only the constraints and options the person approves. A group decision should reveal incompatible requirements and trade-offs—not silently average them away or let the loudest participant overwrite others.

### Stage C — Compare options as trade-offs, not bait

A shortlist should make the options comparable at a glance. Each option should show, where applicable:

- Full known price for the selected dates and party: base price, mandatory taxes/fees, currency, what is included, and what can still change.
- Travel time, number of transfers, arrival/departure friction and relevant uncertainty.
- Cancellation/change terms and material restrictions before a commitment.
- Evidence-backed accessibility details, with a visible state of **verified**, **partially verified**, **barrier known**, or **unknown**. Never infer a step-free room, transfer, accessible vehicle or suitable activity from a generic badge.
- Expected effort: walking/standing, stairs or terrain, activity duration, likely queue or wait uncertainty where evidence exists, and quiet/rest alternatives.
- Local character and local operator options; any claim of local benefit must have a stated basis rather than marketing language alone.
- Material seasonal/weather/resource constraints and closure policies when they affect the trip.

An explanation should answer “Why is this shown?” and “What could make this a poor fit?” The interface must not imply that an option is universally best. Where data cannot be compared honestly, say so.

### Stage D — Build a flexible plan

The itinerary is an editable hypothesis, not a command schedule. Display each day as a rhythm with anchors, travel/transfer blocks, meals, recovery/buffer time and optional activities. Users control intensity and may choose “leave the day mostly open.”

Show map and list views with equivalent functionality. Map gestures must have non-drag alternatives. Give a clear day-load view that makes long travel, stairs, long standing, tightly coupled reservations and missing breaks visible. Recommendations may suggest moving, dropping or substituting an activity; they may not silently alter a confirmed booking or assume the traveller wants more.

Every itinerary item should expose its duration, location, prerequisites, opening/availability evidence, booking status, cost contribution and uncertainty where known. Separate confirmed inventory from suggestions and synthetic examples.

### Stage E — Commit with informed consent

Before any external booking or payment handoff, repeat the exact product/service, dates, party, total due now, known future charges, cancellation/change terms, accessibility requirements confirmed with the supplier, and what is *not* guaranteed. No hidden preselected extras, fake urgency, countdown coercion or ambiguous subscription/recurring charges.

The initial prototype stops before live inventory and payment. Later transaction integration is a separate security, legal, reliability and partner-authorization gate. A recommendation engine has no authority to transact.

### Stage F — Support the real trip, including disruption

Provide a compact, low-bandwidth/offline-ready plan with user-selected essentials, booking references, transit/arrival details, locally relevant contact information and last update time. Avoid collecting precise location continuously. A destination or booking update must identify what changed, the evidence/source and timestamp, and the affected itinerary items.

For a closure, severe weather, missed connection or fatigue change, use a recovery flow:
1. Explain the impact and distinguish confirmed facts from forecast or unknowns.
2. Offer a few feasible choices, including rest, no replacement or ending the day early.
3. Compare changes in cost, time, transport/accessibility fit, lost reservations and effort.
4. Let the traveller edit/accept/decline; show what has and has not changed.
5. Preserve the old plan and the reason for the change in the user's own trip history, with delete/export controls.

Never present an unverified alternative as confirmed or turn a disruption into an upsell funnel.

### Stage G — Reflect, then stop

After the trip, offer optional questions tied to the person's original intent: what worked, what was tiring or stressful, whether the pace fit, whether access information was accurate, and what they would change next time. Allow skip/decline, delete and export. Do not infer mood from movement, spending, face, voice or wearable data. Do not use guilt, streaks or reward loops to solicit reviews.

A person's report of feeling restored or connected is meaningful but subjective. Report response rates, selection bias and sample size; do not label such responses objective wellbeing measurements.

## 4. Proposed prototype: screens and essential states

Build a narrow, static/synthetic-data prototype before connecting bookings:

1. **Intent brief:** a one-question entry, skip path and editable constraints.
2. **Option comparison:** three synthetic destinations with an explicit total-cost breakdown, effort/pacing attributes, accessibility evidence states and visible unknowns.
3. **Day planner:** same data in list and map representations, optional activities, buffers, an open-day mode and edit controls.
4. **Disruption exercise:** cancel one activity, introduce weather/transport disruption, then offer feasible alternatives with cost/time/access changes.
5. **Confirmation summary:** demonstrate exact consent and disclosure requirements without charging, reserving or contacting vendors.
6. **Offline trip card:** demonstrate readable essential information and a clear freshness timestamp.
7. **Optional reflection:** show consent, skip, export and delete paths.

Use synthetic records labelled as synthetic, never imply current availability, verified accessibility, live prices, safety or a real supplier relationship. The first test can run without login, geolocation permissions, analytics or a backend.

## 5. Inclusion and accessibility requirements

Target WCAG 2.2 AA for the in-scope digital experience, plus research with disabled travellers and the relevant assistive technologies. A standards checklist is necessary but does not prove that a whole physical trip is accessible.

At minimum, test:
- full keyboard operation, visible focus and focus not obscured by sticky UI;
- screen-reader names, headings, form labels, errors, announcements and reading order;
- text zoom/reflow, text spacing, contrast and use without color alone;
- touch target size and spacing, no drag-only interaction, reduced motion and no time pressure;
- language clarity, persistent help, predictable navigation and recovery from input errors;
- login/account pathways, if any, without forcing recall/transcription or blocking password-manager paste;
- small-screen and low-bandwidth use;
- representative complete journeys for different mobility, dexterity, vision, hearing, cognitive, sensory, fatigue and communication needs.

Recruit disabled participants and compensate them for their expertise. Do not make one participant represent a whole disability group. Distinguish user-reported barriers from conformance results and physical-site verification.

## 6. Privacy, trust and decision authority

- Collect the minimum information needed for a declared function; browsing and prototype planning should work without an account.
- Make every personalization dimension inspectable and editable. Provide a neutral/non-personalized path.
- Do not infer or store emotional states; do not optimize for compulsive use.
- No hidden cross-site tracking, persistent sensitive visitor dossier or sale of accessibility/health-related preference data.
- Do not share a travel-party member's individual constraints with others without that member's permission.
- Every material recommendation has an explanation, source/last-updated metadata when available, and visible uncertainty.
- Unknown inventory, price, access, weather or travel conditions remain unknown, never “probably fine.”
- Safety, accessibility minima, legal restrictions, privacy rights and known ecological constraints cannot be overridden by a higher commercial or experience score.
- Operators, transport providers, qualified accessibility professionals, local communities, rights-holders and regulators retain their respective real-world authority. Symtropy compares declared options; Symthaea may advise; Mycelix may support transparent community agreements. None of those systems becomes booking, safety or legal authority by recommendation.

## 7. Outcome model: no single holiday score

Keep a versioned vector with definitions, units, source, time horizon, subgroup splits and uncertainty. Recommended dimensions:

- **Task usability:** completion, time, critical errors, recovery and user-rated ease.
- **Planning burden:** number of repeated entries, avoidable decisions, reported mental effort and changes needed after arrival.
- **Agency and trust:** understanding of why an option appears, ability to edit/refuse, confidence in total cost and data freshness.
- **Pace/effort fit:** planned versus actual travel/effort burden, schedule feasibility, buffer consumed and voluntary pace-fit response.
- **Access-chain reliability:** accuracy and completeness of the stated access information; unresolved barriers by stage, not merely an overall badge.
- **Disruption resilience:** time to understand options, viable alternatives available, extra cost/time, and preservation of user choice.
- **Group coordination:** unresolved conflicts, approved shared constraints and privacy violations (zero tolerated).
- **Trip-fit reflection:** optional self-report against the traveller's chosen intent, response rate and selection-bias caveat.
- **Host/worker impacts:** local value claims with traceable basis, staffing impact and resident/service pressure where the destination model has suitable evidence.
- **Ecological/resource boundaries:** relevant water, energy, waste, habitat, seasonal and crowd constraints from the qualified destination model.

Report medians, distributions and worst-served cohorts where sample size and privacy permit. Don't rank destinations with one composite "joy", "happiness", "wellbeing" or "sustainability" number.

## 8. Study protocol and baseline

Before collecting results, preregister:
- research questions and hypotheses;
- prototype/build identity and synthetic fixture data;
- task scripts, success/error definitions, stop rules and exclusion criteria;
- participant recruitment, accessibility needs, compensation, consent and data retention;
- baseline flow, order/counterbalancing and known learning effects;
- metrics, subgroup analysis, missing-data treatment and minimum meaningful difference;
- failure severity and a procedure for reporting serious access barriers;
- which results are exploratory versus confirmatory.

Use at least two rounds. **Formative round:** observe a small, diverse set of participants complete real tasks; use findings to repair comprehension and barriers, not claim statistical representativeness. **Comparative round:** compare the new flow with a documented baseline; recruit adequately across relevant user cohorts; report sample sizes and uncertainty; do not conceal a bad subgroup result behind an aggregate.

Use randomized/counterbalanced order when practical. Don't use a synthetic prototype to claim production price correctness, live-inventory quality, actual destination accessibility, or post-trip wellbeing.

## 9. Proposed benchmark ladder

All figures below are **proposed acceptance targets**, not measured baselines, external industry facts or demonstrated results. Establish baseline performance in round one and revise targets transparently if evidence shows they are unsafe or unsuitable.

### Gate 0 — Integrity and truthfulness
- 100% of synthetic offers are explicitly labelled synthetic.
- 100% of offers with missing material facts display a named unknown state rather than an inferred positive claim.
- 100% of booking summaries show the known total, currency, mandatory fees and key cancellation terms before commitment.
- Zero automatic bookings, silent edits to confirmed plans, false-urgency prompts or non-dismissable recommendations in the prototype.
- Zero known hard-infeasible destination options presented as feasible by the coupled decision layer.

### Gate 1 — Digital accessibility
- WCAG 2.2 A/AA audit of all in-scope paths: zero unresolved critical/serious defects at release, with an explicit exception log for lower-severity issues.
- Manual keyboard-only and screen-reader runs cover every critical task; no critical task is blocked.
- Test at 200% zoom, narrow viewports, reduced motion and low-bandwidth conditions.
- Automated scanning is supplementary; it cannot alone pass this gate.
- This is a programme target, not a certification or legal determination.

### Gate 2 — Core usability
- At least 90% task completion overall for the preregistered critical tasks in the comparative round.
- At least 85% task completion in each adequately sampled priority cohort, with no severe critical-task blocker; show confidence intervals and sample sizes.
- Median task ease at least 5.5/7, and system usability score at least 80/100 as initial hypotheses; interpret alongside task success, qualitative evidence and subgroup outcomes.
- At least 20% relative reduction in reported avoidable planning effort versus the baseline, with task success non-inferior.
- If the baseline is already better, the new interface must not regress to reach an arbitrary improvement target.

### Gate 3 — Decision quality and autonomy
- 100% of tested users can locate the total-cost basis, cancellation conditions and known accessibility gaps before the simulated commit.
- 100% of recommendations can be edited or dismissed; every recommendation includes a user-understandable reason.
- Zero confirmed-plan changes without explicit user action.
- At least 90% correctly identify which offer attributes are unknown in controlled comprehension checks.
- No mandatory account for browse, compare, draft itinerary or export in the prototype.

### Gate 4 — Disruption resilience
- At least 85% of participants complete a preregistered closure/cancelled-activity recovery task unaided.
- Median time to identify a viable option at or below 90 seconds in the prototype, while preserving access to the original plan and showing extra cost/time.
- Zero unverified substitutions represented as confirmed; zero loss of original booking state in tested scenarios.
- Include at least one “rest / skip / end the day” option when appropriate, not only replacement sales.

### Gate 5 — Whole-trip validity
- No claim that a holiday was better, safer, more accessible, more sustainable or more restorative until the corresponding real-world evidence is collected.
- Pilot: voluntary post-trip reflection, planned-versus-actual effort, undisclosed charges, access information accuracy, disruption recovery, local benefits and ecological constraints.
- Define a context-specific success threshold with travellers and local stakeholders before the pilot. Do not manufacture a universal wellbeing threshold.
- Retain null/adverse findings and compare planned intent against voluntary outcomes with explicit limitations.

## 10. Next execution sequence

1. Review this contract with disabled travellers, families/groups, budget-conscious travellers, older travellers and trip operators; compensate participants.
2. Produce the static synthetic prototype with the seven essential states above.
3. Run keyboard/screen-reader and manual accessibility tests before comparative usability testing.
4. Run the baseline comparison using preregistered tasks; change only the tested design variables.
5. Add tested outcomes to the JSON gates as versioned evidence receipts; never change a target silently after seeing results.
6. Only then consider inventory, maps, supplier integrations or a destination-twin connection. Each integration needs its own provenance, freshness, failure-mode and authority contract.

## 11. Research grounding

- **UN Tourism, Accessibility Tourism publications and guidelines:** accessible information and services should support people before, during and after travel; 2025 tourism-business guidance frames accessibility as a whole-service concern. https://www.untourism.int/accessibility-tourism-publications-guidelines
- **ISO 21902:2021, Accessible tourism for all:** requirements and recommendations spanning policy, infrastructure, products, services and stakeholders across the tourism supply chain. https://www.iso.org/standard/72126.html
- **W3C, WCAG 2.2:** in-scope digital interaction requirements and additional criteria including unobscured focus, dragging alternatives, target size, consistent help, redundant entry and accessible authentication. https://www.w3.org/TR/WCAG22/
- **OECD, Tourism Trends and Policies 2026:** tourism resilience, accessible participation, destination management, community benefit, visitor-flow management and data/digitalisation. https://www.oecd.org/en/publications/oecd-tourism-trends-and-policies-2026_3fd3cd75-en.html
- **OECD 2026, Enhancing the social benefits of tourism:** the need to share tourism benefits more widely while managing pressure on local infrastructure, services and residents. https://www.oecd.org/en/publications/oecd-tourism-trends-and-policies-2026_3fd3cd75-en/full-report/enhancing-the-social-benefits-of-tourism_43631a73.html

These sources motivate the requirements; they do not validate this design, set the proposed numeric thresholds, or prove that a prototype improves a real holiday.

## 12. Non-goals

- a universal OTA, live booking marketplace, payment processor or supplier-rating authority;
- maximizing click-through, activity density, spending, app engagement or surveillance;
- one universal happiness, joy, wellbeing, destination-quality or sustainability score;
- inferring emotion or disability from behavior;
- treating a generic accessibility badge as proof of an accessible journey;
- treating a realistic interface or successful simulation as field validation;
- automated physical operations, safety decisions, legal approval, supplier certification or community consent.
