# ACCESS-HOME-001 — Resident-led accessible home planning

**Status:** Draft requirements and prototype specification. This is not an architectural plan, medical assessment, code determination, accessibility certification or safety approval.

## Why we should change the design question

The objective is not to make a generic house that happens to contain some accessibility products. It is to help residents articulate what they need to do, where they encounter barriers, which adaptations fit their routines, and what still requires field measurement or professional judgment.

Universal design gives a useful baseline, but no "universal" solution makes individualized co-design unnecessary. People who are blind, have low vision, are Deaf or hard of hearing, are deafblind, use mobility aids, experience pain/fatigue, have cognitive or sensory access needs, or combine these experiences should not be treated as interchangeable personas.

Start with tasks:
- enter and leave the home with groceries or equipment;
- find rooms, controls, storage and essential items;
- make food and use appliances;
- bathe, toilet, dress and reach the bedroom at night;
- communicate with visitors and receive notifications;
- recognize alerts and follow a personally workable emergency plan;
- rest, work, use outdoor spaces and maintain independence;
- operate essential functions during power, network or device failure.

The design workflow maps resident-selected priorities to rooms, tasks, likely interventions, unresolved evidence and the people qualified to review each question. It must retain all intersections and must never infer medical status.

## Paradigm shifts

1. **Diagnosis-first → task-and-preference-first.** A label is not an adequate specification. Ask what a person wants to do, what equipment they use, what methods they prefer, and which barriers matter to them.
2. **One-time accommodation → adaptable home.** Plan early for reachable controls, clear paths, adaptable reinforcement, changing mobility needs and replacement of products without major reconstruction where feasible.
3. **Feature checklist → whole task-chain design.** Evaluate approach, entrance, route, room, fixture/control, activity, maintenance and emergency operation together.
4. **Single sensory channel → resident-chosen redundancy.** Visual, auditory, tactile, vibration, accessible text and physical controls should be selected by the resident and matched to the signal. A phone alert must not be assumed to replace approved life-safety equipment.
5. **Smart-home dependence → assistive technology with fallback.** Automation can help but should not make independent use contingent on connectivity, a cloud account, phone, charging or an inaccessible interface.
6. **Static "accessible" badge → traceable evidence state.** Every proposal is unverified until appropriately measured, tried, documented and reviewed. Profile selection cannot verify compliance.
7. **Family proxy → resident authority.** Carers/supporters may provide expertise if invited; they do not replace resident consent or preferences.
8. **Checklist pass → measured post-occupancy tasks.** Validate whether residents can actually perform chosen routines and recover from failures, with consent and in their own words.

## Resident-defined profile dimensions

The prototype allows overlapping selections for blind/low vision; Deaf/hard of hearing; mobility aids and maneuvering; dexterity/reach/strength; pain/fatigue/variable energy; sensory environment; cognitive/memory/processing; deafblind or multiple sensory needs; and support-person/service-animal arrangements.

These are optional design dimensions, not diagnoses. The initial prototype stores nothing beyond in-memory interaction state and uses no real resident data.

## Room and task prompts

### Whole-home baseline

- Site approach, exterior route, entrance, threshold, landing, door operation and weather exposure.
- Clear paths between rooms with realistic furniture, equipment, doors and storage.
- Bathroom and night-time bedroom route as a complete task.
- Controls, handles, taps, switches, appliances and essential utilities matched to resident-preferred reach, force, feedback and language.
- Resident-specific life-safety review covering appropriate alarms, signaling, exits, backup, maintenance and local rules.

### Blind / low vision

Potential strategies include consistent room/object placement, resident-selected tactile landmarks and labels, usable appliance controls, contrast where useful, glare control, lighting choices for low vision, accessible text/audio where wanted and removal of protrusions or trip hazards. No one cue is right for every blind or low-vision resident. Lighting alone is not a solution for blindness.

### Deaf / hard of hearing

Co-design communication space, sightlines, seating, lighting on faces and preferred communication modalities. Review visitor/doorbell and important-notification pathways. Choose visual, tactile/vibratory or suitable audible alert methods as appropriate. Life-safety devices must be suitable/approved for the jurisdiction and installed/tested by qualified people; generic phone notifications are not a safety specification.

### Deafblind / multiple sensory needs

Do not treat this as a simple combination of two generic profiles. Work with the resident and specialists selected by the resident to choose meaningful tactile orientation, communication and alert modalities; check range, location, touch safety, meaning, backup and maintenance across the entire home.

### Mobility, dexterity and fatigue

Plan measured circulation, turning and transfer envelopes with the person's actual equipment and helpers if wanted. Co-design bathroom transfer methods, reachable kitchen work zones, storage, door operation, surfaces and task staging. Include seated alternatives and rest points where the resident finds them useful. Don't copy a typical dimension into a building plan without checking the applicable standard, body/equipment envelope and site.

### Sensory, cognitive and support needs

Offer adjustable rather than universal lighting/sound assumptions, quiet or low-stimulation options, consistent organization, readable cues, accessible controls and user-chosen reminders. Balance acoustics and alerts. Plan support-person or animal space only as chosen by the resident, preserving privacy and autonomy.

## Design tensions to flag explicitly

- Visual contrast/lighting versus glare, reflection or sensory overload.
- Tactile/floor-level navigation cues versus cane, wheel, balance and trip hazards.
- Acoustic separation/quiet zones versus communication and notification.
- Visual, auditory and tactile alerts versus the particular signal, resident preferences, sleep and approved life-safety requirements.
- Automation versus offline/manual use, privacy, repairability and backup power.
- Support/transfer space versus the resident's privacy and autonomy.
- Step-free transitions versus local drainage, weatherproofing, thresholds and structure.

A conflict prompts co-design, mockups and appropriate professional review; the software must not automatically choose a winner.

## Proposed benchmarks — targets, not results

- **Traceability:** 100% of displayed suggestions identify their room/task, rationale, evidence question and reviewer role.
- **Truthfulness:** zero unverified items displayed as verified; zero profile selection paths that award a compliance state.
- **Preference retention:** 100% of resident-selected mandatory priorities are either retained in the draft or accompanied by an explicit unresolved conflict and next action.
- **Universal baseline:** all configurations retain entry/access, clear circulation, bathroom/night routine and life-safety review prompts.
- **Hard-gate behavior:** unresolved life-safety, egress or mandatory professional review blocks any later "ready for construction" state.
- **Fallback:** 100% of relevant connected/automated proposals have questions about manual fallback, power/network failure, privacy, maintenance and offline usability.
- **Digital accessibility:** manual and automated WCAG 2.2 AA testing for the planner, including keyboard, screen reader, zoom/reflow, focus, contrast and touch targets. Automated checks alone are insufficient.
- **Co-design usability:** define task success with disabled residents before tests; report per-cohort completion and critical blockers rather than hiding a blocked group in an average.
- **Post-occupancy:** with resident consent, test real selected tasks and failure scenarios; report unresolved barriers and adverse findings. Do not claim the tool itself produced a safe or accessible home.

Numeric architecture, clearances, slopes, forces, alarm behavior and legal criteria must bind to the applicable jurisdiction/standard edition and be evaluated by responsible professionals. The app intentionally does not invent global dimensions or numeric compliance thresholds.

## Architecture

- **Leptos prototype:** need selection, transparent task prompts, tensions and copy-ready brief only.
- **Symtropy housing evidence work (draft PR #1596):** evidence contracts, jurisdiction source registry and layout constraints. It does not itself convert a verified artifact digest into engineering safety or statutory approval.
- **Symtropy digital-twin / layout work:** later, only components with qualified geometry/physics scopes can be used for scenario comparison.
- **Professionals/authorities:** retain responsibility for architecture, structure, fire, electrical systems, alarms, local code and approvals.
- **Residents and paid co-designers:** define needs, preferences, acceptable compromises and post-occupancy tasks.

No production Python is introduced; the UI/model is Rust and the workflow uses Python only to parse-check the versioned JSON contract.

## Research grounding

- ISO 21542:2021, Building construction — Accessibility and usability of the built environment: https://www.iso.org/standard/71860.html. ISO lists the edition as under review in 2026; verify current applicable status before professional use.
- South Africa, National Strategic Framework on Universal Design and Access (Government Gazette notice): https://www.gov.za/sites/default/files/gcis_document/202110/45328gen606.pdf.
- RNIB, Adapting your home for blindness/low vision: https://www.rnib.org.uk/living-with-sight-loss/independent-living/adapting-your-home/.
- RNID, doorbells for Deaf/hard-of-hearing people: https://rnid.org.uk/information-and-support/technology-and-products/doorbells/.
- RNID, smoke alarm systems and sensory alerts: https://rnid.org.uk/information-and-support/technology-and-products/smoke-alarm-systems/.
- Deafblind UK, living with deafblindness and individual variation: https://deafblind.org.uk/get_support/living-with-deafblindness/.
- The UD Project, Residential Universal Design Specifications (2026), covering routes, entrances, circulation, kitchens, bathrooms and home systems: https://universaldesign.org/specifications. Use as a discovery aid, not a replacement for local codes or a licensed design.

These sources justify design dimensions and questions; they do not validate this software or any building.

## Non-goals

- medical diagnosis, treatment advice or inferred medical profiling;
- universal "best home" plan or one-size-fits-all dimensions;
- building-code engine, permit set, engineering calculations or accessibility certification;
- automatic recommendations presented as consent or a resident decision;
- alarm, egress, structural or electrical approval;
- supplier ranking or assistive-product shopping funnel;
- persistent personal data collection in the prototype.

## Exit criterion

A reviewer can select intersecting functional requirements, retain a resident-owned task brief, see the reason and evidence required for each proposed item, find conflicts, and understand which work remains for site measurements, disabled co-design partners and professionals. No profile selection or successful software test is represented as a building pass.
