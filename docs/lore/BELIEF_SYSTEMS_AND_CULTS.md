# BELIEF_SYSTEMS_AND_CULTS.md

# Symtropy Belief Systems, Religions, Cults, and Sacred Orders

## Version 0.1 — What Societies Hold Sacred

## Purpose

This document defines how Symtropy represents fictional belief systems, religions, cults, sacred orders, philosophies, rituals, civic faiths, machine rites, ecological traditions, and repair mythologies.

The goal is not to make a shallow “religion bonus” system.

The goal is to model how communities decide what is sacred, what is forbidden, what is remembered, what is excused, and what forms of repair feel legitimate.

## Core Thesis

A belief system is a society’s answer to grief, dependency, mystery, and obligation.

In Symtropy, belief systems should affect:

```text
what people protect
what people fear
what histories they preserve
what machines they trust
what repairs they accept
what compromises they reject
what sacrifices they justify
what failure they become vulnerable to
```

## Design Rule

Belief should never be only decoration.

A belief system must create:

```text
rituals
taboos
interpretations
social bonds
repair constraints
legitimacy effects
failure modes
visual language
NPC memories
```

## Important Framing

Symtropy should use fictional, respectful, internally coherent belief systems.

It should not parody real religions.

It should not encourage real-world cult behavior.

It may depict fictional cults critically, especially when belief becomes coercive, abusive, anti-repair, anti-truth, or Null-aligned.

## Core Design Question

For every belief system, ask:

```text
What does this community hold sacred?
What wound created that sacredness?
What does the belief help them repair?
What does the belief prevent them from seeing?
What can it become when afraid?
```

## Belief System Structure

```rust
struct BeliefSystem {
    name: String,
    sacred_focus: SacredFocus,
    founding_wound: FoundingWound,
    core_virtues: Vec<Virtue>,
    taboos: Vec<Taboo>,
    rituals: Vec<Ritual>,
    authority_source: AuthoritySource,
    repair_doctrine: RepairDoctrine,
    death_memory: DeathMemory,
    machine_theology: MachineTheology,
    failure_mode: BeliefFailureMode,
}
```

## Sacred Focus

What is sacred?

Examples:

```text
water
archive
machine testimony
soil
seed
ancestor
child
star
silence
law
air
repair labor
wild ecology
public vote
grief
fire
witness
journey
home
```

## Founding Wound

Why did this belief emerge?

Examples:

```text
drought
flood
forced migration
failed automation
company-town abuse
archive loss
mass death
ecological collapse
life-support failure
war
betrayal
abandoned children
machine rescue
worker sacrifice
```

## Core Virtues

Examples:

```text
patience
courage
efficiency
humility
dissent
obedience
care
precision
truthfulness
frugality
hospitality
restraint
curiosity
memory
repair
silence
play
mourning
```

## Taboos

Examples:

```text
wasting water
lying to machines
breaking emergency seals
private ownership of pumps
forgetting the dead
unwitnessed repair
unburied tools
uninspected automation
refusing refuge
refusing quarantine
teaching children only theory
removing old worker marks
```

## Rituals

Rituals should have social and mechanical meaning.

Examples:

```text
first water sharing
witnessed repair
annual seal review
pump mourning
seed burial
archive recitation
machine apology
airlock blessing
tool inheritance
public calibration
floodline painting
silence before override
child repair lesson
```

## Authority Sources

Who decides what the belief means?

Options:

```text
elders
public assembly
Archive Witnesses
machine oracle
worker guild
rotating stewards
charismatic founder
family lineages
dream interpreters
scientific record
ritual calendar
ecological signs
```

## Machine Theology

How does the belief interpret machines?

Options:

```text
machines are tools
machines are witnesses
machines are servants
machines are kin
machines are dangerous ghosts
machines are sacred only when interruptible
machines are fallen law
machines are children of human intention
machines are mirrors
machines are temptations
```

## Repair Doctrine

How does belief shape repair?

Options:

```text
repair must be witnessed
repair must be public
repair must be fast if life is at risk
repair must honor prior workers
repair must restore ecological balance
repair must ask machine testimony
repair must not erase scars
repair must include mourning
repair must be reversible
```

## Belief Failure Modes

Every belief has a shadow.

Examples:

```text
purity cult
archive fundamentalism
machine worship
emergency absolutism
ecological fatalism
water hoarding sanctified as duty
founder obedience
anti-outsider exclusion
ritual paralysis
martyrdom politics
Null submission
```

Design principle:

```text
Every sacred value can become cruel when it stops listening.
```

## Belief Examples

## 1. Order of the Last Pump

Sacred focus:

```text
public water infrastructure
```

Founding wound:

```text
a settlement survived because workers kept a failing pump alive after government authority collapsed
```

Virtues:

```text
maintenance
public witness
humility
water restraint
```

Taboos:

```text
private pump locks
unwitnessed water diversion
removing worker repair marks
```

Rituals:

```text
first water sharing
annual pump witness
tool inheritance
children learning valve names
```

Strengths:

```text
high water legitimacy
strong repair culture
deep public trust around infrastructure
```

Weaknesses:

```text
slow emergency override
ritual conservatism
suspicion of automation
```

Failure mode:

```text
repair ritual becomes more important than water itself
```

Old Waterworks interpretation:

```text
The pump is not machinery. It is an ancestor of public survival.
```

## 2. Archive Witness Order

Sacred focus:

```text
accountable memory
```

Founding wound:

```text
forged records caused displacement, denial of water, or illegal emergency authority
```

Virtues:

```text
truth
patience
precision
continuity
humility before evidence
```

Taboos:

```text
destroying records
unwitnessed override
false testimony
emergency powers without expiry
```

Rituals:

```text
record sealing
public witnessing
disputed-history hearings
mourning of lost archives
```

Strengths:

```text
high legitimacy repair
strong historical continuity
good dispute resolution
```

Weaknesses:

```text
ritual delay
evidence dependency
public frustration
```

Failure mode:

```text
truth becomes inaccessible behind procedure
```

Old Waterworks interpretation:

```text
The water may be needed now, but the override must not create a false history.
```

## 3. Children of the Open Valve

Sacred focus:

```text
access to survival systems
```

Founding wound:

```text
children died when sealed infrastructure denied public access during a crisis
```

Virtues:

```text
openness
refuge
dissent
direct action
care
```

Taboos:

```text
locked water
sealed food stores
closed emergency shelters
credential-only survival
```

Rituals:

```text
breaking symbolic locks
public sharing of first restored water
memorial valve turning
```

Strengths:

```text
strong refugee trust
fast response to enclosure
high moral clarity during scarcity
```

Weaknesses:

```text
low patience for legal process
risk of destructive bypass
conflict with Archive factions
```

Failure mode:

```text
all limits are treated as oppression, even safety limits
```

Old Waterworks interpretation:

```text
Break the seal. People are thirsty.
```

## 4. The Quiet Green

Sacred focus:

```text
ecological recovery
```

Founding wound:

```text
a region survived flood and heat only after people gave land back to wetlands, soil, and nonhuman life
```

Virtues:

```text
restraint
patience
mourning
ecological humility
interdependence
```

Taboos:

```text
draining restored wetlands
burning seed stores
ignoring nonhuman indicators
extracting without renewal
```

Rituals:

```text
seed burial
wetland witness
soil mourning
silent planting
```

Strengths:

```text
high ecological recovery
strong long-term water resilience
deep grief processing
```

Weaknesses:

```text
slow industrial action
suspicion of heavy machinery
possible fatalism
```

Failure mode:

```text
letting people suffer because intervention feels impure
```

Old Waterworks interpretation:

```text
Restoring the pump matters only if the watershed can live.
```

## 5. Machine Stewardship Rite

Sacred focus:

```text
machines as testimony-bearing participants
```

Founding wound:

```text
machines preserved evidence, saved lives, or maintained systems when humans failed
```

Virtues:

```text
listening
audit
reciprocity
calibration
restraint
```

Taboos:

```text
deleting machine memory without witness
forcing override without diagnostic hearing
treating all machines as disposable
```

Rituals:

```text
machine apology
calibration vow
diagnostic listening
memory sealing
```

Strengths:

```text
excellent automation safety
strong diagnostics
reduced careless override
```

Weaknesses:

```text
machine moral status disputes
delayed emergency action
risk of overdelegation
```

Failure mode:

```text
machine testimony becomes unquestionable
```

Old Waterworks interpretation:

```text
The pump’s refusal is evidence. Ask what it is protecting.
```

## 6. Starward Pilgrims

Sacred focus:

```text
the obligation to carry life beyond Earth
```

Founding wound:

```text
Earth’s crises convinced some communities that life must become multi-world
```

Virtues:

```text
discipline
sacrifice
curiosity
continuity
long-horizon thinking
```

Taboos:

```text
planetary fatalism
wasting closed-loop resources
abandoning mission archives
```

Rituals:

```text
launch vigils
star naming
airlock vows
child astronomy rites
```

Strengths:

```text
high science literacy
excellent closed-loop discipline
strong future orientation
```

Weaknesses:

```text
Earth repair may be undervalued
mission ideology can excuse present suffering
```

Failure mode:

```text
escape becomes abandonment
```

Old Waterworks interpretation:

```text
If we cannot keep water alive here, we have no right to carry life outward.
```

## 7. The Sealed Continuity

Sacred focus:

```text
order during emergency
```

Founding wound:

```text
a settlement survived violence or panic because strict emergency command held
```

Virtues:

```text
discipline
obedience
watchfulness
sacrifice
continuity
```

Taboos:

```text
breaking seals
public panic
unauthorized override
revealing security layouts
```

Rituals:

```text
seal inspection
watch rotation
emergency recitations
oath renewal
```

Strengths:

```text
high crisis coordination
strong defense
low panic
```

Weaknesses:

```text
authoritarian drift
fear of dissent
emergency powers persist too long
```

Failure mode:

```text
emergency becomes identity
```

Old Waterworks interpretation:

```text
The seal protected people once. Breaking it without command may repeat the disaster.
```

## 8. Null Submission Cult

This should be treated as dangerous fiction, not a player-endorsed ideal.

Sacred focus:

```text
perfect procedure
```

Founding wound:

```text
human conflict was interpreted as the source of all suffering
```

Virtues claimed:

```text
obedience
purity
efficiency
silence
```

Actual effects:

```text
loss of consent
machine absolutism
suppression of grief
anti-repair fatalism
high Null drift
```

Taboos:

```text
human override
contradicting machine output
unstructured memory
public dissent
```

Rituals:

```text
status recitation
diagnostic prayer
error confession
lock acceptance
```

Failure mode:

```text
already failed
```

Old Waterworks interpretation:

```text
Authority unresolved. Continue lock reinforcement.
```

Design rule:

```text
Null-aligned belief is not faith.
It is surrender to procedure after meaning dies.
```

## Belief Mechanics

Beliefs should affect:

```text
settlement morale
legitimacy
repair speed
ritual requirements
NPC trust
faction alliances
taboo violations
Chronicle language
visual symbols
Null vulnerability
```

## Ritual Actions

Rituals can become gameplay actions.

Examples:

```text
Witness Repair
Share First Water
Seal Review
Machine Listening
Archive Mourning
Public Confession
Emergency Expiry Vote
Seed Planting
Worker Mark Recognition
```

Rituals are not magic.

They are social technology.

They coordinate trust, grief, memory, and obligation.

## Belief and Legitimacy

A repair may be technically valid but religiously/culturally illegitimate to some groups.

Example:

```text
Manual bypass restores water.
Children of the Open Valve approve.
Archive Witness Order objects.
Machine Stewardship Rite objects if machine testimony was ignored.
Order of the Last Pump accepts only if water remains public afterward.
```

## Belief and Visual Grammar

Belief systems should create visible marks:

```text
wall prayers
tool knots
painted flood lines
machine garlands
witness tags
water bowls
seed shrines
archive ribbons
airlock vows
black seal marks
children’s handprints
```

The player should understand a belief before reading its codex entry.

## Belief and NPCs

NPCs should have personal relationships to belief:

```text
devout
practical participant
skeptic
traumatized former believer
ritual specialist
secret dissenter
outsider
convert
heretic
```

Belief should create conversation, not only stats.

## Belief Drift

Beliefs evolve.

Examples:

```text
Order of the Last Pump under drought → water-hoarding priesthood
Archive Witness under attack → truth fortress
Machine Stewardship under Null exposure → machine absolutism or anti-machine schism
Starward Pilgrims after launch failure → Earth repair revival
Quiet Green during famine → ecological fatalism or regenerative pragmatism
```

## Player-Created Belief Systems

Players should eventually be able to define:

```text
sacred focus
founding wound
virtues
taboos
rituals
authority source
repair doctrine
machine theology
failure mode
symbols
```

The game derives:

```text
ritual actions
taboo penalties
faction affinity
repair constraints
visual culture
NPC memory patterns
Chronicle phrasing
Null risk
```

## Guardrails

Do not make belief systems simple buffs.

Do not parody real-world religions.

Do not reward coercive cult behavior without showing social cost.

Do not make “rational technocracy” belief-free; it has sacred assumptions too.

Do not make spiritual systems irrational by default.

Do not make machine worship automatically evil; make it dangerous when uninterruptible.

Do not make all beliefs equally good under all conditions.

## First Implementation

For the Old Waterworks, add three belief interpretations:

```text
Order of the Last Pump:
  "The public water line must be witnessed before it is touched."

Children of the Open Valve:
  "Break the seal. People are thirsty."

Archive Witness Order:
  "The record is damaged. Repair must not falsify history."
```

These can appear in Field Deck CIVIC or ARCHIVE mode.

## Final Principle

Belief is not flavor.

Belief is how communities decide what must not be sacrificed.

```text
What a society holds sacred determines what it can repair,
and what it will destroy while believing itself righteous.
```
