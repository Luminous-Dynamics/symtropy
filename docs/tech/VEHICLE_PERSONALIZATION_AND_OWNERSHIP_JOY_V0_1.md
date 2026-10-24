# Vehicle Personalization and Joy of Ownership — v0.1

**Status:** Product/engineering research contract with synthetic profiles and adversarial controls. No production in-car interface, human-subjects outcome, accessibility certification or road-safety claim is implied.  
**Date:** 2026-10-17  
**Working thesis:** a vehicle should feel personally fitted to the people who use it while remaining predictable, serviceable, privacy-respecting and safe.

## 1. Paradigm shift: from trim packages to an owner-controlled experience

Cosmetic personalization is only one layer of ownership. A genuinely personal vehicle should adapt to how people see, hear, reach, sit, travel, share, repair and recover—without forcing account lock-in, cloud dependence, irreversible changes or unsafe in-motion interaction.

Treat personalization as an **owner-owned configuration contract** spanning five layers:

1. **Non-negotiable safety envelope.** Steering, braking, throttle/torque maps, restraint behavior, warning priority, ADAS safety limits, vehicle stability logic, diagnostic protections and safety interlocks are outside a user profile. They require their own engineering change-control and qualification process; an owner profile cannot override them.
2. **Reversible preferences.** Display theme/contrast/text scale, reduced motion, optional captions, audio and cue preferences, cabin ambience within validated limits, and references to supported seat/climate presets.
3. **Accessibility by default.** Larger controls, high contrast, tactile feedback, optional voice assistance, simplified moving UI and honest explanation when a vehicle lacks a requested capability.
4. **Household identity and privacy.** Separate private/shared/guest sessions; local encrypted storage by default; explicit opt-in for cloud sync/telemetry; user export, deletion and neutral reset.
5. **Repairable personalization.** Open profile format, replaceable user-facing modules, continued basic operation offline, no subscription wall around stored preferences, and no loss of owner settings when a supported module is replaced.

The product goal is not “maximum options.” It is **high personal fit with low effort, low distraction and low regret**.

## 2. Research grounding and caveats

The evidence argues for measured design—not the assumption that any user-designed interface is superior.

- A 2024 comparison of three driver-dashboard concepts found that participatory-design outcomes did not universally outperform a professionally developed user-centered design concept on task efficiency and distraction. Test the resulting interface; do not count workshops or number of customization choices as success. [Study: driver-designed dashboard interfaces](https://www.sciencedirect.com/science/article/pii/S0925753524000729)
- A 2024 Human Factors study of in-vehicle infotainment interactions reports changes in driving-performance, fixation and workload measures. Personalization must be evaluated against a stable moving-task baseline, rather than judged by visual appeal alone. [Study: HMI, infotainment and distraction](https://onlinelibrary.wiley.com/doi/10.1002/hfm.21049)
- ISO/TS 21054:2020 describes accessible control-design principles intended for a broad range of user needs and capabilities. It is a useful design reference, not evidence that these draft profiles conform or that an automotive product is certified. [ISO/TS 21054](https://www.iso.org/standard/69765.html)
- A 2026 stakeholder-centered repairability study found that assessments differ by perspective (consumer, maintenance team, design team), supporting separate owner, workshop and engineering views rather than a single universal score. [Stakeholder-centered repairability index](https://www.sciencedirect.com/science/article/pii/S2666789426000255)
- Automotive repair-data literature describes tension between access needed for independent repair and privacy/security. Export and diagnostic access need deliberate permission boundaries, not indiscriminate sharing. [Automotive repair data access, privacy and security](https://www.sciencedirect.com/science/article/pii/S0160791X22002317)

These sources support constraints and study design. They do not validate our particular profile schema, proposed targets or product concept.

## 3. What the first profile contract contains

The public fixture intentionally uses synthetic labels and opaque sample identifiers rather than names, photos, biometrics, trip histories, location or medical/disability diagnoses.

- **Everyday calm:** low-glare presentation, modest text enlargement, subdued lighting/audio and reduced motion.
- **High contrast and larger controls:** stronger contrast, 1.3 text scale, larger controls, optional captions and tactile/voice alternatives.
- **Shared household:** a separate shared-but-local preference bundle, not a blended profile that erases personal choices.
- **Guest session:** no cloud sync or telemetry, no persistent guest preference trail, clear on session end and short/zero retention.

A profile expresses preferences; it is not permission to actuate hardware. Values are applied only if the vehicle supports the feature, the setting lies inside its approved operating range and the context permits the change. Unsupported preferences are preserved but not applied, with an explanation. Cabin, seating and profile changes in this contract are parked-only. While moving, the interface remains simplified and predictable; profile editing is blocked.

The v0.1 schema deliberately rejects unknown profile fields. This prevents unreviewed feature creep such as a steering-response field from quietly becoming a “personalization option.” A future signed engineering configuration system may exist separately, but it cannot be smuggled in through these owner preferences.

## 4. Personalization that feels good in ordinary life

The experience should remove tiny repeated frictions, not add a new hobby of configuring the car.

### Before a trip

A driver selects a stable profile while parked. The vehicle restores supported preferences predictably: display legibility, seat/climate reference where hardware supports it, preferred audio mode, tactile/voice options and cabin lighting. The driver sees which options are unsupported instead of experiencing silent failure.

### During a trip

Keep the moving interface quiet and consistent. Essential controls, status, warnings and manual overrides stay in fixed, familiar places. Do not animate a dashboard or rearrange controls because the system guesses that the driver is bored, upset, distracted or stressed. Any future context adaptation should be opt-in, bounded, explainable and evaluated against the same distraction/non-inferiority gates. No emotion or health inference is required for v0.1.

### When drivers share a vehicle

Profiles switch only while parked or in a validated non-driving context. A private profile must not leak recent destinations, search history, audio history, saved preferences or telemetry to the next driver. A guest session returns to a clean default. A shared household profile must be explicitly shared; it is not the default place for everyone’s personal data.

### When something breaks or gets replaced

The owner can export the profile in a documented, versioned JSON format, restore a neutral profile and replace a supported user-facing module without losing the rest of the settings. Feature-specific settings remain dormant if a replacement module lacks that capability. No subscription or vendor server should be required to retrieve locally stored preferences.

## 5. Metrics: measure fit, delight and the cost of complexity separately

Do not compress the whole experience into one “joy score.” Report each dimension, its distribution and uncertainty:

- **Personal fit:** “This vehicle's setup fits how I prefer to use it.”
- **Predictability/control:** drivers understand what will happen and can return to a known default.
- **Comfort:** cabin/interface setup feels suitable during the evaluated task.
- **Delight/affection:** moments of positive surprise or pleasure in ordinary ownership.
- **Effort and annoyance:** configuration time, extra steps, unwanted prompts, errors, and “the car fought me” reports.
- **Cognitive workload/distraction:** task time, errors, glance behavior, reaction/performance measures and workload for moving tasks where appropriate.
- **Accessibility task completion:** completion and error rate reported independently for each profile and participant group.
- **Ownership autonomy:** export, deletion, reset, offline use, replaceability and freedom from mandatory cloud/account dependence.
- **Serviceability:** time and tools to restore defaults, migrate profiles and replace/diagnose supported personalization modules; parts availability and repair documentation.

Use a short repeated-measures instrument after relevant tasks, with the exact questions and analysis preregistered. Satisfaction is self-report; task accuracy and distraction are distinct evidence and must not be inferred from satisfaction.

## 6. Proposed v0.1 acceptance targets (hypotheses, not achieved results)

| Area | Proposed target | Guard against |
|---|---:|---|
| Prohibited safety-critical profile overrides | 0 accepted | Reject every unknown/non-allowed safety setting |
| Cloud sync without explicit profile-level consent | 0 accepted | No bundled or implied consent |
| Cross-profile/guest data leakage in adversarial tests | 0 cases | Shared device is not shared identity |
| Export/delete/neutral reset | 100% contract-test success | Owner must not need vendor approval |
| Essential setup while parked | Median under 3 minutes after initial onboarding | Do not optimize for experts only |
| Restore a neutral default | Under 60 seconds | No hidden menus or cloud dependency |
| Per-profile accessibility task success | At least 90% on the preregistered task suite for each tested group | Never hide one failing group behind an average |
| Personal fit / perceived control / delight | Median at least 4/5 each among pilot users | Publish sample, spread and selection limits |
| Annoyance/friction | Median no more than 2/5 | Report separately from delight |
| Moving-task safety non-inferiority | No pre-specified degradation beyond a justified margin in safety-relevant errors, glances, performance and workload | Set the margin and power plan before data collection |
| Offline operation of core profile functions | 100% of declared offline-capable tasks | No mandatory account for core preferences |
| Profile data recovery after supported module replacement | 100% test success | No profile loss because a decorative module fails |

The 4/5, 2/5 and time thresholds are product hypotheses for study planning, not standards or evidence-backed guarantees. The distraction non-inferiority margin must be selected with a human-factors specialist and test design before recruitment; do not invent a universal acceptable glance-time penalty.

## 7. Test protocol

1. **Contract tests first.** Validate four synthetic profiles and eight adversarial mutations. Attempt forbidden control tuning, moving-UI expansion, cloud sync without consent, telemetry without consent, guest-data retention, excessive ambience values and irreversible profile settings.
2. **Parked usability study.** Compare a stable default profile with profile customization for representative tasks. Record setup time, help requests, successful completion, errors, preference restoration, profile handover and reset.
3. **Accessible design evaluation.** Recruit people with diverse visual, hearing, reach, dexterity and cognitive preferences with appropriate consent. Report each participant group separately; do not require a person to disclose a diagnosis to use a preference or feature.
4. **Moving-task simulator study.** Only after parked usability and safety review; compare baseline vs personalized moving interface with matched tasks. Measure glance behavior, task errors, performance and workload. Do not begin with experimental variants on public roads.
5. **Repair/restore exercise.** Swap a replaceable user-facing module on controlled equipment, restore from the open export, verify capability negotiation and ensure no safety-critical calibration is exposed.
6. **Privacy and guest-session exercise.** Inspect storage, logs, exports, guest-session cleanup, consent transition, profile deletion, offline behavior and attempted cross-profile reads.
7. **Held-out confirmation.** Freeze task order, hypotheses, sample/power plan, exclusions, thresholds and analysis before held-out evaluation. Preserve failures and report uncertainty, task learning, individual differences and adverse experiences.

A small formative pilot is for finding usability defects, not claiming population-level success. A confirmatory study needs a justified sample-size plan and independent review.

## 8. Architecture implications

- Keep the personalization data model versioned and open; validate it strictly before any application.
- Separate preference storage from actuator authority. The profile can request “warm amber at low intensity”; only a capability-aware, context-safe controller decides whether the request is applicable.
- Treat safety-critical controls, warnings, ADAS limits and vehicle dynamics as a distinct and non-overridable authority domain.
- Default to local encrypted storage, explicit per-profile opt-in for cloud/telemetry, documented retention, export/delete, and clean guest sessions.
- Expose clear feature support and fallback behavior. Never imply that the vehicle has a capability it does not have.
- Keep profile state independent of display or ambience modules, so replacing the screen/light/audio module does not destroy user data.
- Keep a human-accessible manual override and neutral default. Avoid a design that forces voice, touch or a connected service as the sole path for an essential operation.
- Log only what is needed for diagnostics and the chosen study; study telemetry requires consent and must be separable from normal vehicle operation.

## 9. Implementation artifacts and claim boundary

- Profile contract and eight negative controls: schemas/transport/vehicle_personalization_profile_set_v0_1.schema.json and fixtures/personal_vehicle/vehicle_personalization_profiles_v0_1.json.
- Standalone Rust schema/semantic auditor with mutation tests: tools/vehicle-personalization-audit/.
- Synthetic vehicle capability manifest contract and parked/moving fixtures: schemas/transport/vehicle_personalization_capability_set_v0_1.schema.json and fixtures/personal_vehicle/vehicle_personalization_capability_sets_v0_1.json.
- Capability-aware preview compiler: tools/vehicle-personalization-plan/. It generates a deterministic preview for four synthetic profiles across parked/moving contexts, defers changes while moving, preserves unsupported preferences with reasons, and never calls hardware or vehicle controls.
- Focused workflows: .github/workflows/vehicle-personalization-audit.yml and .github/workflows/vehicle-personalization-plan.yml.
- Design and human-factors research tracker: VEHICLE-USER-FIT-001 (https://github.com/Luminous-Dynamics/symtropy/issues/1801).



## 10. Capability-aware preview compiler

The companion planner takes one owner profile and a versioned capability manifest, then emits a preview plan. It uses explicit dispositions:

- **preview_apply_if_owner_confirms** — the requested preference is within declared capabilities, and the current context is parked.
- **preserve_unapplied** — a capability is absent or a requested value exceeds a declared limit. The original preference remains intact and the plan explains why it cannot be applied; no silent clamp or false capability claim is permitted.
- **defer_until_parked** — profile changes are requested while the vehicle is in the moving context.
- **not_requested** — the profile does not define a value for that feature; the planner invents no preference.

The preview reports zero hardware/vehicle-control calls. It is not an actuator driver. The synthetic demo deliberately lacks seat memory, limits supported text scale to 1.15 and ambient intensity to 0.15, so the accessibility and calm profiles exercise explainable capability mismatches. Those are illustrative fixture values, not claims about a production vehicle.

A real implementation must independently authenticate the active driver, resolve exact hardware/software configuration, enforce manufacturer safety constraints, persist preferences only with explicit authority, and get confirmation from the user before applying a preview. It must fail closed on stale/unknown capabilities, incompatible firmware, or safety-policy changes. The planner alone is not sufficient authorization to write vehicle settings.

The contract is an input to future product engineering, not a running in-car personalization system. The auditor can prove that the listed synthetic profile contract and its seeded invalid cases obey the rules encoded here. It cannot demonstrate enjoyment, comfort, accessibility, distraction reduction, privacy security in deployed hardware, legal compliance or vehicle safety. Those require implementation review, real tests and suitable evidence.
