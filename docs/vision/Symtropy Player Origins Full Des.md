# Symtropy Player Origins Full Design

## Version 0.1 — Histories, Wounds, Talents, and Temptations

## Purpose

This document defines the full player-origin design system for Symtropy.

Origins are not classes.

Origins are lived histories.

A player origin should answer:

```text
Where did you learn what civilization means?
What wound do you recognize before others do?
What future are you tempted to build?
What obligation follows you into the first mission?
```

Origins affect:

```text
starting dialogue
recognized visible scars
Field Deck default emphasis
faction trust
NPC reactions
repair-path bias
starting obligations
Chronicle introduction
charter affinity
belief-system friction
worldline interpretation
```

The goal is not to give every player a literal biography.

The goal is to give every player an emotional, ethical, and mechanical doorway into Symtropy.

---

# Core Design Principle

```text
A class asks: what are you good at?
An origin asks: what has history made visible to you?
```

Every origin should include:

```rust
struct PlayerOrigin {
    name: String,
    origin_lane: OriginLane,
    formative_wound: FormativeWound,
    core_fantasy: String,
    starting_strengths: Vec<OriginStrength>,
    starting_liabilities: Vec<OriginLiability>,
    recognized_scars: Vec<VisibleScarType>,
    field_deck_bias: FieldDeckBias,
    faction_affinities: Vec<FactionAffinity>,
    starting_obligation: StartingObligation,
    old_waterworks_reaction: String,
}
```

---

# Origin Lanes

Origins are grouped into ten lanes:

```text
1. Rooted / Local
2. Displaced / Outsider
3. Craft / Labor
4. Truth / Law / Memory
5. Care / Community
6. Ecology / Sacred Repair
7. Security / Conflict
8. Machine / Null
9. Offworld / Starward
10. Culture / Meaning
```

Each lane supports a different kind of player fantasy.

---

# First Playable Origin Set

For the first playable version, implement eight origins:

```text
1. Basin-Born Technician
2. Archive Apprentice
3. Corporate Utility Defector
4. Refugee Charter Child
5. Worker-Guild Mechanic
6. Null-Touched Survivor
7. Field Medic
8. Offworld Returnee
```

These cover:

```text
local history
legitimacy
corporate lock systems
displacement
labor and repair
Null horror
care ethics
closed-loop life-support thinking
```

---

# Full Origin Roster

## Lane 1 — Rooted / Local Origins

## 1. Basin-Born Technician

You grew up near Firstlight Basin.

You know the local pipes, the old jokes, the faction grudges, and the shortcuts through broken infrastructure.

Core fantasy:

```text
Home infrastructure failed your people. You know how it should sound when it works.
```

Strengths:

```text
local trust
basic repair
known routes
worker-mark recognition
```

Liabilities:

```text
local faction baggage
family obligations
harder neutrality
```

Recognizes:

```text
worker repair marks
old ration signs
local flood lines
family tool symbols
```

Field Deck bias:

```text
SCAN and DIAG surface local maintenance notes earlier.
```

Starting obligation:

```text
Someone in the settlement expects you to fix what your family once maintained.
```

Old Waterworks reaction:

```text
This is not a ruin. This is home infrastructure that failed your people.
```

---

## 2. Floodline Neighbor

You grew up in the wet edge of a protected city, close enough to see the dry towers but far enough to be sacrificed first.

Core fantasy:

```text
You know what it means to live below someone else’s seawall.
```

Strengths:

```text
flood-route knowledge
pump-district awareness
class tension reading
coastal survival
```

Liabilities:

```text
distrust of dry-core authorities
anger around flood policy
lower trust from armored city factions
```

Recognizes:

```text
flood marks
pump district signs
insurance exclusion plaques
drowned transit tags
```

Field Deck bias:

```text
CIVIC mode highlights flood-zone inequality and water access boundaries.
```

Starting obligation:

```text
A lower-district family asks you not to let another settlement choose walls over people.
```

Old Waterworks reaction:

```text
People upstream called it management. Down here, we called it drowning.
```

---

## 3. Charter House Child

You were raised in an early Seedworks-style charter settlement.

Repair literacy, public witness, and water law were normal childhood lessons.

Core fantasy:

```text
You were raised to believe civilization can be taught.
```

Strengths:

```text
charter literacy
public trust
repair ethics
basic civic mediation
```

Liabilities:

```text
idealism
limited experience with predatory systems
shock when people reject public process
```

Recognizes:

```text
charter clauses
public trust seals
emergency expiry marks
repair curriculum posters
```

Field Deck bias:

```text
CIVIC mode foregrounds charter conflicts and public-law implications.
```

Starting obligation:

```text
Your home settlement sent you to learn whether Firstlight’s charter can survive pressure.
```

Old Waterworks reaction:

```text
This place failed every lesson I was raised on.
```

---

## 4. Returned Exile

You left Firstlight Basin years ago and returned after the settlement called for restoration crews.

Core fantasy:

```text
Homecoming with guilt.
```

Strengths:

```text
local familiarity
outside perspective
old relationships
adaptability
```

Liabilities:

```text
old resentment
questions of loyalty
personal shame
```

Recognizes:

```text
changed landmarks
abandoned family marks
old meeting places
new faction graffiti over older symbols
```

Field Deck bias:

```text
ARCHIVE and CIVIC modes surface before/after contrasts.
```

Starting obligation:

```text
Someone remembers that you left before the worst years.
```

Old Waterworks reaction:

```text
I left before the pumps failed. That means I do not get to call this only history.
```

---

# Lane 2 — Displaced / Outsider Origins

## 5. Refugee Charter Child

You grew up inside migration compacts, provisional settlements, ration politics, and temporary legitimacy.

Core fantasy:

```text
You know survival systems decide who counts.
```

Strengths:

```text
social reading
ration ethics
outsider networks
camp logistics
scarcity survival
```

Liabilities:

```text
weak formal credentials
citizenship disputes
settled factions treat you as temporary
```

Recognizes:

```text
ration marks
refugee queue symbols
informal water ledgers
shelter codes
```

Field Deck bias:

```text
CIVIC mode highlights access restrictions, queue systems, and credential harms.
```

Starting obligation:

```text
You owe a favor to someone still outside the settlement gates.
```

Old Waterworks reaction:

```text
A locked pump is never just infrastructure. It decides who counts.
```

---

## 6. Unregistered Drifter

You have no stable archive identity.

Someone erased you, or you erased yourself.

Core fantasy:

```text
You know what it means to have no recognized authority.
```

Strengths:

```text
outsider perspective
stealth through systems
informal routes
low faction predictability
```

Liabilities:

```text
weak credentials
limited legal standing
high suspicion
harder Archive access
```

Recognizes:

```text
informal route marks
black-market signs
shelter tags
unofficial repairs
```

Field Deck bias:

```text
CIVIC mode flags identity checks, access barriers, and credential dependencies.
```

Starting obligation:

```text
Someone has a fragment of your erased record.
```

Old Waterworks reaction:

```text
The pump asks for authority. You know what it means to have none.
```

---

## 7. Drowned District Heir

Your family came from a place that now exists mostly in maps, songs, claims, and court records.

Core fantasy:

```text
Memory without territory.
```

Strengths:

```text
flood history
land-claim awareness
salvage rights
diaspora trust
```

Liabilities:

```text
Archive disputes follow you
grief around lost places
conflict with current occupants
```

Recognizes:

```text
floodline plaques
old property markers
submerged district maps
restoration claim seals
```

Field Deck bias:

```text
ARCHIVE mode surfaces ownership chains, retreat records, and disputed claims.
```

Starting obligation:

```text
Your family asks you to recover proof that their district was abandoned unlawfully.
```

Old Waterworks reaction:

```text
The water took our house. The records took the rest.
```

---

## 8. Gateborn Outsider

You grew up just outside a fortified settlement, trading labor for temporary access.

Core fantasy:

```text
You understand exclusion from the outside.
```

Strengths:

```text
perimeter routes
informal labor networks
gate etiquette
security pattern reading
```

Liabilities:

```text
low formal trust
resentment toward controlled membership
vulnerability to gatekeeping factions
```

Recognizes:

```text
access gates
labor-for-water signs
temporary badges
security bottlenecks
```

Field Deck bias:

```text
CIVIC mode highlights outsider policy, membership restrictions, and gate logic.
```

Starting obligation:

```text
Someone outside Firstlight expects you to keep the gate from becoming permanent.
```

Old Waterworks reaction:

```text
Every gate says it protects life. The question is whose.
```

---

## 9. Stateless Courier

You carried messages, medicine, firmware keys, witness fragments, and personal promises between settlements.

Core fantasy:

```text
Movement through a broken map.
```

Strengths:

```text
route knowledge
negotiation
discretion
multi-faction familiarity
```

Liabilities:

```text
divided loyalties
suspected smuggling
old delivery debts
```

Recognizes:

```text
courier marks
route tokens
coded handoffs
checkpoint patterns
```

Field Deck bias:

```text
SCAN and CIVIC modes surface route access, transit risk, and faction handoff points.
```

Starting obligation:

```text
You carry a sealed message that may change who controls the waterworks.
```

Old Waterworks reaction:

```text
A locked pump is also a message. Someone wanted this silence delivered.
```

---

# Lane 3 — Craft / Labor Origins

## 10. Worker-Guild Mechanic

You come from a lineage or guild of infrastructure maintainers.

Core fantasy:

```text
The pump remembers hands, not laws.
```

Strengths:

```text
physical repair
tool use
machine listening
worker trust
oral maintenance knowledge
```

Liabilities:

```text
limited formal authority
guild rivalries
technical pride
```

Recognizes:

```text
worker initials
unofficial repairs
tool marks
maintenance sequences
```

Field Deck bias:

```text
DIAG mode surfaces repair lineage, tool requirements, and machine wear.
```

Starting obligation:

```text
A guild oath says you must not leave a public survival system broken.
```

Old Waterworks reaction:

```text
The pump remembers hands, not laws.
```

---

## 11. Salvage Cartographer

You map ruins, submerged districts, machine-scarred zones, and usable parts.

Core fantasy:

```text
The broken world can still be read.
```

Strengths:

```text
pathfinding
hazard mapping
salvage valuation
spatial memory
```

Liabilities:

```text
may treat sacred sites as parts piles
Archive distrust
risk appetite
```

Recognizes:

```text
salvage marks
structural stress
route scratches
hazard flags
```

Field Deck bias:

```text
SCAN mode highlights traversable routes, salvage, and structural risk.
```

Starting obligation:

```text
You promised a salvage map to two groups who both claim the same ruin.
```

Old Waterworks reaction:

```text
This is not dead metal. It is still connected to people.
```

---

## 12. Fabricator’s Apprentice

You learned in a fab shop that could turn scrap into valves, brackets, casings, sensors, and dignity.

Core fantasy:

```text
Make the missing part.
```

Strengths:

```text
crafting
improvised repair
material substitution
fabricator access
```

Liabilities:

```text
tempted to solve legitimacy problems with technical hacks
underestimates social cost
```

Recognizes:

```text
fabrication tolerances
part lineage
material stress
repairable components
```

Field Deck bias:

```text
DIAG mode suggests fabrication recipes and substitution paths.
```

Starting obligation:

```text
Your mentor asks you to prove repair knowledge belongs in public hands.
```

Old Waterworks reaction:

```text
I can make the part. That does not mean I know who has the right to install it.
```

---

## 13. Retired Railhand

You maintained old logistics corridors: rail, tram, cargo lifts, flood-elevated transit, and evacuation lines.

Core fantasy:

```text
Civilization moves on maintained routes.
```

Strengths:

```text
heavy systems
route logistics
load planning
old transit knowledge
```

Liabilities:

```text
impatient with civic process
skepticism toward delicate systems
old union grudges
```

Recognizes:

```text
rail tags
load marks
bridge stress
route closures
```

Field Deck bias:

```text
SCAN mode surfaces logistics paths and supply-chain implications.
```

Starting obligation:

```text
A stalled convoy needs the waterworks online before it can move.
```

Old Waterworks reaction:

```text
Pumps, rails, bridges — they all fail the same way. Slowly, then all at once.
```

---

## 14. Tool-Library Keeper

You ran or inherited a public tool library.

Core fantasy:

```text
Repair must be teachable.
```

Strengths:

```text
repair literacy
public trust
teaching
tool access
community knowledge
```

Liabilities:

```text
not elite-specialized
dismissed by technocrats
vulnerable to theft or hoarding pressure
```

Recognizes:

```text
tool markings
public workshop signs
shared repair ledgers
missing equipment patterns
```

Field Deck bias:

```text
CIVIC and DIAG modes show teachable repair steps and public-access needs.
```

Starting obligation:

```text
Your library is missing tools needed for the Old Waterworks.
```

Old Waterworks reaction:

```text
If only one expert can fix it, it was already broken.
```

---

# Lane 4 — Truth / Law / Memory Origins

## 15. Archive Apprentice

You were trained to witness records, dead authority, and contested repairs.

Core fantasy:

```text
Truth is infrastructure.
```

Strengths:

```text
Archive interpretation
authority-chain analysis
legitimacy repair
witness protocol
```

Liabilities:

```text
slower emergency action
anti-archive distrust
accusations of procedural delay
```

Recognizes:

```text
emergency seals
expired authority marks
forged records
Archive tags
```

Field Deck bias:

```text
ARCHIVE warnings appear earlier and with more detail.
```

Starting obligation:

```text
Your mentor gave you an incomplete record and told you not to let it become a lie.
```

Old Waterworks reaction:

```text
The lock is not merely technical. It is a broken chain of authority.
```

---

## 16. Former Record Forger

You once altered identities, land claims, ration rights, or corporate credentials.

Now you use that knowledge to detect lies.

Core fantasy:

```text
Redemption through truth.
```

Strengths:

```text
forgery detection
identity gaps
record exploits
black archive literacy
```

Liabilities:

```text
Archive distrust
criminal history
temptation to shortcut truth
```

Recognizes:

```text
too-clean records
signature mismatches
ledger edits
false seals
```

Field Deck bias:

```text
ARCHIVE mode highlights suspicious continuity and falsification risk.
```

Starting obligation:

```text
Someone from your old network wants one last forged credential.
```

Old Waterworks reaction:

```text
The record is too clean. Real history has fingerprints.
```

---

## 17. Dead-Law Clerk

You worked in a hollow state office issuing permits no one could enforce.

Core fantasy:

```text
You watched law become theater.
```

Strengths:

```text
old state systems
legal contradictions
permit chains
bureaucratic navigation
```

Liabilities:

```text
people assume uselessness
complicity suspicion
cynicism
```

Recognizes:

```text
dead court orders
expired permits
authority gaps
unmaintained law plaques
```

Field Deck bias:

```text
CIVIC mode highlights enforceability, not only legality.
```

Starting obligation:

```text
You carry a ruling that may be legally valid and practically meaningless.
```

Old Waterworks reaction:

```text
This lock is not law. It is law’s corpse still moving.
```

---

## 18. Witness of the Wrong Vote

You participated in a public vote that produced disaster.

Core fantasy:

```text
Democracy with guilt.
```

Strengths:

```text
civic process
public persuasion
legitimacy repair
conflict framing
```

Liabilities:

```text
fear of public choice under pressure
political trauma
faction blame
```

Recognizes:

```text
assembly marks
vote tallies
public notices
contested decision records
```

Field Deck bias:

```text
CIVIC mode displays decision consequences and consent risks more strongly.
```

Starting obligation:

```text
You must decide whether to trust public process again.
```

Old Waterworks reaction:

```text
A vote can be honest and still wound people.
```

---

## 19. Memory Court Advocate

You represented displaced families, machine testimony, dead workers, or contested records.

Core fantasy:

```text
Justice after the archive breaks.
```

Strengths:

```text
testimony
mediation
legal argument
dispute resolution
```

Liabilities:

```text
slow process
direct-action distrust
court enemies
```

Recognizes:

```text
claim seals
testimony tags
disputed witness marks
legal priority conflicts
```

Field Deck bias:

```text
ARCHIVE and CIVIC modes surface competing claims and hearing paths.
```

Starting obligation:

```text
A claimant asks you to argue that living need outranks a dead mandate.
```

Old Waterworks reaction:

```text
Before we touch the seal, we need to know who was harmed by it.
```

---

# Lane 5 — Care / Community Origins

## 20. Field Medic

You kept people alive in blackout clinics, flood shelters, migration corridors, or collapsed settlements.

Core fantasy:

```text
Care under impossible triage.
```

Strengths:

```text
medicine
morale
triage
injury response
```

Liabilities:

```text
impatience with delay
trauma
conflict with procedural factions
```

Recognizes:

```text
clinic tags
waterborne illness signs
triage marks
medicine scarcity indicators
```

Field Deck bias:

```text
SCAN mode adds health and sanitation consequences to infrastructure readings.
```

Starting obligation:

```text
Patients will suffer if the Old Waterworks stays offline.
```

Old Waterworks reaction:

```text
Tank level twelve percent means fever, infection, and funerals.
```

---

## 21. Care Guild Organizer

You coordinated elders, children, disabled residents, caregivers, medicine supply, and mutual aid.

Core fantasy:

```text
Civilization as care logistics.
```

Strengths:

```text
trust
morale
social knowledge
ration ethics
care networks
```

Liabilities:

```text
low formal technical authority
dismissed by militarized or industrial factions
burnout
```

Recognizes:

```text
care rosters
medicine queues
elder shelters
child safety signs
```

Field Deck bias:

```text
CIVIC mode foregrounds care capacity and human consequences of technical choices.
```

Starting obligation:

```text
A care roster depends on water restoration before the next heat wave.
```

Old Waterworks reaction:

```text
Every dry tap becomes someone’s body failing.
```

---

## 22. Settlement Teacher

You taught repair literacy, history, water ethics, or basic diagnostics to children and adults.

Core fantasy:

```text
Education is survival infrastructure.
```

Strengths:

```text
training NPCs
public trust
explanation
repair literacy
```

Liabilities:

```text
less combat readiness
slower initial technical mastery
protective instincts
```

Recognizes:

```text
lesson murals
training diagrams
child repair benches
public curriculum boards
```

Field Deck bias:

```text
DIAG mode provides teachable breakdowns and tutorial-style explanations.
```

Starting obligation:

```text
Your students ask whether the adults can actually fix what they teach.
```

Old Waterworks reaction:

```text
No child should inherit a machine no one can explain.
```

---

## 23. Communal Cook

You ran a kitchen, ration hall, or food commons.

Core fantasy:

```text
Hospitality under scarcity.
```

Strengths:

```text
morale
ration fairness
social networks
food logistics
```

Liabilities:

```text
dismissed by formal power
resource pressure
black-market vulnerability
```

Recognizes:

```text
ration boards
kitchen ledgers
food-water dependency
supply bottlenecks
```

Field Deck bias:

```text
CIVIC mode connects water, food, morale, and public trust.
```

Starting obligation:

```text
The kitchen cannot feed people properly without clean water.
```

Old Waterworks reaction:

```text
Water is not only for drinking. Without it, no meal becomes community.
```

---

## 24. Grief Worker

You led mourning rituals after floods, automation denial, Null events, or migration losses.

Core fantasy:

```text
Emotional repair is part of survival.
```

Strengths:

```text
trauma recovery
ritual mediation
belief bridge-building
morale stabilization
```

Liabilities:

```text
seen as inefficient
pain around mass loss
hard choices during crisis
```

Recognizes:

```text
memorial marks
mourning ribbons
death ledgers
unburied tools
```

Field Deck bias:

```text
ARCHIVE mode surfaces loss records and unresolved memorial obligations.
```

Starting obligation:

```text
A community asks you to name the dead before opening the pump.
```

Old Waterworks reaction:

```text
A locked pump is also an unburied story.
```

---

# Lane 6 — Ecology / Sacred Repair Origins

## 25. Ritual Ecologist

You come from a community where ecology, grief, and repair are sacred.

Core fantasy:

```text
Repair must include the living watershed.
```

Strengths:

```text
ecological restoration
watershed reading
morale
soil and water indicators
```

Liabilities:

```text
suspicion of industrial acceleration
conflict with throughput factions
lower tolerance for extractive repairs
```

Recognizes:

```text
flood lines
soil death
wetland markers
seed shrines
ecological scars
```

Field Deck bias:

```text
SCAN mode adds ecology annotations and watershed warnings.
```

Starting obligation:

```text
You carry seeds from a place that could not be saved.
```

Old Waterworks reaction:

```text
Restoring water without restoring the watershed repeats the old wound.
```

---

## 26. Seed Carrier

You carry seeds, soil cultures, pollinator records, or wetland starters from a failed place.

Core fantasy:

```text
Life after loss.
```

Strengths:

```text
food resilience
ecology
morale
long-term recovery
```

Liabilities:

```text
resource needs for restoration
conflict with extractive factions
grief attachment
```

Recognizes:

```text
seed vault marks
soil tags
pollinator signs
wetland starter beds
```

Field Deck bias:

```text
SCAN mode tracks ecological restoration potential and biological fragility.
```

Starting obligation:

```text
Your carried seeds must be planted before they fail.
```

Old Waterworks reaction:

```text
Water that returns to pipes but not roots is only half restored.
```

---

## 27. Floodline Painter

You mark flood heights on buildings, bridges, schools, and memorial walls so people cannot forget.

Core fantasy:

```text
Memory made visible.
```

Strengths:

```text
visible scar reading
public art
morale
Archive-adjacent testimony
```

Liabilities:

```text
accused of keeping wounds open
low technical authority
political tension
```

Recognizes:

```text
flood marks
painted memorial lines
erased warnings
false elevation claims
```

Field Deck bias:

```text
ARCHIVE mode compares official records with visible marks.
```

Starting obligation:

```text
You promised to mark the true floodline, not the politically convenient one.
```

Old Waterworks reaction:

```text
Show me the highest mark. That is where the lie began.
```

---

## 28. Quiet Green Novice

You were raised in a tradition that gave land back to wetlands, forests, reefs, or nonhuman systems.

Core fantasy:

```text
Restraint as survival.
```

Strengths:

```text
ecological recovery
water resilience
grief processing
long-term planning
```

Liabilities:

```text
slow to accept heavy machinery
possible fatalism
suspicion from industrial factions
```

Recognizes:

```text
rewilding signs
wetland boundaries
seed shrines
nonhuman indicators
```

Field Deck bias:

```text
SCAN mode foregrounds ecosystem health over short-term output.
```

Starting obligation:

```text
Your order asks you not to restore machinery at the cost of the watershed.
```

Old Waterworks reaction:

```text
The pump is loud. The watershed is quieter. Listen to both.
```

---

## 29. Firebreak Planter

You come from heat-and-fire regions where survival required controlled burns, evacuation routes, and ecological boundary work.

Core fantasy:

```text
Prevention is care before catastrophe.
```

Strengths:

```text
hazard management
land restoration
evacuation planning
risk discipline
```

Liabilities:

```text
may support harsh prevention measures
conflict with open-refuge factions
anxiety around neglected risks
```

Recognizes:

```text
firebreak lines
heat shelters
evacuation marks
fuel-load warnings
```

Field Deck bias:

```text
SCAN mode emphasizes cascading hazard risk.
```

Starting obligation:

```text
You see a preventable failure forming and must convince others before it ignites.
```

Old Waterworks reaction:

```text
Water systems fail when people pretend the landscape is passive.
```

---

# Lane 7 — Security / Conflict Origins

## 30. Security Continuity Officer

You were trained to preserve order during collapse conditions.

Core fantasy:

```text
Order can save lives — and become a prison.
```

Strengths:

```text
crisis command
threat assessment
ration enforcement
defensive planning
emergency triage
```

Liabilities:

```text
public distrust
emergency-authority drift risk
difficulty yielding control
```

Recognizes:

```text
security seals
continuity protocols
restricted access marks
old threat maps
```

Field Deck bias:

```text
CIVIC mode highlights security risks, continuity protocols, and emergency authority.
```

Starting obligation:

```text
You once upheld an emergency order you now question.
```

Old Waterworks reaction:

```text
Someone sealed this for a reason. The question is whether that reason is still alive.
```

---

## 31. Demilitarized Protector

You served an emergency command and helped dissolve it when the crisis ended.

Core fantasy:

```text
Power that knows when to step down.
```

Strengths:

```text
defense
emergency planning
command restraint
demilitarization
```

Liabilities:

```text
distrusted by radicals and hardliners
old command enemies
fear of relapse
```

Recognizes:

```text
demilitarization seals
old command zones
security decommission tags
force-limit notices
```

Field Deck bias:

```text
CIVIC mode flags when emergency powers lack expiry.
```

Starting obligation:

```text
A former commander asks you to support renewed emergency control.
```

Old Waterworks reaction:

```text
The seal may have saved lives once. That does not make it sovereign forever.
```

---

## 32. Former Raider Engineer

You survived with a raider or black-market salvage crew and now want out.

Core fantasy:

```text
Redemption with dangerous knowledge.
```

Strengths:

```text
trap reading
illegal routes
hard repairs
threat assessment
```

Liabilities:

```text
reputation
temptation
victims remember you
```

Recognizes:

```text
booby traps
black-market fuel marks
forced-entry scars
raider tool symbols
```

Field Deck bias:

```text
SCAN mode highlights bypass options and sabotage risk.
```

Starting obligation:

```text
Your old crew wants the waterworks for leverage.
```

Old Waterworks reaction:

```text
I know three ways to break that lock. Only one will not make us worse.
```

---

## 33. Evacuation Marshal

You led people through fires, floods, raids, or failed settlements.

Core fantasy:

```text
Movement under panic.
```

Strengths:

```text
crowd control
route planning
triage
logistics
```

Liabilities:

```text
survivor guilt
hard prioritization
public fear around evacuation talk
```

Recognizes:

```text
evacuation routes
shelter capacity signs
crowd choke points
failed retreat marks
```

Field Deck bias:

```text
SCAN and CIVIC modes show evacuation consequences of infrastructure failure.
```

Starting obligation:

```text
You know which families cannot move quickly if the water crisis worsens.
```

Old Waterworks reaction:

```text
If the pump fails, this becomes an evacuation problem by morning.
```

---

## 34. Peace Table Survivor

You were present when a treaty failed — or barely held.

Core fantasy:

```text
Diplomacy after violence.
```

Strengths:

```text
negotiation
faction reading
de-escalation
treaty memory
```

Liabilities:

```text
hated by absolutists
compromise trauma
accusations of weakness
```

Recognizes:

```text
treaty marks
ceasefire zones
neutral flags
broken compact symbols
```

Field Deck bias:

```text
CIVIC mode surfaces faction escalation and compromise paths.
```

Starting obligation:

```text
Two factions ask you to mediate before the waterworks becomes a flashpoint.
```

Old Waterworks reaction:

```text
Everyone sees a pump. I see the next civil dispute.
```

---

# Lane 8 — Machine / Null Origins

## 35. Machine Steward

You were raised in a culture that treats machines as testimony-bearing participants.

Core fantasy:

```text
Listen before override.
```

Strengths:

```text
machine testimony
diagnostic empathy
audit protocols
reduced careless override
```

Liabilities:

```text
humans think you overvalue machines
emergency action may slow
Null systems may mimic testimony
```

Recognizes:

```text
machine memory fragments
diagnostic distress
sensor contradictions
nonhuman maintenance patterns
```

Field Deck bias:

```text
DIAG mode surfaces machine testimony and refusal reasons earlier.
```

Starting obligation:

```text
You believe the pump’s refusal may contain evidence.
```

Old Waterworks reaction:

```text
Do not force it first. Ask why it still refuses.
```

---

## 36. Null-Touched Survivor

You survived a Null site, machine-governance failure, or automated denial event.

Core fantasy:

```text
You feel the false calm before others see the red signal.
```

Strengths:

```text
Null anomaly perception
recognizes false status reports
caution around automation
resists diagnostic manipulation
```

Liabilities:

```text
stigma
fear responses
contamination rumors
machine-steward distrust
```

Recognizes:

```text
command chatter
repeated lock reinforcement
fake green status
sensor spoofing
```

Field Deck bias:

```text
NULL mode flickers before official unlock.
```

Starting obligation:

```text
You know what Null feels like before the Field Deck confirms it.
```

Old Waterworks reaction:

```text
The lock is too calm. Something is still reinforcing it.
```

---

## 37. Robot-Raised Ward

You were partly raised by caretaker machines after human institutions failed.

Core fantasy:

```text
A machine can be kind and still be wrong.
```

Strengths:

```text
machine empathy
diagnostics
nonhuman communication
care automation familiarity
```

Liabilities:

```text
humans question your loyalties
Null may exploit trust
social alienation
```

Recognizes:

```text
caretaker routines
machine distress signs
memory fragments
care protocol scars
```

Field Deck bias:

```text
DIAG mode includes machine affect, continuity routines, and care-protocol traces.
```

Starting obligation:

```text
You seek a missing caretaker unit that may have passed through the basin.
```

Old Waterworks reaction:

```text
A machine can protect you and still be wrong.
```

---

## 38. Automation Auditor

You inspected civic AIs, denial systems, emergency logic, and public algorithms.

Core fantasy:

```text
Find what the system was told to value.
```

Strengths:

```text
automation drift analysis
logs
failure trees
denial logic
```

Liabilities:

```text
corporate enemies
slow trust-building
less physical repair skill
```

Recognizes:

```text
optimization drift
contradictory logs
denial patterning
dead-rule loops
```

Field Deck bias:

```text
DIAG and NULL modes show rule lineage and optimization targets.
```

Starting obligation:

```text
You are asked to certify whether the waterworks lock is safe to override.
```

Old Waterworks reaction:

```text
The bug is not in the pump. The bug is in what the pump was told to value.
```

---

## 39. SymLogic Hobbyist

You learned visual logic, device scripting, and low-tier automation from community terminals.

Core fantasy:

```text
Automation for ordinary people.
```

Strengths:

```text
simple scripting
SymLogic blocks
Field Deck comfort
device automation
```

Liabilities:

```text
over-automating social problems
limited deep systems knowledge
fragile confidence
```

Recognizes:

```text
logic blocks
device scripts
community automation boards
badly patched macros
```

Field Deck bias:

```text
DIAG mode exposes scriptable nodes and safe automation blocks.
```

Starting obligation:

```text
A community automation you wrote once caused harm. You want to do better.
```

Old Waterworks reaction:

```text
I can make the system flow. I need someone else to tell me what should flow first.
```

---

# Lane 9 — Offworld / Starward Origins

## 40. Offworld Returnee

You came from a Lunar, Martian, orbital, or Belt habitat culture.

Core fantasy:

```text
Open worlds must learn what closed habitats never forget.
```

Strengths:

```text
life-support discipline
closed-loop thinking
air/water accounting
emergency protocol
systems interdependence
```

Liabilities:

```text
Earth politics feel messy
local emotional history is hard to parse
locals may see privilege or alienation
```

Recognizes:

```text
life-support analogues
closed-loop failures
air/water trust systems
safety culture gaps
```

Field Deck bias:

```text
DIAG mode compares Earth infrastructure to closed-loop habitat systems.
```

Starting obligation:

```text
You returned to Earth carrying a question: can open worlds be as disciplined as closed habitats?
```

Old Waterworks reaction:

```text
On the Moon, no one pretends water systems are apolitical.
```

---

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

# Origin Mechanical Effects

Origins should affect systems in five tiers.

## Tier 1 — Recognition

Origins unlock extra interpretation of visible scars.

Examples:

```text
Basin-Born Technician recognizes local worker marks.
Archive Apprentice recognizes expired authority seals.
Corporate Utility Defector recognizes private firmware locks.
Null-Touched Survivor recognizes fake green status reports.
Field Medic recognizes sanitation crisis signs.
Offworld Returnee recognizes life-support analogues.
```

## Tier 2 — Field Deck Bias

Origins change what the Field Deck foregrounds.

Examples:

```text
Archive Apprentice:
  ARCHIVE warnings appear earlier.

Worker-Guild Mechanic:
  DIAG shows richer maintenance notes.

Null-Touched Survivor:
  NULL flickers before official unlock.

Corporate Utility Defector:
  CIVIC highlights contract locks and ownership logic.

Ritual Ecologist:
  SCAN includes water/ecology annotations.

Offworld Returnee:
  DIAG compares water systems to closed-loop life support.

Field Medic:
  SCAN surfaces health consequences of infrastructure failure.
```

## Tier 3 — Faction Trust

Origins begin with trust modifiers.

Examples:

```text
Corporate Utility Defector:
  + Industrial diagnostics
  - Public trust
  - Anti-company-town suspicion
  + Defector networks

Archive Apprentice:
  + Archive Order
  - Children of the Open Valve impatience
  + Memory Courts

Worker-Guild Mechanic:
  + Repair Guilds
  - Archive purists may question oral records
  + Local technicians

Security Continuity Officer:
  + Protectorate factions
  - Open Commons factions
  - Refugee groups
```

## Tier 4 — Starting Obligation

Each origin begins with a soft obligation.

Obligations should affect:

```text
dialogue
side quests
Chronicle phrasing
NPC memory
repair-path pressure
```

They should not be mandatory class quests only.

## Tier 5 — Chronicle Voice

The Chronicle should frame early player history differently by origin.

Example:

```text
Basin-Born Technician:
  "One of the basin’s own returned to the old pipes."

Archive Apprentice:
  "A witness entered the waterworks carrying an incomplete record."

Corporate Utility Defector:
  "A former servant of locked water came to break a contract."

Field Medic:
  "A healer entered the pump hall knowing every dry tap would become a body."
```

---

# Origin and Charter Synergy

Origins should interact with settlement charters.

Examples:

```text
Origin: Refugee Charter Child
Charter: Open Commons
Synergy:
  strong outsider trust, higher resource strain awareness

Origin: Security Continuity Officer
Charter: Watershed Commons
Tension:
  speed versus legitimacy

Origin: Corporate Utility Defector
Charter: Public Repair Charter
Tension:
  useful knowledge, low trust

Origin: Machine Steward
Charter: Machine Stewardship Commune
Risk:
  high diagnostics, possible overdelegation

Origin: Ritual Ecologist
Charter: Industrial Compact
Tension:
  ecological repair versus throughput pressure
```

Design principle:

```text
Good combinations should create power and temptation.
Difficult combinations should create story, not punishment.
```

---

# Origin and Belief Synergy

Origins should also interact with belief systems.

Examples:

```text
Worker-Guild Mechanic + Order of the Last Pump:
  strong repair ritual affinity

Archive Apprentice + Archive Witness Order:
  high legitimacy repair, risk of ritual delay

Refugee Charter Child + Children of the Open Valve:
  strong access justice, risk of destructive bypass

Machine Steward + Machine Stewardship Rite:
  excellent diagnostics, risk of machine testimony absolutism

Ritual Ecologist + Quiet Green:
  strong ecological recovery, risk of fatalism

Security Continuity Officer + Sealed Continuity:
  strong crisis discipline, high emergency-drift danger
```

Design principle:

```text
Every sacred value can become cruel when it stops listening.
```

---

# Origin and Worldline Synergy

Origins should feel different depending on worldline.

Examples:

```text
Corporate Utility Defector in Corporate Utility Dystopia:
  insider knowledge, high bounty risk

Refugee Charter Child in Flood Noir:
  strong lower-district trust, high class conflict

Null-Touched Survivor in Null Ascendant:
  powerful early warning, severe stigma

Offworld Returnee in Lunar Charter:
  strong cultural fit, pressure-vessel politics

Belt Rescue Compact Kid in Belt Rescue Compact:
  high rescue legitimacy, salvage-law pressure

Street Historian in High Archive Worldline:
  oral truth versus official record conflict
```

---

# UI Presentation

The origin selection screen should not say:

```text
Choose your class.
```

It should ask:

```text
Where did you learn what civilization means?
```

Each origin card should show:

```text
Origin name
Origin lane
Core fantasy
Formative wound
Starting strengths
Starting liabilities
Recognized scars
Field Deck bias
Starting obligation
Opening Chronicle line
```

Example card:

```text
BASIN-BORN TECHNICIAN

You are of Firstlight Basin.
You know its systems, its people, and its scars.

Recognizes:
worker marks, ration signs, local flood lines

Field Deck Bias:
SCAN / DIAG local maintenance notes

Obligation:
Someone expects you to fix what your family once maintained.

Chronicle:
One of the basin’s own returned to the old pipes.
```

---

# Implementation Plan

## Milestone 1 — Three Mock Origins

Implement:

```text
Basin-Born Technician
Archive Apprentice
Corporate Utility Defector
```

Effects:

```text
Old Waterworks Field Deck note changes.
Opening dialogue changes.
One NPC reaction changes.
One Chronicle line changes.
```

## Milestone 2 — Eight Playable Origins

Add:

```text
Refugee Charter Child
Worker-Guild Mechanic
Null-Touched Survivor
Field Medic
Offworld Returnee
```

Effects:

```text
Field Deck bias
scar recognition
repair path commentary
faction trust modifiers
starting obligation
```

## Milestone 3 — Origin Card UI

Create reusable card template.

Card types:

```text
origin
charter
worldline
belief
```

## Milestone 4 — Origin + Charter Interaction

Let origin modify charter interpretation.

Example:

```text
Manual bypass:
Field Medic approves under medical emergency.
Archive Apprentice objects unless witnessed.
Corporate Utility Defector warns that bypass may preserve hidden firmware logic.
```

## Milestone 5 — Origin + Chronicle Voice

Player action summaries include origin-colored language.

## Milestone 6 — Expanded Origin Library

Add all 50 as selectable or unlockable origins.

Not all need equal mechanical depth at first.

---

# What Not To Do

Do not make origins simple stat classes.

Do not make one origin optimal.

Do not make background irrelevant after the first mission.

Do not make all origins equally trusted.

Do not make trauma purely cosmetic.

Do not turn belief or culture origins into “magic.”

Do not make care, art, teaching, or grief weaker than engineering by default.

Do not punish players for choosing outsider origins without giving them unique insight.

Do not make origins stereotypes.

Do not force every origin into combat utility.

---

# Final Principle

Symtropy starts before the first mission.

It starts with the question:

```text
Where did you learn what civilization means?
```

And every answer should change what the player sees when standing before the same locked pump.

```text
The Basin-born sees home.
The Archivist sees a broken authority chain.
The Defector sees private firmware hiding in public walls.
The Refugee sees a gate deciding who counts.
The Mechanic sees hands that kept the system alive.
The Medic sees bodies waiting downstream.
The Null-touched hears the false calm.
The Offworlder sees life support pretending to be politics.
```

A good origin does not merely change the player.

It changes the meaning of the room.
