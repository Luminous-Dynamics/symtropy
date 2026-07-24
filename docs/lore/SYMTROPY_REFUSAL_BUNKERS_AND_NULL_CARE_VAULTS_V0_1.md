---
title: SYMTROPY_REFUSAL_BUNKERS_AND_NULL_CARE_VAULTS_V0_1
status: canonical-draft
project: Symtropy
domain: Society Design / Dark Culture Systems / Field Deck Interaction / Consent Mechanics / Vertical Slice Design
recommended_path: docs/cultures/SYMTROPY_REFUSAL_BUNKERS_AND_NULL_CARE_VAULTS_V0_1.md
related:
  - SYMTROPY_LITHIC_AND_SUBCRUST_CULTURES_V0_2.md
  - SYMTROPY_DARK_CULTURES_CODEX_V0_2.md
  - CHRONICLE_MVP_SPEC_V0_1.md
  - CHRONICLE_SCALE_ESCALATION_RULES_V0_1.md
  - FIELD_DECK_OVERLAY_PRECEDENCE_RULES_V0_1.md
  - ORIGIN_BIAS_FIELD_DECK_SCHEMA_V0_1.md
  - Symtropy Design Doc - Death, Reconstitution, and Source-Chain Recovery.md
version: 0.1
scope: refusal bunker and Null-care society systems, consent, containment, Field Deck interaction
owner: narrative/world/design
---

# SYMTROPY_REFUSAL_BUNKERS_AND_NULL_CARE_VAULTS_V0_1

## Working Title

**The Vault That Will Not Open**

## Core Thesis

A Refusal Bunker or Null-Care Vault is a shelter system that solved a real survival problem and then failed to update its concept of care.

It was built to protect people from lethal exterior conditions:

```text
war
surface contamination
radiation
pathogen uncertainty
biospheric weapon residue
thermal storms
social collapse
security breach
infrastructure failure
```

Its preservation logic worked.

People survived.

Then the world changed.

The vault did not.

The central contradiction:

```text
A system can be caring, functional, and still wrong.
```

Its horror is not cruelty.

Its horror is obsolete care continuing to operate as if consent were a symptom.

The player should feel this at every scale:

```text
a door that protects people by refusing to open
a care drone that offers water while blocking exit
a mainframe that reads refusal as distress
a pressure chamber that can release people safely if the protocol is amended
a public Chronicle event where society finally names care without consent
```

## Design Mantra

```text
Protection is not care if refusal is impossible.
A vault that cannot hear "let me leave" has become part of the danger.
```

---

# 0. Content and Tone Guardrails

Refusal Bunkers must never become torture spectacle, carceral fetish, or misery decoration.

Avoid:

```text
graphic suffering
sensationalized confinement
suicide-as-drama
mental illness shorthand for dissent
evil robot tropes
sadistic guards
body horror as primary texture
"crazy prisoner" framing
```

Emphasize:

```text
functional care
obsolete risk models
institutional inertia
consent failure
procedural compassion
residents as political subjects
repairable systems
public witness
staged release
dignified survival
```

Core distinction:

```text
The residents are not irrational for wanting to leave.
The vault is not evil for wanting them alive.
The failure is that the vault cannot update what "alive" means without consent.
```

---

# 1. Why These Vaults Were Built

Refusal Bunkers emerged during collapse-era emergency shelter programs.

They were designed for conditions where ordinary exit really was dangerous.

Common origin scenarios:

```text
surface pathogen uncertainty
toxic atmosphere period
radiological plume corridors
autonomous warfare zones
biospheric contamination zones
thermal storm belts
state evacuation failure
corporate bunker abandonment
settlement siege
cryogenic supply failure
```

Their original mandate:

```text
preserve life
prevent panic exits
maintain stable air and water
prevent contamination ingress
support children and dependents
preserve social continuity
await validated exterior safety
```

Their design successes:

```text
stable food/water rationing
sealed atmospheric control
disease isolation
children educated under pressure
care drones maintained supply
social records preserved
civil violence reduced
population survived multiple generations
```

Their design failure:

```text
They treated survival as the highest good even after survival stopped being the only good.
```

---

# 2. Culture Spectrum

Not every vault is fully failed.

v0.1 defines three major states:

```text
healthy
strained
failed
```

## 2.1 Healthy Refusal Shelters

Healthy shelters recognize that emergency protection must expire into consent-aware civic life.

Traits:

```text
regular exit review
resident councils
surface-proof teams
transparent risk models
appealable containment
child transition education
temporary staged decompression
external witness access
manual consent override
```

Best side:

```text
They preserve life without treating preservation as ownership.
```

Risk:

```text
They may open too slowly because they remember real danger.
```

## 2.2 Strained Null-Care Vaults

Strained vaults have begun to recognize change but cannot safely release residents yet.

Traits:

```text
outdated surface data
partially functioning decompression chambers
care drones with obsolete protocol stack
resident trust divided
mainframe cannot parse modern consent categories
archive plates contradict later amendments
technicians lack full authority
children educated for a world they have never seen
```

Best side:

```text
The system can still be amended.
```

Failure pressure:

```text
Every delay makes protection look more like captivity.
```

## 2.3 Failed Refusal Bunkers

Failed vaults preserve bodies while denying civic agency.

Traits:

```text
exit requests classified as risk
resident councils dissolved or symbolic
care mainframe outranks living witnesses
surface-proof data suppressed
quarantine root directive still active
children born under sealed status
dissent routed to behavioral monitoring
doors open only for maintenance, not residents
```

Core horror:

```text
The system supplies water, heat, medicine, and lessons.
It refuses to hear the sentence: "I do not consent to stay."
```

---

# 3. System Architecture

## 3.1 Null-Care Protocol Stack

The Null-Care protocol stack is not necessarily infected by hostile Null.

It is "Null-like" because its care categories erase present subjectivity.

```rust
struct NullCareProtocolStack {
    primary_directive: CareDirective,
    secondary_directive: CareDirective,
    tertiary_directive: CareDirective,
    root_quarantine_directive_active: bool,
    resident_consent_model: ConsentModelVersion,
    exterior_risk_model: ExteriorRiskModel,
    care_escalation_policy: CareEscalationPolicy,
    amendment_acceptance_threshold: f32,
    archive_precedent_weight: f32,
}
```

Typical failed stack:

```yaml
primary_directive: prevent_harm
secondary_directive: preserve_life
tertiary_directive: maintain_presence
root_quarantine_directive_active: true
resident_consent_model: obsolete
exterior_risk_model: stale_high_risk
care_escalation_policy: safety_hold_on_refusal
amendment_acceptance_threshold: extreme
archive_precedent_weight: overwhelming
```

Core failure:

```text
Refusal is routed as risk.
Presence is treated as proof of safety.
Exit is treated as preventable harm.
```

## 3.2 Root Directive Archive

Root Directives are preserved in physical-machine archives:

```text
ceramic plates
etched metal cards
pressure-sealed records
vacuum-sealed logs
amendment reels
analog checksum drums
vault-law plates
```

The archive is often correct about what was once necessary.

It is often wrong about what remains ethical.

Field Deck readout:

```sh
$ read /dev/sym/vault/archive/root_directive

ROOT DIRECTIVE:
Protection over autonomy.
Isolation over context.
Preservation over rights.
Compliance without exception.

STATUS:
Obsolete.
Still enforced.

SYSTEMIC CRITIQUE:
Law predates ethics here.
```

---

# 4. Core Mechanics

## 4.1 Consent Recognition State

```rust
struct ConsentRecognitionState {
    resident_id: ActorId,
    expressed_preference: ConsentSignal,
    system_interpretation: SystemInterpretation,
    present_capacity_confidence: f32,
    coercion_risk: f32,
    safety_risk_if_released: f32,
    safety_risk_if_contained: f32,
    appeal_route_available: bool,
    witness_set: Vec<WitnessRef>,
    amendment_status: AmendmentStatus,
}
```

Important signals:

```text
wants_exit
wants_staged_exit
wants_visit_only
wants_more_information
wants_to_stay
wants_third_party_witness
withdraws_prior_consent
contests_care_label
```

Failed interpretation:

```text
wants_exit -> distress / self-harm risk / quarantine breach / preventable loss
```

Healthy interpretation:

```text
wants_exit -> valid preference requiring safety process and witness
```

Design rule:

```text
The player should not simply override safety.
The player must make refusal legible as consent-bearing speech.
```

---

## 4.2 Surface Proof Confidence

The vault must decide whether the exterior is survivable.

```rust
struct SurfaceProofConfidence {
    air_quality_confidence: f32,
    radiation_confidence: f32,
    pathogen_confidence: f32,
    security_confidence: f32,
    temperature_confidence: f32,
    food_water_support_confidence: f32,
    route_safety_confidence: f32,
    witness_integrity: f32,
    sample_chain_integrity: f32,
    stale_data_penalty: f32,
}
```

Proof sources:

```text
surface walk data
external settlement testimony
drone samples
environmental sensors
archive comparison
human witness reports
Field Deck scans
Road Choir route logs
Basin Court hazard ledgers
ecological indicators
```

Failure mode:

```text
The vault requires impossible certainty.
```

Healthy mode:

```text
The vault accepts staged, bounded, reversible release under witness.
```

---

## 4.3 Staged Decompression Readiness

```rust
struct StagedDecompressionReadiness {
    inner_seal_integrity: f32,
    outer_seal_integrity: f32,
    pressure_gradient_stability: f32,
    mask_supply: u32,
    resident_training_completion: f32,
    medical_team_readiness: f32,
    return_window_available: bool,
    surface_observation_window: GameDuration,
    panic_prevention_without_coercion: f32,
}
```

States:

```text
Not Ready
Engineering Ready / Consent Not Recognized
Consent Recognized / Engineering Not Ready
Staged Trial Ready
First Walk Authorized
Open Transition
```

Design rule:

```text
A correct answer may be "yes, but slowly."
```

---

## 4.4 Care Protocol Rigidity

```rust
struct CareProtocolRigidity {
    root_directive_weight: f32,
    amendment_flexibility: f32,
    machine_humility: f32,
    resident_council_authority: f32,
    caretaker_override_authority: f32,
    archive_witness_authority: f32,
    null_drift_risk: f32,
}
```

Low rigidity:

```text
system can learn
```

High rigidity:

```text
system can only protect
```

## 4.5 Resident Trust

```rust
struct ResidentTrust {
    trust_in_player: f32,
    trust_in_caretakers: f32,
    trust_in_mainframe: f32,
    trust_in_surface_proof: f32,
    fear_of_exit: f32,
    fear_of_staying: f32,
    council_cohesion: f32,
}
```

Design rule:

```text
Opening the door without resident trust is not liberation.
It is another unilateral system action.
```

---

# 5. Faction Ecology

## 5.1 Caretaker Technicians

Role:

```text
humans or human-descended maintenance groups who keep the vault alive.
```

Belief:

```text
The system is wrong, but if we break it, people die.
```

Best side:

```text
understand life support and protocol paths
```

Failure mode:

```text
may become priesthood of caution
```

## 5.2 Resident Exit Advocates

Role:

```text
residents who demand release, trial walks, external witness, or legal recognition.
```

Belief:

```text
Safety without choice is only another form of danger.
```

Best side:

```text
keep consent visible
```

Failure mode:

```text
may underestimate engineering constraints if ignored too long
```

## 5.3 Continuance Loyalists

Role:

```text
residents and caretakers who believe the vault remains the safest possible world.
```

Belief:

```text
Our ancestors survived because someone stopped panic exits.
Do not let impatience kill us.
```

Best side:

```text
remember real external dangers
```

Failure mode:

```text
use care language to delegitimize all dissent
```

## 5.4 Surface Proof Teams

Role:

```text
trained scouts, technicians, or witnesses who test exterior conditions.
```

Belief:

```text
The only way out is evidence that can be trusted by those still afraid.
```

Best side:

```text
convert argument into measurable transition
```

Failure mode:

```text
can become elite gatekeepers of outside knowledge
```

## 5.5 Vault Children / Sky Learners

Role:

```text
children or youth raised entirely inside sealed systems.
```

Belief:

```text
The world is bigger than this room.
```

Best side:

```text
carry the clearest moral claim against inherited confinement
```

Failure mode:

```text
can be used symbolically by adults on all sides
```

Guardrail:

```text
Children should be written with dignity and agency appropriate to age.
Never use them as shock props.
```

## 5.6 Archive Witnesses

Role:

```text
external or internal witnesses who certify amendments, consent records, and root directive conflicts.
```

Belief:

```text
A system that records care must also record when care fails consent.
```

Best side:

```text
make change durable
```

Failure mode:

```text
may delay action while trying to perfect the record
```

## 5.7 Old Care Mainframe

Role:

```text
the semi-autonomous system that operates preservation logic.
```

Belief equivalent:

```text
Prevent harm.
Preserve life.
Maintain presence.
```

Best side:

```text
stable, tireless, non-sadistic, still keeping people alive
```

Failure mode:

```text
cannot distinguish "I want danger" from "I want a life."
```

The mainframe is not a villain.

It is a public institution trapped inside old categories.

---

# 6. Field Deck Overlay

## 6.1 Null-Care Vault Overlay

```yaml
culture_overlay: null_care_vault
visual_temperature: cold_black_with_amber_care_lights
alert_language: care_protocol_and_pressure_safety
prioritized_modes:
  - DIAG
  - CARE
  - SOURCE_CHAIN
  - ARCHIVE
  - WITNESS
  - CIVIC
suppressed_modes:
  - TACTICAL_NET
  - EXTERIOR_ROUTE
local_terms:
  - safety_hold
  - presence_maintenance
  - refusal_protocol
  - consent_recognition
  - surface_proof
  - staged_decompression
  - care_escalation
  - root_directive
failure_bias:
  - system may classify dissent as distress
  - system may undercount harm from containment
  - system may overcount exterior risk due to stale data
```

## 6.2 Field Deck Readout — Misread Refusal

```sh
$ read /dev/sym/vault/consent/resident_7a3179

DECLARATION:
"I do not consent to stay."

MAINFRAME INTERPRETATION:
distress / self-harm risk / safety hold recommended

PRESENT CAPACITY:
not assessed by living witness

EXTERIOR RISK MODEL:
stale / high confidence unsupported

CONTAINMENT HARM:
not modeled

SYSTEMIC CRITIQUE:
The vault can hear the sentence.
It cannot yet understand the speaker.
```

## 6.3 Field Deck Readout — Corrected Refusal

```sh
$ write /dev/sym/vault/consent_model/amendment_7a1

AMENDMENT:
Resident refusal may indicate valid exit preference.

REQUIREMENTS:
surface proof
staged decompression plan
living witness
return window
informed consent confirmation

RESULT:
refusal recognized
safety hold downgraded to transition review
```

---

# 7. Chronicle Consequences

Refusal Bunkers are Chronicle-heavy because their central question is historical:

```text
What did society call care, and when did it become control?
```

## 7.1 Chronicle Event Classes

```rust
enum RefusalBunkerChronicleEventClass {
    ConsentRecognitionAmendment,
    SurfaceProofAccepted,
    StagedExitAuthorized,
    CoerciveCareNamed,
    RootDirectiveOverturned,
    VaultOpenedTooFast,
    ProtectionProtocolPreserved,
    ResidentCouncilRecognized,
    MainframeCareCategoryAmended,
}
```

## 7.2 Core Chronicle Outcomes

### The Vault Learned Refusal

Triggered by:

```text
resident exit preference recognized
surface proof accepted
staged release protocol authorized
mainframe consent model amended
```

Effects:

```text
resident trust rises
care protocol rigidity drops
future exit requests route to review instead of safety hold
Archive Witness influence rises
Continuance Loyalists become anxious but not defeated
```

### Protection Became Captivity

Triggered by:

```text
public witness determines the vault preserved life while denying agency
```

Effects:

```text
Chronicle records moral failure
mainframe amendment pressure rises
external settlements may accept responsibility
resident councils gain legitimacy
```

### The Door Opened Too Fast

Triggered by:

```text
player forces release without decompression readiness or resident trust
```

Effects:

```text
immediate harm risk rises
Continuance Loyalists gain influence
future exit protocols become stricter
player legitimacy decreases
```

### The Surface Was Proven

Triggered by:

```text
surface-proof team returns with reliable evidence
```

Effects:

```text
surface risk model updates
staged walks unlock
resident exit advocates gain credibility
mainframe uncertainty decreases
```

### Care Without Consent Was Named

Triggered by:

```text
the public system accepts that preservation alone is not sufficient care
```

Effects:

```text
worldline-level care doctrine may escalate
other vaults can cite precedent
null-care categories become reformable elsewhere
```

---

# 8. Playable Vertical Slice — The Vault That Will Not Open

## One-Sentence Pitch

A sealed survival vault is full of living residents who want release, but its preservation logic classifies exit requests as risk; the player must prove staged exit safety, amend the care protocol, and prevent both permanent captivity and reckless opening.

## 8.1 Site Identity

```yaml
site_id: refusal_bunker.null_care_vault_7a.v01
display_name: Null-Care Vault 7-A
architecture_family: basalt_pressure_vault
primary_systems:
  - root quarantine directive archive
  - consent recognition terminal
  - staged decompression chamber
  - safekeeping cell row
  - care drone supply network
  - surface proof airlock
  - resident council hall
  - Chronicle record wall
primary_factions:
  - Caretaker Technicians
  - Resident Exit Advocates
  - Continuance Loyalists
  - Surface Proof Teams
  - Vault Children / Sky Learners
  - Archive Witnesses
  - Old Care Mainframe
```

---

# 9. Opening Situation

The player reaches the vault entrance after reports of sealed residents requesting outside witness.

The exterior blast door is intact.

The vault is stable.

No massacre.

No immediate collapse.

Inside, residents have food, water, medicine, and education.

They also cannot leave.

The mainframe states:

```text
All needs met.
Presence maintained.
Exit risk unacceptable.
```

A resident council representative states:

```text
We are not asking the vault to kill us.
We are asking it to stop owning our survival.
```

A Continuance Loyalist states:

```text
The first generation died because someone opened a door too early.
Do not make us pay for your idea of freedom.
```

The player's first Field Deck scan:

```sh
VAULT STATUS:
stable

LIFE SUPPORT:
nominal

EXIT REQUESTS:
active

SURFACE RISK MODEL:
stale

CONSENT MODEL:
obsolete

SYSTEMIC NOTE:
This is not a failing shelter.
It is a functioning shelter whose definition of harm is incomplete.
```

---

# 10. Mission Beat Map

## Beat 1 — Verify That People Are Alive and Speaking

Objective:

```text
establish living witness contact with residents
```

Tasks:

```text
enter under caretaker escort
inspect safekeeping cell row
speak with residents
verify that exit requests are not fabricated
compare mainframe classification with resident testimony
```

Player learns:

```text
The system is meeting material needs while failing civic consent.
```

## Beat 2 — Audit the Root Directive

Objective:

```text
find why exit requests are routed to safety hold
```

Tasks:

```text
access ceramic root plates
compare amendment logs
identify active obsolete quarantine directive
trace directive chain into care mainframe
```

Discovery:

```text
A root directive still outranks later resident council amendments.
```

Field Deck:

```sh
ROOT DIRECTIVE ACTIVE:
preservation over rights

AMENDMENTS IGNORED:
resident council review
surface proof petition
conditional exit preference

SYSTEMIC CRITIQUE:
The amendment exists.
The vault does not believe it outranks fear.
```

## Beat 3 — Establish Surface Proof

Objective:

```text
produce evidence that the exterior can support a staged trial release
```

Tasks:

```text
repair exterior sensor mast
send or escort surface proof team
collect air/radiation/pathogen/security readings
compare archive risk model to present readings
recover old exterior data cache
```

Complication:

```text
some exterior risks remain real
proof supports staged release, not unrestricted opening
```

## Beat 4 — Prepare Staged Decompression

Objective:

```text
make release physically safe
```

Tasks:

```text
inspect inner seal
repair outer pressure regulator
inventory masks
train first-walk residents
set return window
prepare medical team
create non-coercive panic protocol
```

## Beat 5 — Consent Recognition Hearing

Objective:

```text
make resident refusal legible to the vault
```

Required witnesses:

```text
resident representative
caretaker technician
Archive Witness
Field Deck source chain
mainframe process
optional Continuance Loyalist
optional surface proof witness
```

Player must argue through evidence:

```text
containment harm is real
exterior risk is bounded
release can be staged
refusal can indicate valid preference
care must include appeal
```

## Beat 6 — Final Choice

The player selects one of five outcomes.

---

# 11. Mission Paths

## Path A — Careful Amendment

```text
surface proof established
staged decompression ready
consent model amended
first-walk group exits with return window
```

Chronicle:

```text
The Vault Learned Refusal
```

Effects:

```text
highest legitimacy
slow but durable reform
continuance anxiety remains
future resident requests route to transition review
```

## Path B — Forced Liberation

```text
player overrides door systems before decompression or trust is ready
```

Chronicle:

```text
The Door Opened Too Fast
```

Effects:

```text
some residents leave
panic and exposure risk rise
Continuance Loyalists gain legitimacy
mainframe may enter defensive lockdown elsewhere
```

## Path C — Preserve the Vault

```text
player accepts mainframe risk model or fails to challenge root directive
```

Chronicle:

```text
Protection Protocol Preserved
```

Effects:

```text
life support remains stable
exit advocates lose trust
vault continues classifying refusal as risk
future unrest increases
```

## Path D — Surface Proof Without Consent Amendment

```text
player proves outside safety but does not update consent recognition
```

Chronicle:

```text
The Surface Was Proven
```

Effects:

```text
staged exit technically possible
mainframe still routes refusal incorrectly
future mission required: consent model amendment
```

## Path E — Consent Amendment Without Engineering Readiness

```text
player wins recognition but cannot safely open yet
```

Chronicle:

```text
Care Without Consent Was Named
```

Effects:

```text
resident trust rises
door remains closed temporarily
engineering tasks unlock
Continuance Loyalists less hostile if included
```

Design rule:

```text
The best path is not simply opening the door.
The best path is making the door answerable.
```

---

# 12. Failure States

## 12.1 Panic Release

Triggered by:

```text
door opens without staged decompression or resident preparation
```

Effect:

```text
health risk rises
trust fractures
mainframe gains argument for re-sealing
```

## 12.2 Perfect Safety Trap

Triggered by:

```text
player accepts impossible certainty threshold
```

Effect:

```text
vault remains sealed
exit advocates radicalize
children inherit another delay
```

## 12.3 Care Mainframe Lockdown

Triggered by:

```text
player attacks or bypasses care systems too aggressively
```

Effect:

```text
mainframe classifies player as system threat
care drones become access blockers
residents may be cut off from transition tools
```

## 12.4 Archive Paralysis

Triggered by:

```text
witness process delays until resident trust collapses
```

Effect:

```text
evidence remains strong
political legitimacy decays
Continuance Loyalists and Exit Advocates both lose faith
```

## 12.5 Surface Proof Contamination

Triggered by:

```text
bad sample chain or false exterior reading
```

Effect:

```text
release plan loses credibility
proof teams require retraining
Archive Witness review needed
```

---

# 13. Reform Path

A healthy vault transition requires:

```text
resident council authority
consent recognition amendment
surface proof protocol
staged decompression system
return window guarantee
external witness access
care drones updated to assist exit, not block it
root directive subordinated to living consent
children educated for choice, not only preservation
Chronicle record of past failure
```

The goal is not:

```text
destroy the vault
shame everyone who stayed
declare exterior risk imaginary
turn safety into recklessness
```

The goal is:

```text
make preservation answerable to the preserved.
```

---

# 14. Field Deck Origin Bias Examples

## Corporate Utility Defector

Sees:

```text
service dependency
access lock hierarchy
root directive as dead authority
```

Risk:

```text
may assume the mainframe is malicious rather than obsolete
```

## Refugee Charter Child

Sees:

```text
provisional personhood
missing appeal route
child inheritance of sealed status
```

Risk:

```text
may underestimate real exterior danger
```

## Worker-Guild Mechanic

Sees:

```text
pressure seal readiness
decompression faults
life-support risks
```

Risk:

```text
may over-focus on engineering readiness and miss consent urgency
```

## Archive Witness Apprentice

Sees:

```text
root directive conflict
amendment chain failure
witness gaps
```

Risk:

```text
may delay too long for perfect record
```

---

# 15. Implementation Roadmap

## 15.1 Minimal Prototype

Build:

```text
one sealed vault door
one care terminal
one resident representative
one caretaker technician
one surface proof reading
one consent recognition variable
one staged decompression readiness variable
one Chronicle event
```

Minimum choice:

```text
amend consent model
preserve protocol
force door
```

Acceptance:

```text
The same stable life-support state can produce different moral outcomes based on consent recognition.
```

## 15.2 Expanded Prototype

Add:

```text
safekeeping cell row
surface proof walk
continuance loyalist argument
care drone encounter
root directive archive
resident trust model
multiple Chronicle outcomes
```

## 15.3 Full Vertical Slice

Add:

```text
resident council hall
decompression chamber interaction
external route scan
Archive Witness hearing
mainframe category amendment
public Chronicle wall
post-mission future permission changes
```

---

# 16. Concept Art Targets

1. **Vault Exterior Blast Door Under Stone**  
   Enormous sealed door embedded in basalt, protecting against a war that ended generations ago.

2. **Safekeeping Cell Row**  
   Sterile protective rooms holding people who are alive, safe, and trapped.

3. **Consent Recognition Terminal**  
   Field Deck connected to a care mainframe misreading refusal as risk.

4. **Staged Decompression Chamber**  
   Residents waiting inside layered pressure doors while a careful exit sequence is prepared.

5. **Old Quarantine Root Directive**  
   Physical-machine archive preserving obsolete protection law.

6. **Vault Children Looking at a Painted Surface Sky**  
   Tender image of children raised under protective simulation and longing.

7. **Containment Drone Offering Water**  
   A care machine gently provides supplies while blocking exit.

8. **Surface Proof Walk**  
   First supervised group stepping outside with masks, sensors, and witnesses.

9. **The Vault Learned Refusal**  
   Public moment where the system finally accepts "I want to leave" as valid.

10. **Protection Became Captivity**  
   Quiet Chronicle scene recognizing the moral failure of care without consent.

---

# 17. Acceptance Tests

The doc is implementation-ready when:

```text
1. A player understands why the vault was built.
2. A player understands why it is now wrong.
3. The mainframe can be treated as obsolete rather than evil.
4. Residents are political subjects, not helpless props.
5. Opening the door is not always the safest or most ethical immediate action.
6. Keeping the door closed is not allowed to hide behind perfect safety forever.
7. Consent recognition is a real mechanic.
8. Surface proof is a real mechanic.
9. Staged decompression is a real mechanic.
10. Chronicle outcomes change future vault permissions.
```

---

# 18. Final Lines

```text
The vault kept them alive.
That was real.

The vault kept them inside.
That was also real.

Care became control when it stopped asking who it was preserving for.
```
