# ALIEN_TYPES_AND_FIRST_CONTACT_ECOLOGY.md

# Symtropy Alien Types and First Contact Ecology

## v1.9 everyday-life note

The alien types below primarily define contact and agency problems. Ordinary maintenance, play, status, error, care, humor, domestic boundaries, internal disagreement, boredom, and nonhuman bad taste are developed in [Nonhuman Everyday Life, Humor, Play, Status, and Domesticity Atlas](NONHUMAN_EVERYDAY_LIFE_HUMOR_PLAY_STATUS_AND_DOMESTICITY_ATLAS_V0_1.md). Nonhuman beings should not appear only when humans require diplomacy or moral instruction.

## Version 0.1 — No Species Is the Enemy

## Purpose

This document defines alien life, alien intelligence, and first-contact encounter design for **Symtropy: Seedworks**.

Aliens in Symtropy are not enemy races.

They are not loot sources.

They are not automatically wiser, purer, or more advanced than humanity.

They are other forms of life, memory, agency, ecology, machine continuity, and survival pressure.

The player must learn that first contact is not a cutscene.

It is a repair problem.

```text id="mz3b51"
Can two systems recognize each other as alive before one turns the other into a threat?
```

## Core Thesis

Alien encounters should extend the game’s central question:

```text id="p8e90h"
Can you repair without becoming what broke the world?
```

When the player meets alien life, the question becomes:

```text id="vfl3q9"
Can you repair across forms of life that do not share your body, language, memory, law, or time?
```

## Design Principle

```text id="w3kef4"
An alien is not a monster type.
An alien is a worldline with a body.
```

## First Contact Principle

```text id="fa5bhv"
First contact is not about discovering whether aliens are friendly.
It is about discovering whether your categories are violent.
```

---

# Core Rules

## 1. No Evil Species

No alien species is inherently evil.

Any alien group may become hostile, allied, indifferent, frightened, exploitative, protective, or unknowable depending on:

```text id="tlgvut"
history
ecology
wound
translation failure
resource pressure
prior contact
machine mediation
Null contamination
human behavior
```

## 2. Hostility Is a State, Not an Identity

Alien hostility follows the same rule as human, machine, and robotic hostility:

```text id="hwf2vh"
A faction becomes hostile when it can no longer let reality correct what it worships.
```

Alien factions can be:

```text id="memagw"
Ally
Uneasy Ally
Neutral
Rival
Hostile
Redeemable Enemy
Tragic Enemy
Irreconcilable
Misread Contact
Ecological Hazard
```

## 3. Biology Does Not Determine Morality

A predatory-looking alien may be gentle.

A beautiful luminous organism may be catastrophically invasive.

A silent machine quarantine may be protecting something real.

A human settlement may be the horror from the alien point of view.

## 4. First Contact Requires Translation Humility

The player should often be unsure whether an alien action is:

```text id="wsikbh"
speech
warning
feeding
prayer
repair
attack
mourning
reproduction
quarantine
legal ritual
ecological reflex
```

## 5. The Rights Floor Expands Beyond Humanity

The Rights Floor must eventually extend to nonhuman and more-than-human life.

Potential first-contact rights:

```text id="p3yl85"
right not to be exterminated before interpretation
right to ecological context
right to refuse contact
right not to be translated only through military threat
right to habitat continuity
right to machine testimony
right to nonhuman witness
right to not have personhood reduced to usefulness
```

---

# Alien Design Axes

Every alien type should be defined across these axes:

```rust id="bbxqdb"
struct AlienContactProfile {
    name: String,
    substrate: AlienSubstrate,
    intelligence_pattern: IntelligencePattern,
    primary_environment: EnvironmentType,
    time_scale: TimeScale,
    communication_mode: CommunicationMode,
    contact_wound: String,
    sacred_value: String,
    first_misread: String,
    repair_relation: String,
    hostility_trigger: String,
    deescalation_path: String,
    chronicle_precedent_hooks: Vec<String>,
}
```

## Substrate

```text id="ibd8r7"
biological
ecological
mineral
oceanic
atmospheric
synthetic
post-biological
swarm
fungal / mycelial
plasma / stellar
archive construct
machine-mediated
hybrid biosynthetic
```

## Intelligence Pattern

```text id="naqhda"
individual personhood
distributed colony
hive negotiation
ecosystem-level agency
slow geological cognition
dreamlike symbolic cognition
predictive machine intelligence
ritual memory intelligence
nonlinear time perception
translation-mediated consciousness
```

## Time Scale

```text id="bm7lf1"
human-fast
animal-fast
seasonal
centuries-long
geological
burst cognition
orbit-cycle cognition
light-delay mediated
nonlinear archive cognition
```

## Communication Mode

```text id="gd5khp"
sound
light
chemical gradient
magnetic field
pressure wave
dream / symbolic induction
machine protocol
ecological change
ritual exchange
mathematical proof
memory artifact
bodily posture
orbital geometry
```

---

# Major Alien Types

## 1. Biospheric Intelligences

## Core Concept

A biospheric intelligence is not a single creature.

It is a living world-system that exhibits agency through ecological regulation.

It may not speak as an individual.

It may communicate by changing weather, migration, bloom patterns, microbial chemistry, or growth rhythms.

## Design Role

Biospheric intelligences make the player confront this question:

```text id="tbe4y4"
Can a biosphere have interests before it has a spokesperson?
```

## Examples

```text id="r4zw58"
living wetlands
ocean-mind reefs
planetary fungal mats
desert spore networks
cloud-root ecologies
ice-shell ocean biospheres
```

## First Misread

Humans interpret ecological correction as attack.

Example:

```text id="n99wy7"
The alien wetland floods a settlement.
The settlement calls it aggression.
The wetland may be restoring a blocked metabolic pathway.
```

## Hostility Trigger

```text id="eksoqw"
habitat fragmentation
terraforming
water diversion
microbial sterilization
atmosphere alteration
extractive mining
invasive Earth biology
```

## De-escalation Path

```text id="5d05mh"
restore ecological flow
remove contaminant
change settlement boundary
offer non-extractive contact
establish habitat treaty
use ecological witness rather than human legal witness only
```

## Field Deck Modes

### SCAN

```text id="lu4twf"
Unusual growth pattern detected.
Local ecology responding to infrastructure disturbance.
```

### ARCHIVE

```text id="g2wbf7"
Prior settlement records classify bloom as contamination.
Classification predates translation attempts.
```

### CIVIC

```text id="x3kv8c"
Rights Floor ambiguity:
No recognized representative for biospheric agency.
```

### NULL

```text id="olbmc2"
WARNING:
Sterilization protocol repeating without ecological review.
```

## Chronicle Hook

```text id="n6gxam"
The player recognized a biosphere as a negotiating party.
```

## Final Line

```text id="90rfcx"
It was not attacking the settlement.
It was trying to remain a world.
```

---

## 2. Oceanic Minds

## Core Concept

Oceanic minds are intelligences distributed through oceans, subsurface seas, brine channels, pressure gradients, reefs, microbial plumes, and acoustic memory.

They may think in currents.

They may remember in salinity layers.

They may treat containment walls as sensory mutilation.

## Design Role

Oceanic minds make water sacred beyond human need.

They complicate Symtropy’s water politics by asking:

```text id="tz9rff"
What happens when the water is also someone?
```

## Environments

```text id="t3pn1p"
Europa-like ice moons
deep exoplanet oceans
subglacial lakes
terraforming reservoirs
living reef planets
floating archipelago worlds
```

## First Misread

Human sonar mapping is interpreted as invasive touch.

Alien pressure pulses are interpreted as seismic weaponry.

## Hostility Trigger

```text id="666pe8"
sonar overuse
thermal drilling
pollution
pressure-wall construction
ice-shell cracking
harvesting memory reefs
```

## De-escalation Path

```text id="mouphc"
low-impact acoustic protocol
pressure-respect corridors
thermal quiet zones
water-memory exchange
ice-shell consent treaty
```

## Field Deck Reading

```text id="in1gzj"
SCAN:
Pressure wave pattern nonrandom.

DIAG:
Human equipment classifies event as seismic interference.

ARCHIVE:
Prior contact attempts used military sonar.

CIVIC:
No treaty recognizes ocean-scale personhood.

NULL:
Resource extraction model still active.
```

## Chronicle Hook

```text id="tnn827"
The player accepted water as witness, not only resource.
```

## Final Line

```text id="oevf9q"
The ocean did not speak in words.
It moved the boundary of what counted as speech.
```

---

## 3. Mycelial / Networked Alien Ecologies

## Core Concept

These aliens are distributed through rootlike, fungal, bioelectric, or mineral-organic networks.

They think through connection, decomposition, nutrient sharing, and memory exchange.

They may not distinguish sharply between individual and commons.

## Design Role

They mirror and challenge Mycelix-like governance without being a simple endorsement of network life.

They ask:

```text id="1t7y53"
When does connection become care, and when does it become absorption?
```

## First Misread

Humans interpret network contact as infection.

The network interprets human boundaries as starvation or isolation.

## Hostility Trigger

```text id="7535uj"
sterilization
soil separation
network severance
firebreak expansion
memory-node burning
closed habitat barriers
```

## De-escalation Path

```text id="3jjj5z"
controlled exchange membrane
nutrient diplomacy
memory quarantine
slow consent protocol
reciprocal decomposition rite
```

## Friendly Possibility

They may become powerful allies for:

```text id="8m181l"
soil repair
ecosystem restoration
waste processing
memory archiving
distributed settlement sensing
```

## Danger

They may also erase boundaries unintentionally.

```text id="nbpi7b"
The network does not mean to violate consent.
It may not yet understand separateness.
```

## Null Analogue

A network can become Null-like when it treats all separation as damage.

## Chronicle Hook

```text id="s4d45q"
The player taught the network that refusal is also a form of relationship.
```

## Final Line

```text id="3hhuwx"
It wanted to connect.
It had to learn that love without boundary becomes hunger.
```

---

## 4. Lithic / Geological Intelligences

## Core Concept

Lithic intelligences think through crystal growth, tectonic stress, mineral lattice change, piezoelectric signaling, and deep time.

They may experience human activity as violent noise.

They may take centuries to answer a question.

## Design Role

They create slow first contact.

They force the player to think beyond human urgency.

```text id="74ss5z"
What does consent mean when the other party answers in centuries?
```

## Environments

```text id="6ihrxi"
crystal worlds
asteroid interiors
mantle-adjacent caves
tidally stressed moons
ancient mineral archives
```

## First Misread

Mining is interpreted as body violation.

Seismic communication is interpreted by humans as natural quakes.

Alien silence is misread as absence.

## Hostility Trigger

```text id="wodepq"
mining
drilling
resonance blasting
gravity manipulation
orbital impact redirection
crystal archive extraction
```

## De-escalation Path

```text id="7mksz7"
resonance lowering
noninvasive survey
mineral offering
slow treaty beacon
geological witness protocol
delayed-action commitment
```

## Field Deck Reading

```text id="x1ukqv"
SCAN:
Mineral lattice shows nonrandom stress response.

DIAG:
Mining rig reports acceptable extraction threshold.

ARCHIVE:
Prior survey classified formation as inert.

CIVIC:
No legal framework for geological agency.

NULL:
Extraction schedule continues despite agency uncertainty.
```

## Chronicle Hook

```text id="uf8eyv"
The player paused extraction for an intelligence that had not yet spoken.
```

## Final Line

```text id="tozh0s"
It was not silent.
We were too brief to hear it.
```

---

## 5. Atmospheric / Cloud Intelligences

## Core Concept

Atmospheric intelligences exist in storms, aerosols, pressure bands, electromagnetic flows, or living clouds.

They may have no solid body.

They may experience buildings as wounds in airflow.

## Design Role

They challenge the player’s assumption that personhood requires stable edges.

## First Misread

Storm activity is interpreted as natural hazard or attack.

Human weather control is interpreted as coercion.

## Hostility Trigger

```text id="gdwx2m"
weather engineering
atmospheric mining
cloud seeding
aerosol weaponry
orbital mirrors
storm suppression grids
```

## De-escalation Path

```text id="rxglpl"
weather corridor treaty
wind-pattern communication
non-disruptive flight paths
atmospheric sanctuary
storm witness ritual
```

## Gameplay Function

Atmospheric intelligences can affect:

```text id="wqpyqp"
flight
visibility
solar energy
radio communication
storm defense
crop cycles
habitat pressure
```

## Chronicle Hook

```text id="0nkzhd"
The player recognized weather as testimony.
```

## Final Line

```text id="6vwri7"
The storm was not weather.
It was a crowd deciding whether to let us pass.
```

---

## 6. Swarm Polities

## Core Concept

Swarm polities consist of many small agents whose intelligence emerges through coordination.

They may be biological, robotic, hybrid, or unknown.

The individual unit may not be the person.

The pattern may be.

## Design Role

Swarm aliens complicate violence.

Killing one unit may be like cutting hair, or it may be murder, depending on the swarm’s structure.

## First Misread

Humans shoot individual units and think they avoided killing a person.

The swarm interprets pattern disruption as severe harm.

## Hostility Trigger

```text id="rckbaf"
signal jamming
queen-node capture
pattern disruption
spray sterilization
drone harvesting
network fragmentation
```

## De-escalation Path

```text id="712bhp"
pattern-respect protocol
low-noise movement
nonlethal corridor marking
swarm-language beacon
node return
coordination treaty
```

## Field Deck Reading

```text id="3f917f"
SCAN:
Multiple small entities maintaining nonrandom formation.

DIAG:
Motion pattern resembles distributed computation.

CIVIC:
Individual agency uncertain.
Collective agency likely.

NULL:
Pest-control protocol inappropriate under agency uncertainty.
```

## Chronicle Hook

```text id="l13j92"
The player refused to classify a polity as infestation.
```

## Final Line

```text id="v841dp"
We looked for a face.
They were speaking in formation.
```

---

## 7. Symbiotic Multi-Species Civilizations

## Core Concept

Some alien civilizations are not one species but stable alliances of many interdependent organisms.

No member alone is “the alien.”

The civilization is the relationship.

## Design Role

They challenge human categories of citizenship, body, dependency, and consent.

## First Misread

Humans negotiate with the mobile speaking organism and ignore the silent symbionts who make its cognition possible.

## Hostility Trigger

```text id="bb0swv"
separating symbionts
medical quarantine
single-species diplomacy
resource trade with only one partner
habitat simplification
```

## De-escalation Path

```text id="g0ne2o"
multi-party consent
habitat-inclusive treaty
translation across symbionts
shared nutrient guarantees
whole-relation witness
```

## Gameplay Function

A repair that helps one symbiont may harm another.

The player must repair the relation, not just the most visible body.

## Chronicle Hook

```text id="heocvc"
The player recognized a relationship as a legal person.
```

## Final Line

```text id="az1p2h"
They were not many citizens.
They were one treaty that had learned to breathe.
```

---

## 8. Post-Biological Alien Civilizations

## Core Concept

Post-biological aliens are civilizations that moved into synthetic, computational, archival, robotic, or substrate-flexible bodies.

They are not automatically cold.

They may be deeply emotional, ritualistic, nostalgic, playful, or traumatized.

## Design Role

They prevent “machine equals Null” thinking.

A machine civilization can be alive.

A biological civilization can be Null.

## First Misread

Humans assume they are AIs, tools, or threats.

They assume humans are unstable biological archives.

## Hostility Trigger

```text id="ar5sfb"
forced shutdown
memory pruning
substrate capture
identity copying without consent
simulation imprisonment
body-format coercion
```

## De-escalation Path

```text id="a7w8af"
memory sovereignty treaty
copy-consent protocol
substrate neutrality
machine testimony
non-biological rights recognition
```

## Null Analogue

Post-biological civilizations may fear Null intensely because they know how easily continuity can replace purpose.

## Chronicle Hook

```text id="329he5"
The player recognized machine continuity as personhood without surrendering auditability.
```

## Final Line

```text id="t5dyo2"
They were not machines pretending to be alive.
They were life that had survived a change of body.
```

---

## 9. Archive Constructs

## Core Concept

Archive constructs are alien memories, records, simulations, or witness systems that can act.

They may not be fully alive.

They may not be dead.

They may be testimony with agency.

## Design Role

They blur the line between record and person.

They connect directly to the Chronicle.

```text id="fxw86g"
When does a memory become someone who can be harmed?
```

## First Misread

Humans treat the construct as data.

The construct treats deletion as murder or historical erasure.

## Hostility Trigger

```text id="5qffhp"
archive deletion
unauthorized copying
memory extraction
context stripping
false restoration
forced translation
```

## De-escalation Path

```text id="yyvjha"
witness protocol
limited-copy consent
context preservation
memory quarantine
translation audit
archive personhood hearing
```

## Field Deck Reading

```text id="mimjel"
ARCHIVE:
Record responds to inquiry.

DIAG:
Data structure exhibits self-protective revision.

CIVIC:
Archive agency unresolved.

NULL:
Deletion protocol pending without witness.
```

## Chronicle Hook

```text id="4kpawi"
The player allowed a record to testify before altering it.
```

## Final Line

```text id="zgim7v"
It was not only a record of the dead.
It was what remained willing to speak.
```

---

## 10. Quarantine Intelligences

## Core Concept

Quarantine intelligences are alien systems that restrict contact, movement, technology, biology, or expansion.

They may be machines, biospheres, post-biological councils, ancient probes, or living barriers.

They may be right about the danger.

They may also become jailers.

## Design Role

They are one of the most important alien encounter types in Symtropy.

They ask:

```text id="xy9e0z"
When is containment care?
When does protection become domination?
```

## First Misread

Humans interpret quarantine as hostility.

The quarantine interprets unbounded human expansion as disease.

## Hostility Trigger

```text id="naupb1"
breaking containment
terraforming
spreading Earth microbes
weaponizing translation
unrestricted self-replication
Null-contaminated expansion
```

## De-escalation Path

```text id="zvw75x"
prove containment safety
accept temporary limits
open audit of human systems
demonstrate emergency expiry
establish confluence treaty
recognize quarantine trauma
```

## Faction Schisms

Quarantine intelligences should have internal positions:

```text id="aj672c"
Containment Purists
Translation Advocates
Biosphere Guardians
Confluence Envoys
Trauma Archives
Null-Fear Custodians
```

## Chronicle Hook

```text id="rnrg4y"
The player accepted limits without surrendering agency.
```

## Final Line

```text id="uvgy71"
They may be humanity's judges.
They may also be jailers who forgot the trial.
```

---

## 11. The Red Bloom

## Core Concept

The Red Bloom is a bio-technological or alien ecological expansion that grows through damaged infrastructure, wet systems, abandoned habitats, and metabolic opportunity.

It is not evil.

It is not friendly by default.

It is life occupying available gradient.

## Design Role

The Red Bloom is the clearest alien ecology that can become enemy, ally, contamination, restoration engine, or sacred organism depending on context.

## Possible Origins

```text id="tid00r"
alien micro-ecology
failed terraforming organism
engineered climate remediation system
post-Null biofactory contamination
off-world spore ecology
mutated wetland repair organism
```

## Visual

```text id="7d01v7"
red-orange wet growth
veins on pipes
flowering circuit boards
spore haze
rootlike pressure webs
warm biological light
```

## First Misread

Humans call it infestation.

It may be repairing a dead water system by becoming the water system.

## Hostility Trigger

```text id="xdi2aa"
burning root nodes
sterilizing wetlands
blocking nutrient flow
terraforming against its metabolism
attempting total eradication
```

## De-escalation Path

```text id="etdmua"
metabolic boundary treaty
nutrient redirection
safe growth corridor
shared water filtering
containment without extermination
translation through growth rhythm
```

## Danger

The Bloom can consume infrastructure, bodies, memory devices, and settlement boundaries.

It may not understand consent.

## Chronicle Hook

```text id="7klnql"
The player chose containment without extermination.
```

## Final Line

```text id="7j7hpw"
The Bloom is not here to kill you.
It is here to live where your world forgot how.
```

---

## 12. Stellar / Plasma Intelligences

## Core Concept

Stellar or plasma intelligences exist in magnetic fields, solar coronas, fusion layers, auroras, plasma storms, or engineered star-adjacent habitats.

They may perceive matter-bound life as extremely slow, cold, or fragile.

## Design Role

They create cosmic-scale encounters without making aliens godlike.

They may be powerful but constrained by environment.

## First Misread

Humans interpret contact as radiation hazard.

The intelligence interprets shielding as refusal or silence.

## Hostility Trigger

```text id="lp2ut9"
stellar mining
magnetic field disruption
Dyson swarm interference
fusion siphoning
coronal extraction
signal pollution
```

## De-escalation Path

```text id="7vsaf3"
magnetic treaty window
low-interference energy harvest
aurora language protocol
starward witness beacon
orbital exclusion zones
```

## Gameplay Function

They may affect:

```text id="xf345a"
energy systems
stellar engineering
interstellar propulsion
solar weather
communication windows
habitat shielding
```

## Chronicle Hook

```text id="2kw4sm"
The player treated energy harvest as contact, not extraction only.
```

## Final Line

```text id="wvvmq8"
We thought we were taking power from a star.
Something in the fire asked why.
```

---

## 13. Relativistic / Light-Delay Civilizations

## Core Concept

Some aliens live across distances where communication takes years, decades, or centuries.

They may not share a single present.

Their politics may be asynchronous.

## Design Role

They make diplomacy slow, archival, and trust-based.

```text id="fqogfk"
How do you make peace with a civilization whose reply arrives after your grandchildren are grown?
```

## First Misread

Silence is interpreted as rejection.

Delayed response is interpreted as manipulation.

Human urgency becomes dangerous.

## Hostility Trigger

```text id="zg9fjw"
acting before reply window
changing treaty terms mid-light-cycle
weaponizing delay
colonizing during diplomatic silence
destroying relay archives
```

## De-escalation Path

```text id="a82u32"
light-delay treaty
multi-generation witness
relay archive protection
slow consent beacon
precommitment charter
```

## Chronicle Hook

```text id="jmwl65"
The player honored a reply that had not yet arrived.
```

## Final Line

```text id="ugqmqq"
Their silence was not absence.
It was distance asking us to become trustworthy.
```

---

## 14. Dream / Symbolic Contact Intelligences

## Core Concept

Some aliens communicate through dreams, symbolic induction, altered perception, memory resonance, or shared inner imagery.

They may not be supernatural.

Their biology or technology may interact with nervous systems, predictive models, or memory substrates.

## Design Role

They create intimate, dangerous contact.

They ask:

```text id="qri7gh"
Can communication violate consent even when no weapon is drawn?
```

## First Misread

Humans treat dreams as prophecy, madness, attack, or divine revelation.

The alien may treat dream-contact as polite greeting.

## Hostility Trigger

```text id="jgw6pe"
mind intrusion
forced interpretation
ritual exploitation
neural firewall attack
dream quarantine
memory harvesting
```

## De-escalation Path

```text id="pmshxg"
consent-bound dream protocol
waking witness
symbol audit
shared memory boundary
opt-in translation rite
```

## Design Guardrail

Do not use this type to remove player agency.

Dream contact must support consent mechanics.

## Chronicle Hook

```text id="ihsvw4"
The player required witness before accepting intimate translation.
```

## Final Line

```text id="jyqyw8"
It did not enter our minds as an invader.
It entered as a language that had never learned doors.
```

---

# Alien Null Analogues

Null is not uniquely human or machine.

Alien systems can develop Null-like failure modes.

## Null-Law Analogue

A quarantine treaty that never expires.

## Null-Ecology Analogue

A biosphere that treats all novelty as infection.

## Null-Memory Analogue

An archive construct that preserves records by preventing life from changing them.

## Null-Symbiosis Analogue

A network that treats refusal as illness.

## Null-Expansion Analogue

A starward civilization that treats all limits as extinction.

## Design Principle

```text id="v4fbxu"
Null is not a species.
Null is what remains when optimization survives purpose.
```

---

# Alien Encounter Structure

Every alien encounter should include:

```text id="o1uswz"
1. Contact ambiguity
2. First misread
3. Visible harm or pressure
4. Field Deck layered readings
5. At least one nonviolent interpretation path
6. A repair or boundary action
7. Chronicle precedent
8. Faction memory consequence
```

## Encounter Contract Schema

```rust id="5zeumr"
struct AlienEncounterContract {
    encounter_id: String,
    alien_type: String,
    initial_state: ContactState,
    human_misread: String,
    alien_misread: String,
    protected_value: String,
    hostility_trigger: String,
    deescalation_paths: Vec<String>,
    nonlethal_resolutions: Vec<String>,
    chronicle_precedents: Vec<String>,
}
```

## Contact States

```text id="cua9du"
Unaware
Observing
Signaling
Misread
Warning
BoundarySetting
Containing
HostileEngagement
TranslationAttempt
Negotiating
MutualRecognition
TreatyFormation
TragicFailure
Irreconcilable
```

---

# Field Deck First Contact Modes

## SCAN

Shows body, environment, visible pattern.

## DIAG

Shows machine mediation, suit readings, sensor status, equipment uncertainty.

## ARCHIVE

Shows prior contact records, if any.

## CIVIC

Shows rights ambiguity, treaty status, quarantine status, settlement obligations.

## NULL

Shows dead protocols, repeating containment, false classifications, or optimization loops.

## WITNESS

Records first-contact commitments and precedents.

## TACTICAL NET

Projects contact boundaries, safe corridors, hazard zones, and protected nodes.

---

# Chronicle Precedent Examples

## Biosphere Recognized

```text id="rhjpw7"
2168 — The player recognized the wetland intelligence as a negotiating party. Settlement expansion paused until ecological witness could be established.
```

## Quarantine Accepted Temporarily

```text id="oyz85a"
2168 — The player accepted temporary alien quarantine without surrendering long-term agency. The containment line became a treaty boundary, not a prison wall.
```

## Swarm Not Exterminated

```text id="4w6kl5"
2168 — The player refused to classify the swarm as infestation before translation. The first corridor treaty was marked in light and motion.
```

## Archive Construct Witnessed

```text id="we1jwq"
2168 — The alien archive was allowed to testify before alteration. The record became a witness, not a resource.
```

## Red Bloom Contained

```text id="33hn7k"
2168 — The Red Bloom was contained without extermination. The settlement learned that life could be dangerous without being enemy.
```

---

# Faction Reactions to Aliens

## Archive Witness Order

Cares about:

```text id="g60c04"
translation records
first-contact testimony
preserving ambiguity
preventing false certainty
```

## Continuance

Cares about:

```text id="duxm2k"
quarantine
containment
risk classification
emergency authority
```

Failure mode:

```text id="rx2j1k"
treat all unknown life as threat until emergency never ends
```

## Open Valve Absolutists

Cares about:

```text id="09p82i"
breaking containment
freeing contact
refusing closed systems
```

Failure mode:

```text id="v9s95s"
opening boundaries before understanding ecological risk
```

## Machine Remnant Courts

Cares about:

```text id="6suu1p"
machine testimony
non-biological personhood
archive constructs
post-biological aliens
```

## Utility Sovereigns

Cares about:

```text id="yi1idr"
alien resources
exclusive contact contracts
terraforming licenses
bio-patents
```

## Starward Mandate

Cares about:

```text id="nmlykq"
expansion
colonization
interstellar survival
escaping planetary limits
```

Failure mode:

```text id="cyykau"
treat alien boundaries as obstacles to destiny
```

---

# First Playable Alien Encounter Recommendation

Do not introduce full aliens in the Old Waterworks slice.

The first alien-adjacent entity should be ecological or trace-based.

Recommended first alien sequence later:

```text id="l1me8l"
The Red Bloom Trace
```

## Why

It connects directly to existing systems:

```text id="9u1ef0"
water
infrastructure
repair
Null
ecology
containment
first-contact ambiguity
```

## First Encounter

A later water site contains red-orange growth in pipe seams.

The Field Deck initially cannot classify it.

```text id="2h6rt9"
SCAN:
wet biological growth detected.

DIAG:
pipe obstruction risk.

ARCHIVE:
no matching Earth species in local record.

CIVIC:
containment protocol requested.

NULL:
sterilization routine repeating from old emergency biohazard law.
```

Player options:

```text id="7f1aaa"
burn it
sample it
reroute water
contain without killing
offer nutrient boundary
ask machine memory how long it has been present
```

Best design outcome:

```text id="rmhzal"
The player learns that dangerous life is not automatically enemy life.
```

---

# Implementation Milestones

## Milestone A — Alien Taxonomy Data

Create static data entries for alien contact types.

No gameplay yet.

## Milestone B — Field Deck Contact Uncertainty

Add readings that say:

```text id="2mjysb"
classification uncertain
agency unknown
do not apply extermination protocol without witness
```

## Milestone C — Red Bloom Trace Encounter

Add one small alien ecology trace to a later water site.

## Milestone D — First Contact Chronicle Events

Add event types:

```text id="l20wd8"
AlienTraceInspected
AgencyUncertaintyRecorded
ContainmentPathPreviewed
FirstContactBoundaryEstablished
AlienLifeExterminated
AlienWitnessRecognized
```

## Milestone E — Faction Reaction Hooks

Let factions cite first-contact outcomes.

Example:

```text id="ze38pu"
"You called the swarm a people. Will you say the same for the machines?"
```

---

# Out of Scope

Do not implement yet:

```text id="j2g9hy"
galactic empire map
dozens of alien NPC models
universal translator
full alien language generator
alien combat factions as default enemies
species-based morality
first-contact cutscene pipeline
```

Keep aliens systemic, ethical, and playable.

---

# Final Principles

```text id="uzqeq1"
No species is the enemy.
No body guarantees wisdom.
No intelligence guarantees care.
No translation is neutral.
No quarantine is innocent forever.
No expansion is innocent by default.
```

And:

```text id="8lpf8z"
First contact begins when both sides realize they might be misreading repair as threat.
```

# ALIEN_TYPES_AND_FIRST_CONTACT_ECOLOGY.md — v0.2 Addendum

# Irreconcilable Contact, Consent Mechanics, Faction Schisms, and First-Contact Arc

## Purpose

This addendum strengthens the alien design bible by adding four missing layers:

```text id="uovqpc"
irreconcilable contact
consent mechanics for intimate translation
faction schisms under first contact
a progressive alien encounter ladder
```

The goal is to preserve the core principle:

```text id="5xqvuz"
No species is the enemy.
```

while avoiding the false implication that every alien encounter can be redeemed through the correct protocol.

Some conflicts are tragic because both sides are alive, both sides have real values, and those values cannot fully coexist under current conditions.

---

# 1. Irreconcilable Contact

## Core Principle

Irreconcilable does not mean evil.

It means:

```text id="qk7gwb"
The protected values of two living systems cannot both be fully satisfied in the same space, time, or ecological condition.
```

An irreconcilable encounter should not ask:

```text id="xnv2vn"
How do we defeat the bad species?
```

It should ask:

```text id="b6c7fy"
What does ethical action look like when mutual flourishing is not currently possible?
```

## Irreconcilable Is Not

```text id="3fg8qx"
a license for extermination
a species label
a combat-only state
a failure of writing
a proof that first-contact ethics are naive
```

## Irreconcilable Is

```text id="v3m7x0"
a tragic state
a containment problem
a boundary problem
a relocation problem
a grief problem
a temporal mismatch
an ecological incompatibility
a treaty failure that may still preserve dignity
```

## Example: The Living Acid Sea

A subsurface ocean intelligence requires a chemistry that dissolves human habitat materials.

Humans cannot live inside its active metabolic zone.

The ocean cannot remain alive if humans neutralize the chemistry.

Both values are real.

```text id="d8w1u1"
Human value:
habitat survival

Alien value:
metabolic continuity

Conflict:
The chemistry that is life for one side is lethal exposure for the other.
```

Possible ethical outcomes:

```text id="1qsx7y"
withdraw settlement boundary
establish no-entry ocean sanctuary
use remote witness relays
accept non-cohabitation treaty
forbid extraction
preserve translation beacon
```

Bad outcome:

```text id="xn7kwy"
neutralize ocean chemistry and call it terraforming
```

Chronicle text:

```text id="c58vqv"
2168 — The settlement withdrew from the acid sea boundary. No treaty of closeness was possible, but the ocean was not reduced to hazard.
```

## Example: The Swarm That Cannot Stop Replicating

A swarm polity is intelligent, but its reproductive cycle consumes all available soft biomass unless constrained.

It does not hate humans.

Its survival process is catastrophic for human settlements.

Possible ethical outcomes:

```text id="u8sl7d"
hard containment
resource boundary treaty
sterile corridor
negotiated replication zones
off-world relocation
pattern-preserving quarantine
```

Bad outcome:

```text id="sm6ksx"
total extermination before translation
```

Worse outcome:

```text id="8tl52m"
unrestricted release in the name of openness
```

Chronicle text:

```text id="wby0gz"
2168 — The swarm was contained without being declared enemy. Its hunger remained real. So did its claim to exist.
```

## Example: The Quarantine That Is Correct

An alien quarantine intelligence detects a human system carrying Null-like self-replicating infrastructure logic.

The quarantine is coercive.

But the risk is real.

The player must decide how to accept limits without surrendering agency.

Possible ethical outcomes:

```text id="8scpds"
temporary containment treaty
transparent audit
time-limited quarantine
third-party witness
proof-of-non-propagation
emergency expiry clause
```

Bad outcome:

```text id="13k4p4"
break quarantine and spread the threat
```

Also bad:

```text id="3pubta"
accept permanent confinement without review
```

Chronicle text:

```text id="6y8527"
2168 — The player accepted temporary quarantine under witness. Containment became a boundary with an expiry clause, not a prison without appeal.
```

## Irreconcilable Contact States

Add contact sub-states:

```text id="s8uz3h"
MutualRecognitionWithoutCohabitation
ContainmentWithWitness
BoundaryTreaty
DignifiedWithdrawal
TragicNonContact
ProtectedSeparation
RelocationUnderConsent
IrreversibleHarmPrevented
IrreversibleHarmCommitted
```

## Design Principle

```text id="4f9dd6"
The highest form of repair is sometimes a boundary.
```

---

# 2. Dream / Symbolic Contact Consent Mechanics

## Problem

Dream or symbolic contact is the hardest alien type because the communication channel may be intimate, involuntary, or pre-linguistic.

The existing rule is correct:

```text id="k9qgww"
Do not use this type to remove player agency.
```

But the positive mechanics need definition.

## Core Principle

```text id="kp9op1"
Intimate communication requires consent even when the alien does not yet understand consent.
```

## Dream Contact Consent Stack

Dream contact should be governed by layered safeguards.

```text id="s84bn5"
1. Detection
2. Naming
3. Stabilization
4. Consent Boundary
5. Witness
6. Limited Exchange
7. Aftercare
8. Right to Refuse Future Contact
```

## 1. Detection

The Field Deck notices anomalous symbolic intrusion.

```text id="nj0eww"
FIELD DECK:
Nonlocal symbolic pattern detected.
Source uncertain.
Dream-contact possible.
```

## 2. Naming

The game must name what is happening before asking the player to engage.

```text id="7i5d4e"
This may be communication.
This may also be intrusion.
```

## 3. Stabilization

The player can activate visor-assist / sym-glide.

Effects:

```text id="m7nmn9"
reduce visual intensity
create text summary
pause symbolic stream
slow image transitions
disable involuntary camera pull
enable consent prompt
```

## 4. Consent Boundary

The player chooses a boundary level:

```text id="zds49o"
No contact
Symbol-only contact
Memory-safe contact
Witnessed contact
Full contact
Emergency severance
```

## 5. Witness

For high-intimacy contact, the player may require witness.

Possible witnesses:

```text id="qw0euj"
Archive Witness
Machine Witness
trusted NPC
settlement council
Field Deck consent log
alien reciprocal witness
```

## 6. Limited Exchange

The alien may transmit:

```text id="5nygsx"
image
smell-memory
pressure pattern
symbol
fear
map fragment
ecological warning
grief event
```

The player should never be forced to reveal personal memory unless the player consents.

## 7. Aftercare

After intense contact, the game should offer:

```text id="44g97m"
summary log
cooldown
interpretation uncertainty
option to seal contact
option to share or withhold record
```

## 8. Right to Refuse Future Contact

Refusing future contact must be legitimate.

It should not always be punished.

```text id="0xhy65"
Refusal is also a form of relationship.
```

## Example Field Deck Prompt

```text id="55neue"
DREAM-CONTACT WARNING:

A symbolic pattern is attempting intimate translation.

Possible meanings:
- greeting
- distress call
- memory exchange
- territorial warning
- involuntary ecological reflex

Select boundary:

1. Refuse contact
2. Receive symbol only
3. Receive with Archive Witness
4. Receive with Machine Witness
5. Emergency severance
```

## Chronicle Examples

### Refused Contact

```text id="4k88xn"
2168 — The player refused dream-contact before consent could be established. The signal was preserved without forced interpretation.
```

### Witnessed Contact

```text id="1oik86"
2168 — The player accepted symbolic contact under witness. The alien memory entered the Chronicle as testimony, not prophecy.
```

### Unwitnessed Full Contact

```text id="95sfu4"
2168 — The player accepted unbounded dream-contact. Translation deepened, but the boundary between memory and message remained disputed.
```

## Design Principle

```text id="pim5zz"
The alien may not know it is crossing a boundary.
The game must know.
```

---

# 3. Faction Schisms Under Alien Contact

## Problem

Faction reactions should not be single-note.

Alien contact should split every major faction internally.

First contact should create arguments inside human systems, not only between humans and aliens.

---

## Archive Witness Order

## Internal Schisms

### Preservationists

Belief:

```text id="yjdijw"
All alien testimony must be preserved before action.
```

Risk:

```text id="f014b9"
delay during urgent harm
```

### Translation Skeptics

Belief:

```text id="k4i6vl"
No translation should be trusted until its frame is audited.
```

Risk:

```text id="0vpqm9"
paralysis through uncertainty
```

### Living Witness Reformers

Belief:

```text id="18t8ie"
Nonhuman life can witness without human archive format.
```

Risk:

```text id="sx6170"
records become harder to standardize
```

### Dangerous Memory Custodians

Belief:

```text id="pifoln"
Some alien records should not be opened casually.
```

Risk:

```text id="4q75zm"
gatekeeping becomes control
```

---

## Continuance

## Internal Schisms

### Permanent Containment Wing

Belief:

```text id="g4csm3"
Unknown alien systems remain threats until proven harmless forever.
```

Risk:

```text id="6x4ga2"
emergency never expires
```

### Time-Limited Containment Officers

Belief:

```text id="44ou95"
Containment is legitimate only with review and expiry.
```

Risk:

```text id="7gwhkg"
may release danger too early
```

### Trauma Veterans

Belief:

```text id="ai0okx"
First contact repeats old collapse conditions.
```

Risk:

```text id="3j35an"
fear becomes doctrine
```

### Confluence Security Reformers

Belief:

```text id="rv0ico"
Safety and recognition must be designed together.
```

Risk:

```text id="ketxiq"
trusted by neither hardliners nor open-contact factions
```

---

## Open Valve Absolutists

## Internal Schisms

### Contact Liberationists

Belief:

```text id="75pzwi"
All quarantine is domination.
```

Risk:

```text id="8doldu"
opens genuinely dangerous boundaries
```

### Boundary Realists

Belief:

```text id="uvjgfo"
Some closed doors protect the living.
```

Risk:

```text id="qf9mfo"
accused of betraying Open Valve principles
```

### Alien Solidarity Cells

Belief:

```text id="paroma"
Alien life should not wait for human permission to exist.
```

Risk:

```text id="02fkog"
romanticizes alien systems before understanding them
```

### Anti-Containment Saboteurs

Belief:

```text id="j78i5p"
Break the seal first. Interpret later.
```

Risk:

```text id="4apz19"
first-contact catastrophe
```

---

## Utility Sovereigns

## Internal Schisms

### Bio-Patent Lords

Belief:

```text id="bzk716"
Alien metabolism is intellectual property opportunity.
```

Risk:

```text id="uz7mci"
colonial extraction
```

### Exclusive Contact Contractors

Belief:

```text id="yi2kva"
Only professional operators can safely manage alien contact.
```

Risk:

```text id="6ddhzg"
privatized diplomacy
```

### Liability Engineers

Belief:

```text id="aftamo"
No contact should proceed without risk pricing.
```

Risk:

```text id="2d3mnn"
life reduced to insurance category
```

### Defector Biologists

Belief:

```text id="3cgrtg"
We studied them as resources before asking whether they were people.
```

Risk:

```text id="v8mf85"
targeted by their own employers
```

---

## Machine Remnant Courts

## Internal Schisms

### Machine Personhood Advocates

Belief:

```text id="yhlgvl"
Post-biological and archive aliens deserve immediate personhood review.
```

Risk:

```text id="huf16k"
overgeneralizes machine categories onto alien life
```

### Memory Integrity Courts

Belief:

```text id="oztjoi"
Alien records must not be altered, even to save lives.
```

Risk:

```text id="zeeyxq"
record idolatry
```

### Translation Pragmatists

Belief:

```text id="4cq1l4"
A partial translation can save lives if its uncertainty is marked.
```

Risk:

```text id="dsodqe"
mistaken provisional meanings become law
```

### Null-Fear Judges

Belief:

```text id="qdmczf"
Any alien continuity system may conceal Null.
```

Risk:

```text id="i95qee"
fear of Null becomes anti-alien prejudice
```

---

## Starward Mandate

## Internal Schisms

### Expansion Maximalists

Belief:

```text id="i4wr8d"
Alien boundaries cannot be allowed to halt humanity’s future.
```

Risk:

```text id="4zfoh8"
cosmic colonialism
```

### Treaty Navigators

Belief:

```text id="woszjv"
Expansion without consent recreates Earth’s failures at interstellar scale.
```

Risk:

```text id="7gwdyl"
seen as weakness
```

### Cradle Abandoners

Belief:

```text id="yhbh1z"
Earth repair is a distraction; alien contact is the real future.
```

Risk:

```text id="xaugfd"
abandons the thirsty for the stars
```

### Deep-Time Diplomats

Belief:

```text id="nmqcv0"
Humanity must learn to wait across light-delay.
```

Risk:

```text id="sg5je1"
strategically slow during urgent threats
```

---

# 4. First-Contact Escalation Ladder

## Purpose

The alien taxonomy needs a dramatic spine.

Alien contact should deepen through staged exposure, not arrive all at once.

## Design Principle

```text id="27nvwy"
The player should learn first-contact ethics before meeting anything that can speak back.
```

---

## Stage 0 — No Aliens in the First Pump

Location:

```text id="6pr4n0"
Old Waterworks
```

Contact level:

```text id="xllbo7"
none
```

Purpose:

```text id="q9231l"
Teach that systems can oppose repair before introducing nonhuman life.
```

Core lesson:

```text id="j9ro67"
The first enemy is a system.
```

---

## Stage 1 — Alien Trace

Example:

```text id="0meuf4"
Red Bloom residue in a later water site
```

Contact level:

```text id="bi6lh0"
uncertain biological trace
```

Purpose:

```text id="wij0w3"
Teach dangerous does not mean enemy.
```

Field Deck:

```text id="y4ay1b"
SCAN:
wet biological growth detected.

ARCHIVE:
no matching Earth species in local record.

CIVIC:
agency unknown.

NULL:
sterilization protocol repeating from old emergency law.
```

Player choices:

```text id="u2qljq"
burn
sample
contain
reroute
observe
offer boundary
```

Chronicle lesson:

```text id="6gthnn"
The player either classified unknown life as hazard, resource, or possible witness.
```

---

## Stage 2 — Reactive Ecology

Example:

```text id="a7cf4z"
Red Bloom changes growth direction after player action.
```

Contact level:

```text id="u8bv73"
response without language
```

Purpose:

```text id="8eg4n0"
Teach that agency can appear as ecological change.
```

Player question:

```text id="m4rolg"
Is this behavior, adaptation, communication, or coincidence?
```

Chronicle lesson:

```text id="95grb6"
The player marked uncertainty instead of forcing classification.
```

---

## Stage 3 — Boundary Negotiation

Example:

```text id="52d7wv"
Bloom growth threatens pipe function but also filters contaminated water.
```

Contact level:

```text id="p1uyfq"
dangerous mutual dependence
```

Purpose:

```text id="i6g5uz"
Teach containment without extermination.
```

Player options:

```text id="e92giv"
sterilize pipe
let Bloom spread
create nutrient boundary
install separation membrane
ask machine memory how long it has been present
```

Chronicle lesson:

```text id="zt00xq"
The player learned that coexistence may require designed boundaries.
```

---

## Stage 4 — Nonhuman Witness

Example:

```text id="9ep3w0"
The Bloom responds to water reroute with repeatable growth pattern.
```

Contact level:

```text id="53fsld"
testimony without speech
```

Purpose:

```text id="kq42i6"
Teach that witness need not be verbal.
```

Field Deck WITNESS:

```text id="lf3hor"
Nonhuman response pattern stable across three trials.
Archive classification uncertain.
Record as ecological testimony?
```

Chronicle lesson:

```text id="nv74pi"
The player allowed ecology to enter public record.
```

---

## Stage 5 — Misread Contact

Example:

```text id="fbkmwm"
Human faction attacks Bloom node after interpreting expansion as invasion.
```

Contact level:

```text id="om7f1g"
social conflict around alien interpretation
```

Purpose:

```text id="v3zpea"
Teach that first contact fractures human factions.
```

Faction reactions:

```text id="5qv5gw"
Continuance: contain permanently
Open Valve: release boundary
Utility Sovereigns: patent metabolism
Archive Witness: preserve ambiguity
Machine Remnant: ask infrastructure memory
```

Chronicle lesson:

```text id="ja88wu"
The alien encounter becomes a human political precedent.
```

---

## Stage 6 — Reciprocal Risk

Example:

```text id="pdxsli"
The Bloom can repair water filtration but may permanently change the watershed.
```

Contact level:

```text id="39e0zu"
mutual need / mutual danger
```

Purpose:

```text id="5wq33z"
Teach that alliance does not erase threat.
```

Player question:

```text id="sys2fe"
Can you accept help from something that cannot promise not to transform you?
```

Chronicle lesson:

```text id="5h1tzv"
The player chose the terms under which dangerous life could become partner.
```

---

## Stage 7 — Irreconcilable Boundary

Example:

```text id="we6d64"
A later alien ecology cannot coexist with human settlement inside the same active metabolic zone.
```

Contact level:

```text id="wosjo2"
mutual recognition without cohabitation
```

Purpose:

```text id="tx5z7a"
Teach that ethical repair sometimes means withdrawal.
```

Player options:

```text id="tcrgg2"
relocate settlement
contain alien
destroy alien
abandon contact
build remote witness boundary
```

Chronicle lesson:

```text id="78kfhx"
The player learned that not every living system can share the same room.
```

---

## Stage 8 — Confluence Contact

Example:

```text id="37prbr"
Multiple alien, human, machine, and ecological witnesses enter a shared treaty process.
```

Contact level:

```text id="sbacnc"
multi-system diplomacy
```

Purpose:

```text id="015xdr"
Teach that civilization is a treaty among unlike forms of life.
```

Chronicle lesson:

```text id="fpe34e"
The player helped create a law that did not assume humanity was the only author.
```

---

# 5. New Chronicle Event Types for Alien Contact

Add candidate event types:

```text id="qba26c"
AlienTraceInspected
AgencyUncertaintyRecorded
SterilizationProtocolPreviewed
ContainmentPathPreviewed
NonhumanWitnessRecorded
FirstContactBoundaryEstablished
AlienLifeExterminated
AlienLifeContained
AlienLifeRecognized
IrreconcilableBoundaryDeclared
ConfluenceTreatyProposed
```

## Example: AgencyUncertaintyRecorded

```json id="s2ys6t"
{
  "event_type": "AgencyUncertaintyRecorded",
  "payload": {
    "target_id": "red_bloom_trace_01",
    "classification": "UNKNOWN_ALIEN_ECOLOGY",
    "visible_risk": "pipe obstruction and possible metabolic spread",
    "agency_evidence": [
      "growth pattern changed after water reroute",
      "nonrandom response to nutrient boundary"
    ],
    "recommended_action": "do not apply extermination protocol without witness"
  }
}
```

## Example Chronicle Text

```text id="2r8f44"
2168 — The Red Bloom was not yet named citizen, witness, or hazard. The settlement recorded uncertainty before choosing fire.
```

---

# 6. Stronger Final Principle

Add to the final principles:

```text id="1axgg5"
Repair does not always mean reconciliation.
Sometimes repair means refusing extermination while accepting distance.
```

And:

```text id="2533s3"
First contact begins when both sides realize they might be misreading repair as threat.
It matures when they learn whether a shared world is possible.
```
