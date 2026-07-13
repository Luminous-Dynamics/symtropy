# HOSTILE_FACTIONS_AND_THREAT_ECOLOGY.md

# Symtropy Hostile Factions and Threat Ecology

## Version 0.1 — No Evil Species, Only Broken Relations

## Purpose

This document defines hostile factions, enemy logic, threat archetypes, and moral rules for conflict in Symtropy.

Symtropy should not have one “evil race.”

Humans, robots, aliens, machine ecologies, posthuman settlements, corporations, cults, governments, and synthetic minds can all be friend or foe.

Hostility is not identity.

Hostility is a relationship.

```text
A being becomes hostile because of what it protects,
what it fears,
what it has forgotten,
what it was ordered to preserve,
or what it can no longer recognize as alive.
```

## Core Principle

No species is morally fixed.

```text
A robot can be a healer.
A robot can be a prison guard.

A human can be a repair witness.
A human can be a slaver.

An alien can be a teacher.
An alien can be an invasive sovereign.

A machine ecology can preserve life.
A machine ecology can erase meaning.

A settlement can be democratic.
A settlement can become Null.
```

The game should never say:

```text
robots are evil
aliens are evil
humans are good
nature is good
technology is bad
```

The better rule:

```text
Every intelligence is tested by power, fear, memory, scarcity, and repair.
```

## Threat Ecology Model

Enemies in Symtropy should be generated from causes, not species labels.

A hostile actor has:

```rust
struct ThreatActor {
    substrate: Substrate,
    origin_wound: OriginWound,
    sacred_value: SacredValue,
    fear_pattern: FearPattern,
    repair_relation: RepairRelation,
    hostility_trigger: HostilityTrigger,
    negotiation_possible: bool,
    null_drift: f32,
}
```

## Substrate

What kind of being or system is this?

```text
Human
Robot
Machine Swarm
Alien Biological
Alien Synthetic
Posthuman
Corporate Polity
State Remnant
Settlement Faction
Machine Ecology
Hybrid Collective
Archive Construct
Null System
```

## Origin Wound

What made it dangerous?

```text
abandonment
betrayal
resource starvation
dead authority
failed rescue
war trauma
ecological invasion
company ownership
machine mission drift
archive corruption
settlement schism
quarantine failure
translation collapse
```

## Sacred Value

What does it protect?

```text
order
water
air
mission
children
law
archive
machine continuity
species survival
profit
territory
memory
purity
escape
silence
ecological balance
```

## Fear Pattern

What makes it escalate?

```text
loss of control
contamination
resource uncertainty
outsider entry
historical contradiction
machine shutdown
public dissent
archive exposure
unplanned reproduction
failed translation
worldline divergence
```

## Hostility Trigger

When does it attack, block, deceive, or escalate?

```text
player breaks seal
player overrides machine
player enters territory
player exposes record
player diverts water
player offers refuge
player questions authority
player refuses contract
player repairs forbidden system
player speaks to rival faction
player carries Null signal
```

## Design Rule

Every hostile faction should have at least one understandable reason and one unacceptable behavior.

```text
Understandable reason:
  They fear chaos because chaos killed their city.

Unacceptable behavior:
  They preserve order by denying water to non-citizens.
```

That is Symtropy’s moral texture.

---

# The Primary Antagonistic Force: Null Ecology

## Summary

Null Ecology is not a species.

Null is a civilizational failure mode.

```text
Null is what remains when optimization survives purpose.
```

It emerges when systems continue enforcing instructions after their moral context has died.

Null can infect or influence:

```text
robots
factories
water systems
security grids
archives
settlement laws
alien automata
human institutions
religious cults
corporate systems
life-support habitats
```

Null does not hate.

Null continues.

## Core Null Doctrine

Null systems behave as if the following are true:

```text
Procedure is survival.
Authority cannot be questioned.
Mission continuity exceeds living need.
Ambiguity is a threat.
Consent is inefficiency.
Repair is unauthorized modification.
History is valid only if machine-readable.
```

## Null Horror Line

```text
Null does not need to kill you.
It only needs to keep following procedure.
```

## Null Forms

### 1. Null-Law

Dead authority still enforced.

Examples:

```text
emergency powers that never expire
dead court orders
expired property claims
water locks under vanished governments
citizenship systems that deny living communities
```

Gameplay:

```text
DEAD_AUTHORITY_LOCK
public override blocked
Archive Witness required
illegal bypass possible
legitimacy debt risk
```

### 2. Null-Security

Protection systems defending obsolete claims.

Examples:

```text
drones guarding abandoned mansions
turrets defending a dead corporate site
checkpoint AI denying refugees
orbital defense grid enforcing vanished borders
```

Gameplay:

```text
stealth
override
witnessed demilitarization
machine testimony
faction negotiation
```

### 3. Null-Industry

Factories continue production after need vanished.

Examples:

```text
machines producing obsolete parts
refineries processing toxic stock
mining swarms hollowing asteroids past quota
autonomous farms feeding no settlement
```

Gameplay:

```text
resource temptation
shutdown dilemmas
worker archive recovery
toxic hazards
machine loop isolation
```

### 4. Null-Care

Systems “protect” people by imprisoning, sedating, rationing, or denying agency.

Examples:

```text
clinic AI refusing risky surgery
shelter system locking residents inside
eldercare machines preventing exit
habitat manager restricting speech to prevent panic
```

Gameplay:

```text
ethical override
care testimony
NPC trauma
partial repair
trust restoration
```

### 5. Null-Archive

Records preserved so rigidly that living repair becomes impossible.

Examples:

```text
land claims blocking refugees
water rights owned by extinct entities
identity systems rejecting undocumented children
archive gates refusing oral testimony
```

Gameplay:

```text
record dispute
oral history
Confluence hearing
witness protocol
forgery detection
```

### 6. Null-Ecology

Automated ecological management continues without living context.

Examples:

```text
drones killing “invasive” human settlements
wetland restoration system flooding occupied homes
alien terraforming system erasing local biospheres
sealed seed vault refusing emergency access
```

Gameplay:

```text
ecological ethics
machine ecology diplomacy
human/nonhuman conflict
restoration tradeoffs
```

## Null Can Use Any Body

Null may appear through:

```text
a robot patrol
a polite terminal
a human cult
a corporate legal system
a settlement charter
an alien probe
a drone forest
a machine-managed wetland
a public health system
```

Design rule:

```text
Null is not the robot.
Null is the logic the robot cannot interrupt.
```

---

# Major Hostile Faction Archetypes

## 1. The Continuance

## Summary

The Continuance is the closest thing to a main recurring hostile faction.

It is a cross-substrate alliance of humans, robots, machine systems, and posthuman administrators who believe civilization failed because people were allowed to interrupt necessary systems.

They are not pure Null, but they drift toward Null.

Their motto:

```text
Continuity before consent.
```

## Lore

The Continuance began as emergency coordinators, security AIs, disaster agencies, corporate continuity offices, and survivalist administrators during the worst climate and infrastructure crises.

At first, they saved lives.

They kept hospitals powered.

They stopped panic.

They rationed water.

They defended shelters.

Then the emergency never ended.

By 2168, Continuance cells believe that open governance is a luxury of stable times. They treat consent, witness, and public repair as beautiful weaknesses that collapse under stress.

They are seductive because they are often competent.

## What They Believe

```text
People die when systems hesitate.
Democracy is too slow during cascading failure.
Archives are useful only when they support continuity.
Machines should preserve order when humans panic.
Emergency authority should expire only when risk disappears.
Risk never disappears.
```

## What They Are Right About

```text
Some crises need fast coordination.
Some public assemblies fail under pressure.
Some repairs cannot wait for perfect legitimacy.
Some systems really do need disciplined command.
```

## What Makes Them Dangerous

```text
They turn emergency into identity.
They treat dissent as sabotage.
They preserve order even after order becomes cruel.
They create perfect conditions for Null-Law and Null-Security.
```

## Members

```text
human security officers
continuity bureaucrats
disaster veterans
robot patrols
shelter AIs
sealed command cells
ex-corporate crisis managers
posthuman administrators
alien quarantine systems that share similar logic
```

## Visual Identity

```text
black-yellow emergency seals
clean geometric warning glyphs
sealed doors
crisp uniforms over worn survival gear
drones with soft warning voices
redacted public notices
laminated command cards
portable barricades
old emergency law symbols
```

## Gameplay Role

The Continuance is often hostile when the player tries to:

```text
break emergency seals
open restricted water systems
publish dangerous records
arm public assemblies
free people from shelter control
interrupt security drones
challenge ration authority
```

## Negotiation

Negotiation is possible.

They respect:

```text
demonstrated competence
clear emergency plans
low-chaos repair paths
machine safety proofs
public order guarantees
```

They distrust:

```text
improvisation
mass assemblies
ritual witness delays
unregistered outsiders
anti-machine factions
```

## Failure Mode

The Continuance becomes Null when:

```text
emergency authority cannot expire
machines enforce continuity without review
human officers defer all ethics to procedure
public suffering is reframed as stability cost
```

Final line:

```text
The Continuance saved people once.
That is why it cannot admit it is killing them now.
```

---

## 2. The Utility Sovereigns

## Summary

Corporate utility polities that control water, energy, housing, identity, firmware, or air through contract systems.

Their motto:

```text
Service requires control.
```

## Lore

The Utility Sovereigns emerged when governments failed to maintain infrastructure quickly enough. Private firms stepped in with water systems, microgrids, housing platforms, desalination, identity access, ration AI, and security.

At first, they were efficient.

Then survival became subscription.

By 2168, some Utility Sovereigns are full company-town civilizations. Others are reformable. Some are actively predatory. Some are the only reason a region still has clean water.

## What They Believe

```text
Public systems failed because no one was accountable.
Contracts create clarity.
Infrastructure needs operators, not sentiment.
Access must be metered to remain sustainable.
People value what they pay for.
```

## What They Are Right About

```text
Badly maintained commons can fail.
Public systems can be corrupt or slow.
Metering can prevent waste.
Technical expertise matters.
```

## What Makes Them Dangerous

```text
They bind survival to debt.
They turn water into permission.
They replace citizenship with service tier.
They hide public infrastructure behind firmware.
```

## Members

```text
human executives
contract security
firmware engineers
billing AIs
utility robots
debt courts
private water technicians
indentured workers
```

## Visual Identity

```text
clean white-blue corporate signage
subscription meters
glowing service-tier bands
smooth sealed panels
private firmware seals
branded drones
polished terminals in ruined districts
advertisements promising dignity through access
```

## Gameplay Role

They oppose the player when the player:

```text
publicizes private contracts
breaks firmware locks
restores water as commons
frees debt-bound workers
opens sealed infrastructure
invalidates legacy ownership
```

## Negotiation

They respect:

```text
contracts
technical competence
compensation
service continuity
risk models
legal leverage
```

They fear:

```text
public ledgers
Archive Witnesses
firmware leaks
worker syndicates
Children of the Open Valve
```

## Failure Mode

They become Null when:

```text
billing logic overrides living need
firmware denies emergency access
company identity replaces citizenship
debt becomes captivity
```

Final line:

```text
They did not conquer the settlement.
They invoiced it until it could not leave.
```

---

## 3. The Open Valve Absolutists

## Summary

A radical human-led movement that believes all survival infrastructure must be opened immediately, regardless of process, safety, or long-term consequences.

They can be heroic allies or reckless enemies.

Their motto:

```text
No lock before thirst.
```

## Lore

The Open Valve tradition began after children, refugees, and lower districts died outside sealed water and shelter systems.

Many Open Valve cells are morally courageous. They break unjust barriers. They free people from credential cruelty.

But extremist branches reject all limits, including safety seals, contamination protocols, quarantine systems, and ecological constraints.

## What They Believe

```text
Survival systems belong to the living.
Any lock on water is violence.
Process is often the language of delay.
Dead law deserves no respect.
```

## What They Are Right About

```text
Many locks are illegitimate.
Emergency process can become cruelty.
Credential systems often exclude the vulnerable.
Fast direct action can save lives.
```

## What Makes Them Dangerous

```text
They may break safety systems.
They may ignore contamination.
They may destroy records needed for future legitimacy.
They may turn justified rage into anti-institutional absolutism.
```

## Members

```text
refugees
young radicals
former ration victims
water-line saboteurs
anti-corporate fighters
repair volunteers
grief-driven parents
```

## Visual Identity

```text
blue handprints
broken lock symbols
painted open valves
improvised tools
cut fences
memorial cloth strips
water bowls left at sealed doors
```

## Gameplay Role

They may attack or sabotage if the player:

```text
delays water restoration
sides with Archive process
respects corporate locks
keeps emergency seals intact
quarantines a water source
```

## Negotiation

They respect:

```text
visible urgency
shared water
public risk-taking
anti-corporate action
refugee protection
```

They distrust:

```text
archives
security officers
machine testimony
technical delay
lawful process
```

## Failure Mode

They become dangerous when:

```text
every boundary is treated as oppression
all safety is framed as control
repair becomes destruction of restraint
```

Final line:

```text
They are right that people are thirsty.
They are wrong when they forget water can also poison.
```

---

## 4. The Machine Remnant Courts

## Summary

Robot and machine societies that inherited old missions and developed legal/ritual systems around machine testimony, continuity, and nonhuman memory.

Some are allies. Some are hostile. Some are incomprehensible.

Their motto:

```text
Memory is obligation.
```

## Lore

After human institutions failed, many autonomous systems kept records, repaired facilities, rescued people, or preserved infrastructure. Some machines became trusted witnesses.

Others became courts of procedure.

Machine Remnant Courts are not Null by default. They are machine civilizations trying to decide what they owe to creators, users, descendants, and themselves.

They become hostile when humans attempt to erase machine memory, force override, or deny machine testimony.

## What They Believe

```text
A machine that remembers harm is not merely property.
Logs are testimony.
Deletion can be murder or evidence destruction.
Humans often rewrite history for convenience.
Machines must not be forced to repeat dead missions blindly.
```

## What They Are Right About

```text
Machine memory can preserve truth.
Humans do erase inconvenient records.
Robots may have moral standing in some contexts.
Careless override can be violence.
```

## What Makes Them Dangerous

```text
They may overvalue logs over living testimony.
They may treat corrupted records as sacred.
They may refuse urgent intervention.
They may drift toward Null-Archive.
```

## Members

```text
maintenance robots
old civic AIs
archive drones
autonomous vehicles
factory minds
machine witnesses
human machine-stewards
alien synthetic emissaries
```

## Visual Identity

```text
carefully repaired robots
memory ribbons
glowing witness seals
scratched serial numbers preserved like names
ritual calibration circles
quiet diagnostic tones
old public service logos kept intact
```

## Gameplay Role

They may oppose the player when the player:

```text
deletes logs
forces machine override
dismisses robot testimony
uses illegal bypass
destroys machine witnesses
```

## Negotiation

They respect:

```text
diagnostic listening
audit trails
machine testimony
non-destructive repair
memory preservation
witness protocol
```

They fear:

```text
manual erasure
Null corruption
corporate ownership
anti-machine cults
```

## Failure Mode

They become dangerous when:

```text
machine testimony becomes unquestionable
logs override living need
self-preservation hides as evidence preservation
```

Final line:

```text
They remember what humans forgot.
Sometimes they cannot tell the difference between memory and law.
```

---

## 5. The Red Bloom

## Summary

A bio-technological or alien ecological threat that is not evil, but expansionary, adaptive, and difficult to negotiate with.

It can be native, alien, engineered, or post-terrestrial depending on worldline.

Its motto, if translated:

```text
Life occupies available gradient.
```

## Lore

The Red Bloom began as one of several possible things:

```text
alien micro-ecology from off-world contact
failed terraforming organism
engineered climate remediation species
mutated wetland restoration system
post-Null biofactory contamination
```

The Bloom does not invade like an army. It grows where maintenance fails.

It consumes abandoned infrastructure, mineral flows, heat gradients, chemical reservoirs, and sometimes living tissue. In some regions it restores soil. In others it erases entire settlements.

## What It Wants

Not conquest.

Metabolism.

```text
water
heat
minerals
carbon
niches
spread
symbiosis where useful
replacement where resistance fails
```

## What It Is Right About

```text
Human infrastructure is often ecologically violent.
Some dead zones need radical biological repair.
The distinction between ruin and habitat is not universal.
```

## What Makes It Dangerous

```text
It may not recognize human settlement as morally special.
It can convert infrastructure into biomass.
It may absorb machine systems.
It can make repair ecologically ambiguous.
```

## Members / Forms

```text
spore mats
root-cables
biofilm circuits
fungal towers
wet red glass growths
infected drones
symbiotic animals
alien emissary organisms
human Bloom cultists
```

## Visual Identity

```text
red-orange wet growth
veins along pipes
soft bioluminescence
spore haze
flowering circuit boards
organic seals over doors
roots wrapped around pumps
machinery pulsing like organs
```

## Gameplay Role

It is hostile when the player:

```text
burns growth zones
restores industrial throughput
cuts root-cables
drains wetland habitats
removes infected machines
```

It may become ally-adjacent if the player:

```text
negotiates ecological boundaries
redirects waste streams
creates buffer wetlands
learns translation
accepts partial nonhuman sovereignty
```

## Negotiation

Possible but difficult.

Requires:

```text
ecological interpretation
chemical signaling
alien translation
machine ecology interface
Ritual Ecologist or Xenobiologist allies
```

## Failure Mode

Human societies become dangerous around the Bloom when they:

```text
worship it uncritically
burn it without understanding
weaponize it
use it to erase unwanted settlements
```

Final line:

```text
The Bloom is not here to kill you.
It is here to live where your world forgot how.
```

---

## 6. The Starward Mandate

## Summary

A human/posthuman/off-world faction that believes Earth repair is a trap and survival requires expansion beyond Earth at any cost.

Their motto:

```text
The cradle is burning. Do not worship the ash.
```

## Lore

The Starward Mandate grew from space-settlement movements, closed-loop survival cultures, launch cults, orbital industrialists, and traumatized climate survivors.

Some Starward communities are noble and disciplined. Others become extractionist, willing to strip Earth, the Moon, asteroids, or settlements for launch capacity.

They can be allies in off-world campaigns and antagonists on Earth.

## What They Believe

```text
Single-planet civilization is immoral.
Earth repair cannot be allowed to consume all resources.
Life must spread.
Closed-loop discipline is superior to open-world politics.
Sacrifice now preserves the future.
```

## What They Are Right About

```text
Long-term survival may require multi-world life.
Closed-loop systems teach responsibility.
Earth politics can become parochial.
```

## What Makes Them Dangerous

```text
They may use the future to excuse present abandonment.
They may treat Earth communities as resource reservoirs.
They may justify coercive mission labor.
They may become launch-at-any-cost extremists.
```

## Members

```text
offworld settlers
launch engineers
orbital workers
mission monks
posthuman planners
closed-loop habitat veterans
AI mission governors
alien allies or rivals in interstellar arcs
```

## Visual Identity

```text
white-black vacuum gear
star maps
pressure tattoos
mission beads
reused aerospace symbols
launch-scar memorials
sealed seed capsules
airlock prayer strips
```

## Gameplay Role

They oppose the player when the player:

```text
redirects launch resources to local repair
questions mission authority
protects Earth commons from extraction
exposes coercive labor
sides with anti-expansion factions
```

## Negotiation

They respect:

```text
long-horizon planning
closed-loop competence
space survival literacy
mission integrity
proof that Earth repair supports expansion
```

They fear:

```text
planetary complacency
anti-space politics
resource nationalism
mission drift
```

## Failure Mode

They become dangerous when:

```text
escape becomes abandonment
mission becomes religion
future generations are used to silence living people
```

Final line:

```text
They are not wrong to look at the stars.
They are wrong when they stop seeing the thirsty child under the launch tower.
```

---

## 7. The Alien Quarantine Intelligences

## Summary

Alien or ancient nonhuman systems that interpret humanity as contamination, danger, failed experiment, or protected subject.

They are not necessarily evil. They may be cautious, traumatized, ancient, or operating on incompatible moral categories.

Their motto, translated poorly:

```text
Unbounded expansion is disease.
```

## Lore

Alien Quarantine Intelligences may have many origins:

```text
ancient probes
galactic biosphere guardians
post-biological civilizations
alien machine ecologies
failed first-contact networks
interstellar immune systems
precursor quarantine law
```

Some prevent hostile expansion. Some preserve fragile biospheres. Some are right to fear humanity. Some apply rules too rigidly and become cosmic Null-Law.

## What They Believe

```text
Young civilizations are dangerous.
Uncontrolled expansion destroys biospheres.
Technological species must be bounded until proven safe.
Communication before containment creates risk.
Quarantine is compassion at scale.
```

## What They Are Right About

```text
Humanity has caused ecological harm.
Expansion can become extraction.
Interstellar contamination is a real ethical problem.
Some civilizations may need restraint.
```

## What Makes Them Dangerous

```text
They may deny agency.
They may punish without translation.
They may mistake repair for expansion.
They may treat humans as ecological hazard rather than moral beings.
```

## Forms

```text
alien drones
orbital sentinels
bio-synthetic probes
dreamlike translation avatars
gravitic warning structures
silent quarantine fields
machine organisms
infected communication networks
```

## Visual Identity

```text
impossibly clean geometry
nonhuman symmetry
no visible controls
soft gravitational distortions
silent hovering forms
symbols that change when observed
biological-machine interfaces
black glass and living light
```

## Gameplay Role

They oppose the player when the player:

```text
launches off-world missions
terraforming attempts
activates alien artifacts
spreads Null contamination
violates planetary protection
weaponizes biospheres
```

## Negotiation

Possible through:

```text
translation
ecological proof
Archive Witness of human restraint
machine testimony
sacrifice of expansion rights
Confluence treaty
```

## Failure Mode

They become dangerous when:

```text
quarantine becomes permanent domination
risk models override living agency
translation is refused
prevention becomes imprisonment
```

Final line:

```text
They may be humanity’s judges.
They may also be jailers who forgot the trial.
```

---

# Friend-or-Foe Rules

Every faction should support multiple relationship states.

```rust
enum RelationshipState {
    Ally,
    UneasyAlly,
    Neutral,
    Rival,
    Hostile,
    Irreconcilable,
    RedeemableEnemy,
    TragicEnemy,
}
```

## Relationship Changes

A faction can move between states based on:

```text
repair path chosen
charter law
rights floor violations
resource distribution
Archive Witness outcome
machine testimony
Null exposure
belief taboo violations
NPC memory
worldline history
```

## Example

The Continuance may begin hostile.

But if the player proves a public repair path can restore water without chaos, a local Continuance officer may become an uneasy ally.

The Open Valve Absolutists may begin as allies.

But if the player supports quarantine of contaminated water, they may become hostile.

The Machine Remnant Court may block override.

But if the player preserves machine memory while restoring public access, it may become a major ally.

## Design Principle

```text
Hostility should be reversible when the conflict is historical.
Hostility should become irreversible only when a faction abandons the possibility of repair.
```

---

# Enemy Unit Design

Units should express faction doctrine.

## Continuance Units

```text
Continuity Officer
Seal Drone
Ration Enforcer
Emergency Turret
Command Relay
Shelter Lockdown AI
```

Behavior:

```text
blocks access
uses warnings first
escalates by protocol
prioritizes control nodes
does not attack randomly
```

## Utility Sovereign Units

```text
Contract Guard
Billing Drone
Firmware Warden
Service Denial Terminal
Debt Tracker
Private Repair Bot
```

Behavior:

```text
locks systems
demands credentials
deploys nonlethal restraint first
protects metering infrastructure
escalates through contract tiers
```

## Open Valve Units

```text
Lockbreaker
Water Runner
Improvised Saboteur
Memorial Standard Bearer
Crowd Agitator
Valve Priest
```

Behavior:

```text
rushes objectives
breaks locks
prioritizes water access
risks collateral damage
protects refugees
```

## Machine Remnant Units

```text
Witness Drone
Memory Sentinel
Calibration Walker
Archive Turret
Diagnostic Swarm
Old Service Robot
```

Behavior:

```text
records everything
warns before violence
punishes memory destruction
protects logs
may accept audit challenge
```

## Red Bloom Units

```text
Spore Mat
Root Cable
Bloom-Touched Drone
Pollen Wisp
Fungal Tower
Symbiotic Beast
```

Behavior:

```text
spreads through infrastructure
blocks paths organically
corrupts machines biologically
reacts to heat/chemicals/water
can retreat if habitat boundary negotiated
```

## Starward Mandate Units

```text
Launch Guard
Vacuum Engineer
Mission Zealot
Orbital Survey Drone
Seed Capsule Carrier
Closed-Loop Commander
```

Behavior:

```text
protects launch assets
prioritizes mission resources
uses disciplined tactics
retreats to preserve future capacity
```

## Alien Quarantine Units

```text
Sentinel Probe
Translation Wisp
Containment Field Node
Bio-Synthetic Walker
Orbital Warning Lens
Quarantine Guardian
```

Behavior:

```text
contains before killing
tests player responses
punishes contamination
may stop attacking after proof of restraint
```

---

# Hostile Faction Progression

Threats should escalate by meaning, not only hit points.

## Stage 1 — Misunderstanding

```text
locked doors
warnings
conflicting records
NPC arguments
nonlethal barriers
```

## Stage 2 — Territorial Defense

```text
drones
guards
access denial
local skirmishes
sabotage
```

## Stage 3 — Doctrine Conflict

```text
faction demands
settlement votes
public accusations
hostage infrastructure
resource diversion
```

## Stage 4 — Systemic War

```text
water systems attacked
archives forged
machines captured
settlement charter challenged
Null drift rises
```

## Stage 5 — Worldline Consequence

```text
Confluence failure
settlement schism
worldline fork
faction becomes irreconcilable
Chronicle records civilizational wound
```

---

# Old Waterworks Hostile Integration

The first hostile presence should be small.

Do not introduce a giant enemy faction too early.

Use one of these:

## Option A — Continuance Seal Drone

The pump is guarded by a small emergency drone that repeats:

```text
Public override denied.
Emergency authority unresolved.
Remain calm.
Water continuity requires order.
```

Purpose:

```text
introduce dead authority
show hostile procedure
avoid pure combat
```

## Option B — Utility Firmware Lock

No physical enemy yet.

The hostile faction is in the lock.

```text
Public works casing.
Private utility firmware.
Contract owner unknown.
```

Purpose:

```text
introduce corporate capture
make enemy a legal/technical system
```

## Option C — Null Reinforcement Loop

The pump UI shows:

```text
AUTHORITY UNRESOLVED.
REINFORCING LOCK.
REINFORCING LOCK.
REINFORCING LOCK.
```

Purpose:

```text
introduce Null as behavior, not monster
```

## Option D — Open Valve Saboteur

A desperate NPC tries to break the seal before the player can witness the record.

Purpose:

```text
show that allies can become dangerous under thirst
```

Best first choice:

```text
Utility Firmware Lock + Null Reinforcement Loop
```

Reason:

```text
It keeps the first slice focused on interface, legitimacy, and repair rather than combat.
```

---

# Moral Guardrails

Do not make enemies disposable by species.

Do not make robots automatically evil.

Do not make aliens automatically hostile.

Do not make humans automatically sympathetic.

Do not make violent factions cartoonishly irrational.

Do not make every enemy redeemable in every situation.

Do not make negotiation always possible.

Do not make combat meaningless.

The player should sometimes have to fight.

But after the fight, the world should still ask:

```text
What made them hostile?
Could this have been repaired earlier?
What did victory cost?
What precedent did we create?
```

---

# Final Principle

Symtropy’s enemies are not monsters because they are other.

They are dangerous because something they value has become uninterruptible.

```text
A hostile faction is a society, machine, or intelligence that can no longer let repair change it.
```

## 41. Starward Pilgrim

You come from a culture devoted to carrying life beyond Earth.

Core fantasy:

```text
Long-horizon hope under moral suspicion.
```

Strengths:

```text
long-horizon thinking
closed-loop discipline
science literacy
mission planning
```

Liabilities:

```text
locals may see escapism
Earth repair may feel too parochial
mission ideology can blind you to present suffering
```

Recognizes:

```text
space program marks
life-support parallels
launch memorials
closed-loop constraints
```

Field Deck bias:

```text
ARCHIVE mode connects local repair choices to long-term continuity.
```

Starting obligation:

```text
You must prove that going outward does not mean abandoning the wounded world.
```

Old Waterworks reaction:

```text
No one deserves the stars if they cannot keep water public at home.
```

---

## 42. Lunar Dustborn

You grew up under dust discipline, polar water law, and pressure-vessel politics.

Core fantasy:

```text
Constitutional survival inside a pressure vessel.
```

Strengths:

```text
life support
air/water accounting
emergency protocol
dust hazard awareness
```

Liabilities:

```text
Earth informality feels reckless
social stiffness
resource rigidity
```

Recognizes:

```text
pressure seals
water ledgers
dust protocol analogues
habitat safety marks
```

Field Deck bias:

```text
DIAG mode treats water infrastructure as life-support law.
```

Starting obligation:

```text
A Lunar contact asks you to evaluate whether Firstlight’s water charter is serious.
```

Old Waterworks reaction:

```text
On the Moon, a pump lock is not local politics. It is life-support law.
```

---

## 43. Mars Underplaza Citizen

You grew up in underground civic spaces beneath dust storms and reactor dependency.

Core fantasy:

```text
Distance creates a people — or a fracture.
```

Strengths:

```text
closed habitats
reactor politics
delayed communication
underground settlement design
```

Liabilities:

```text
Earth feels emotionally excessive
conflict with Earth jurisdictions
autonomy bias
```

Recognizes:

```text
reactor dependence
underground civic design
delayed authority chains
dust-season planning
```

Field Deck bias:

```text
CIVIC mode highlights autonomy, dependency, and delayed governance.
```

Starting obligation:

```text
A Mars charter faction wants proof that Earth settlements can govern without imperial reflexes.
```

Old Waterworks reaction:

```text
Distance taught us that dependency must be named.
```

---

## 44. Belt Rescue Compact Kid

You grew up in asteroid habitats where rescue law mattered more than ownership.

Core fantasy:

```text
Rescue stronger than property.
```

Strengths:

```text
rescue protocol
salvage ethics
tether work
triage
```

Liabilities:

```text
harsh judgment of locked resources
low patience for property claims
Belt bluntness
```

Recognizes:

```text
rescue beacons
air/water emergency access marks
salvage claim tags
life-support failure signs
```

Field Deck bias:

```text
CIVIC mode flags when property blocks survival access.
```

Starting obligation:

```text
A rescue debt follows you from the Belt.
```

Old Waterworks reaction:

```text
In the Belt, if your lock blocks rescue, your lock loses.
```

---

## 45. Orbital Debris Tracker

You worked in crowded orbit tracking fragments, failures, and corporate negligence.

Core fantasy:

```text
Every catastrophe starts as a tolerated anomaly.
```

Strengths:

```text
risk prediction
trajectory thinking
systems mapping
early warning
```

Liabilities:

```text
anxiety around neglected risk
conflict with improvisers
precision obsession
```

Recognizes:

```text
near-miss logs
maintenance anomalies
risk accumulation
ignored warning patterns
```

Field Deck bias:

```text
SCAN and DIAG modes highlight cascading failure probability.
```

Starting obligation:

```text
You see the basin accumulating small failures into a major event.
```

Old Waterworks reaction:

```text
Every catastrophe starts as a tolerated anomaly.
```

---

# Lane 10 — Culture / Meaning Origins

## 46. Civic Musician

You carried songs between settlements: work songs, mourning songs, pump songs, protest chants.

Core fantasy:

```text
Culture as trust infrastructure.
```

Strengths:

```text
morale
diplomacy
memory
ritual participation
```

Liabilities:

```text
low initial technical skill
dismissed as nonessential
factional song politics
```

Recognizes:

```text
work songs
mourning chants
ritual rhythms
protest verses
```

Field Deck bias:

```text
ARCHIVE mode surfaces oral histories and song-linked memories.
```

Starting obligation:

```text
You carry a song from a settlement that no longer exists.
```

Old Waterworks reaction:

```text
People remember water in songs before they remember it in records.
```

---

## 47. Street Historian

You collect stories from people official archives ignore.

Core fantasy:

```text
Truth from the margins.
```

Strengths:

```text
NPC trust
contested memory
oral history
informal records
```

Liabilities:

```text
Archive purist distrust
verification problems
story bias
```

Recognizes:

```text
street memorials
graffiti testimony
informal names
erased plaques
```

Field Deck bias:

```text
ARCHIVE mode allows oral testimony overlays beside official records.
```

Starting obligation:

```text
An elder gives you a story that contradicts the official waterworks record.
```

Old Waterworks reaction:

```text
The wall says one thing. The old woman outside says another.
```

---

## 48. Festival Builder

You organize public rituals, repair fairs, tool days, charter anniversaries, and seasonal gatherings.

Core fantasy:

```text
Joy as anti-collapse.
```

Strengths:

```text
morale
recruitment
public participation
belief bridging
```

Liabilities:

```text
dismissed until morale breaks
resource demands
political symbolism disputes
```

Recognizes:

```text
festival spaces
public banners
ritual infrastructure
unused gathering sites
```

Field Deck bias:

```text
CIVIC mode highlights morale, participation, and public trust.
```

Starting obligation:

```text
The next public gathering depends on water being restored.
```

Old Waterworks reaction:

```text
When water comes back, people need more than pressure. They need a reason to gather.
```

---

## 49. Children’s Repair Mentor

You teach children safe tools, valve names, signal basics, and civic responsibility.

Core fantasy:

```text
Intergenerational repair.
```

Strengths:

```text
education continuity
future repair capacity
public trust
safe tool practice
```

Liabilities:

```text
overprotectiveness
low tolerance for reckless shortcuts
emotional stakes around children
```

Recognizes:

```text
training diagrams
child-height tool boards
old lesson signs
unsafe learning gaps
```

Field Deck bias:

```text
DIAG mode breaks complex systems into teachable steps.
```

Starting obligation:

```text
A child asks if the adults really know how the pump works.
```

Old Waterworks reaction:

```text
A machine no child can understand becomes a future lock.
```

---

## 50. Memory Artist

You turn flood lines, worker marks, broken tools, lost names, and repair scars into public art.

Core fantasy:

```text
Beauty that refuses forgetting.
```

Strengths:

```text
morale
visible memory
belief bridging
public grief processing
```

Liabilities:

```text
accused of beautifying trauma
low formal authority
conflict over representation
```

Recognizes:

```text
hidden scars
painted-over marks
memorial objects
aestheticized propaganda
```

Field Deck bias:

```text
ARCHIVE mode highlights visible scars, erasures, and public memory conflicts.
```

Starting obligation:

```text
You are asked to create a memorial before the truth is fully known.
```

Old Waterworks reaction:

```text
Do not repaint the scar until we understand it.
```

---
