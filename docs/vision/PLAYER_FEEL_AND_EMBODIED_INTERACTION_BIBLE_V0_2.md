---
title: Player Feel and Embodied Interaction Bible
version: 0.2
status: canonical-draft
scope: Seedworks first-person body, tools, repair, combat, co-op embodiment
milestone: seedworks-regional-slice
owner: design/animation/audio/engineering/accessibility
canon_dependencies:
  - Seedworks Regional Civilization Slice
  - Symtropy Design Doc - Cybernetic Crafting & Physical Node Assembly
  - Symtropy Design Doc - Death, Reconstitution, and Source-Chain Recovery
  - Symtropy Vehicle & Mobility Design Bible
---

# Symtropy Player Feel and Embodied Interaction Bible

## Working Title

**The World Answers Through the Hands**

## Core Thesis

Symtropy’s civilization systems only matter if the player can feel them through a believable body.

The first-person experience must communicate:

- weight without sluggishness;
- precision without fragility;
- tactility without busywork;
- danger without constant combat;
- technical competence without requiring real-world trade expertise;
- bodily limitation without making movement unpleasant.

The primary embodied fantasy is:

> I connected my instrument to a broken civilization, and the world answered.

## Prime Directive

## Version 0.2 Scope Expansion

The first version emphasized patch cables and repair because those interactions are an important tactile proof.

They are not the center of the entire game.

This version requires the embodied model to support:

```text
exploration
combat
vehicles
heavy construction
scientific fieldwork
ecological interaction
hazard survival
social presence
space and alien environments
```

Prime correction:

```text
The hands do not only repair civilization.
They climb, carry, fight, build, discover, drive, heal, and make culture.
```


Every core interaction must have four layers:

1. **Intention** — the player knows what they are trying to do.
2. **Contact** — the body or tool visibly meets the world.
3. **Resistance** — the world provides weight, alignment, friction, danger, or uncertainty.
4. **Response** — the system answers through motion, sound, state, and consequence.

An interaction that skips contact becomes a menu action.

An interaction that adds resistance without meaningful response becomes busywork.

# 1. Design Pillars

## 1.1 Reliable Before Realistic

The player must trust controls before appreciating simulation.

Priorities:

1. stable input;
2. readable targeting;
3. forgiving alignment;
4. immediate feedback;
5. believable animation;
6. deeper physical nuance.

Realism may never justify:

- dropped inputs;
- pixel hunting;
- motion sickness;
- unclear ownership of failure;
- animations that lock the player during danger without warning.

## 1.2 Hands Are the Primary Interface

The player’s hands should visibly:

- raise and brace the Field Deck;
- unclip and route cables;
- hold panels open;
- wipe contamination from labels;
- tighten fasteners;
- carry parts;
- stabilize injured people or machines;
- signal to co-op partners;
- transition between tool and weapon states.

The hands should not constantly perform decorative animation. They should clarify intention and contact.

## 1.3 Tools Have Bodies

Every important tool has:

- weight;
- power state;
- heat or wear where relevant;
- grip and working end;
- sound signature;
- failure cues;
- storage or carry position;
- relationship to the Field Deck.

A tool is not a cursor skin.

## 1.4 Failure Must Be Diagnosable

The player should be able to tell whether an action failed because of:

- bad alignment;
- missing material;
- insufficient authority;
- unsafe pressure;
- damaged tool;
- environmental interference;
- hostile interruption;
- an inaccessible system state.

The game must never collapse all failure into a red error sound.

## 1.5 Competence Is Expressive

Skilled play should create differences in:

- speed;
- material waste;
- repair quality;
- evidence preservation;
- noise generation;
- safety;
- ability to work under pressure.

Skill should not determine whether basic actions are possible. It determines how cleanly, safely, and resourcefully they are completed.

# 2. First-Person Body Model

## 2.1 Embodiment Goals

The body should feel:

- grounded on uneven surfaces;
- capable of carrying real objects;
- vulnerable to heat, water, pressure, and impact;
- able to brace and work;
- socially present to NPCs and other players.

The body should not feel:

- like a free-floating camera;
- excessively heavy;
- constantly stumbling;
- locked into long animations;
- disconnected from carried objects.

## 2.2 Visible Body Scope

For the vertical slice:

Required:

- hands and forearms;
- partial torso/chest rig when looking down;
- feet or lower-body grounding where feasible;
- visible Field Deck lanyard or mount;
- carried-object contact points;
- shadow consistent with body pose.

Deferred:

- fully simulated clothing layers;
- complex body customization effects;
- persistent wound visualization;
- full-body first-person parkour.

## 2.3 Stance States

```rust
pub enum BodyStance {
    Standing,
    Crouched,
    Braced,
    Wading,
    Climbing,
    CarryingLight,
    CarryingHeavy,
    Working,
    Downed,
}
```

Stances modify:

- speed;
- camera response;
- tool stability;
- noise;
- stamina or exertion;
- available actions.

Transitions must be interruptible unless safety requires commitment.

## 2.4 Camera Rules

Default camera behavior:

- low-amplitude movement tied to actual acceleration;
- no constant sinusoidal head bob;
- landing response proportional to drop;
- tool recoil separated from whole-camera recoil;
- collision prevention without aggressive camera snapping;
- horizon stability during ordinary walking.

Options:

- camera motion 0–100%;
- independent weapon/tool sway;
- FOV slider;
- sprint FOV effect toggle;
- camera roll toggle;
- screen shake by source category.

# 3. Locomotion

## 3.1 Walking

Walking is the default exploration speed, not a punishment.

It must support:

- fine panel approach;
- conversational movement;
- stable scanning;
- stepping over small debris;
- shallow-water resistance;
- co-op formation.

Small obstacles should be automatically stepped when safe and visually plausible.

## 3.2 Sprinting

Sprinting is for urgency, escape, and open traversal.

Rules:

- no arbitrary exhaustion after a few seconds;
- carrying and terrain affect duration;
- the player can lower or secure the Field Deck while sprinting;
- sprint cannot silently cancel a critical tool action;
- sound and breathing communicate exertion without obscuring dialogue.

## 3.3 Crouching and Bracing

Crouch supports:

- low access panels;
- stealth;
- waterline inspection;
- reduced tool sway.

Brace is a contextual work stance used when:

- applying torque;
- cutting resistant material;
- holding a cable under tension;
- firing a heavy tool or weapon;
- stabilizing another player.

Brace should feel like gaining leverage, not losing control.

## 3.4 Climbing and Mantling

Vertical-slice scope:

- ladder use;
- waist-to-chest-high mantle;
- stepping onto stable service platforms;
- assisted co-op climb optional.

Rules:

- valid surfaces are visually legible;
- carried heavy objects block or alter mantle;
- climb initiation is generous;
- dangerous drops require deliberate input;
- the player may look around during ladders unless animation constraints forbid it.

## 3.5 Wading and Water

Water is a central material, so movement through it must be legible.

Depth bands:

- wet surface;
- ankle;
- knee;
- waist;
- swimming, deferred for the first slice unless required.

Effects:

- drag;
- sound change;
- tool safety warnings;
- current force where authored;
- contamination exposure;
- reduced visibility of footing.

The player must always know whether a tool can be safely used in the current water state.

# 4. Interaction Targeting

## 4.1 Interaction Cone

Use a forgiving spatial interaction cone, not a single center pixel.

Ranking inputs:

- distance;
- facing;
- current task context;
- object salience;
- hand availability;
- hazard state.

The game should favor the panel handle over the wall behind it and the cable socket over decorative bolts.

## 4.2 Affordance Language

Affordances are communicated by:

- physical shape;
- wear marks;
- tool response;
- subtle reticle change;
- Field Deck contextual hint;
- hand pre-positioning;
- sound.

Avoid glowing every usable object.

Highlight modes may be offered as accessibility options.

## 4.3 Interaction Commitment Classes

### Instant

Examples:

- press button;
- collect small loose item;
- toggle local switch.

### Brief Contact

Examples:

- open panel;
- insert fuse;
- connect cable;
- turn valve.

### Sustained Work

Examples:

- weld seam;
- cut root;
- calibrate sensor;
- revive ally.

### Staged Assembly

Examples:

- place frame;
- stage materials;
- attach components;
- initialize node.

The UI and animation must clearly signal the commitment class before activation.

# 5. Field Deck Embodiment

## 5.1 Physical States

```rust
pub enum DeckPhysicalState {
    Stowed,
    ChestLow,
    RaisedRead,
    RaisedScan,
    CableDeploying,
    CableConnected,
    Shared,
    PanicDropped,
    Damaged,
}
```

## 5.2 Raise and Lower

The Deck should raise quickly enough for repeated use.

Targets:

- readable state in approximately 250–400 ms;
- full stable inspection in approximately 500–700 ms;
- immediate lowering on combat or movement demand;
- toggle and hold modes supported.

The player may begin reading before the animation fully settles.

## 5.3 Cable Interaction

The cable is a hero interaction.

Required phases:

1. identify port;
2. reach toward Deck mount;
3. unclip connector;
4. extend cable with visible slack;
5. align to socket using forgiving snap volume;
6. feel resistance at incorrect orientation;
7. seat connector with mechanical and audio confirmation;
8. show negotiated connection state;
9. preserve cable line in world while connected;
10. disconnect deliberately or through emergency breakaway.

Failure cues:

- incompatible socket;
- corroded contacts;
- unsafe voltage;
- no local power;
- authority handshake rejected;
- cable under tension.

The connector must never teleport silently into place.

## 5.4 Panic Drop

When danger interrupts Deck use:

- the player may drop it to chest-low on its lanyard;
- the screen remains physically present but less readable;
- active connection persists if cable slack permits;
- abrupt movement may break the connection safely;
- private screens are not automatically exposed to nearby players.

Panic Drop is not a punishment animation. It preserves continuity between technical work and danger.

# 6. Inspection and Scanning

## 6.1 Scan Loop

1. Raise Deck.
2. Aim broad sensor field.
3. Acquire target or region.
4. Receive observation confidence.
5. Hold or move to improve reading.
6. Switch mode only when a different question is useful.
7. Mark or compare evidence.

## 6.2 Scanning Must Not Freeze Play

While scanning, the player may:

- walk slowly;
- look away;
- lower Deck;
- receive warnings;
- ask a partner to view shared output.

Only deep calibration interactions may constrain movement.

## 6.3 Confidence Through Motion

Read quality may improve through:

- proximity;
- stable aim;
- multiple angles;
- cable connection;
- matching archive evidence;
- clean sensor surface.

It must not require arbitrary wait bars.

## 6.4 Material Responses

Different targets should feel different under inspection:

- metal gives resonance and heat information;
- water gives flow, conductivity, and contamination estimates;
- roots give living response, moisture, and uptake patterns;
- electronics give signal, power, and timing;
- archives give provenance and gaps;
- Null systems give disagreement between reported and observed state.

# 7. Carrying and Object Weight

## 7.1 Carry Classes

### Pocket

Small parts and evidence items.

### One-Hand

Tools, small components, samples.

### Two-Hand

Conduit sections, battery packs, cases.

### Team Carry

Large pump parts, injured bodies, heavy machinery.

### Mechanically Assisted

Gantry, cart, rover, cable hoist.

## 7.2 Weight Communication

Weight is communicated through:

- hand position;
- acceleration response;
- footfall;
- turn inertia;
- breathing;
- object collision;
- need to set down safely.

Do not make carried objects oscillate or collide unpredictably with doorframes.

## 7.3 Setting Down

The player must be able to:

- place deliberately;
- drop in emergency;
- lean against valid support;
- stage at repair frame;
- hand to another player.

Important parts should not roll into inaccessible geometry.

## 7.4 Co-op Handoff

Handoffs use a short shared interaction:

- receiver signals readiness;
- ownership transfers at contact;
- network authority follows the object;
- either player can cancel before transfer;
- latency smoothing must not duplicate or drop the item.

# 8. Tool Families

## 8.1 Repair Tool

Functions:

- open service fasteners;
- clean corrosion;
- torque bolts;
- basic weld or seal attachment depending on head;
- read local material response.

Design goal:

One early tool should support multiple understandable tasks without becoming a magic omni-tool.

## 8.2 Cutting Tool

Functions:

- cut metal, cable, root, or panel material with appropriate attachment;
- generate heat, noise, and contamination risk;
- preserve or destroy evidence depending on use.

The game must make cutting living material feel different from cutting metal.

## 8.3 Sealant Injector

Functions:

- fill cracks;
- create temporary water or pressure seal;
- show cure state;
- expose material compatibility.

## 8.4 Torque Tool

Functions:

- fastener sequence;
- torque window;
- over-torque damage;
- under-torque leakage or vibration.

Accessibility mode may automate fine timing while retaining sequence and consequence.

## 8.5 Weapon/Defense Tool

The first slice should prioritize a practical defense tool over a large arsenal.

Possible functions:

- short-range pulse;
- drone motor disruption;
- cable cutting;
- panel overload;
- conventional damage as fallback.

It should bridge combat and infrastructure interaction.

# 9. Physical Repair Grammar

## 9.1 Repair Phases

```text
Expose
Diagnose
Isolate
Prepare
Align
Attach
Seal
Initialize
Authorize
Test
```

Not every repair uses every phase, but the order should remain conceptually stable.

## 9.2 Meaningful Inputs

Good repair inputs:

- choose isolation point;
- decide whether to preserve contaminated evidence;
- align to real anchors;
- control tool heat;
- follow fastener sequence;
- test under partial load;
- choose temporary versus permanent authorization.

Bad repair inputs:

- repeat a generic minigame unrelated to the material;
- trace arbitrary shapes;
- hold a button through an unchanging bar;
- solve disconnected pipe puzzles while standing at a real pipe.

## 9.3 Quality Model

```rust
pub struct RepairQuality {
    pub alignment: f32,
    pub seal_integrity: f32,
    pub material_match: f32,
    pub evidence_preservation: f32,
    pub safety_compliance: f32,
    pub maintainability: f32,
}
```

Quality influences:

- efficiency;
- leak or breakdown chance;
- public trust;
- inspection requirements;
- future repair ease;
- sound signature.

## 9.4 Imperfect Success

A rough repair should usually work now and create future risk.

Examples:

- a rough seal leaks during pressure spikes;
- misalignment increases power draw;
- undocumented bypass lowers legitimacy;
- destroyed labels make later diagnosis harder;
- preserved worker marks unlock a witness account.

# 10. Combat Feel

## 10.1 Combat Philosophy

Combat is a dangerous interruption of repair and an alternative form of system interaction.

It should feel:

- fast enough to demand attention;
- readable enough to diagnose;
- materially connected to the room;
- costly without being exhausting;
- avoidable or reducible in some encounters.

## 10.2 Weapon Handling

Rules:

- low input latency;
- clear hit confirmation without arcade excess;
- recoil mostly in weapon and hands, not whole camera;
- repair tools remain useful under threat;
- switching between Deck, tool, and weapon is fast;
- ammunition or charge state is physically and digitally readable.

## 10.3 Enemy Contact

Enemies telegraph through:

- motor sound;
- light behavior;
- movement intent;
- device chatter;
- Field Deck warning when raised;
- environmental response.

Null systems may report green while behaving dangerously. The player learns to trust observation over status color alone.

## 10.4 Damage and Recovery

Damage feedback layers:

- direction;
- body region or system;
- movement effect;
- audio and haptic cue;
- readable health state.

Avoid excessive screen blood, blur, or chromatic distortion that prevents technical interaction.

# 11. Social Embodiment

## 11.1 Presence

NPCs respond to:

- distance;
- facing;
- raised weapon;
- raised Deck;
- carrying a body or evidence;
- interrupting work;
- entering restricted space;
- visible contamination.

## 11.2 Conversation While Working

Conversations should often occur during:

- walking;
- carrying;
- repair;
- waiting for a pressure test;
- watching a public board.

The game should not pull the player into a separate dialogue universe for every exchange.

## 11.3 Gesture Vocabulary

Minimum co-op/NPC gestures:

- look here;
- hold this;
- stop;
- safe/unsafe;
- pass object;
- witness this;
- help lift;
- retreat.

Gestures should have both animation and contextual communication meaning.

# 12. Audio-Haptic Interaction Standard

Every important action needs:

- anticipation cue;
- contact cue;
- state-change cue;
- completion or failure cue.

Examples:

## Cable

- connector rattle;
- orientation scrape;
- seating click;
- relay handshake;
- stable connection hum.

## Valve

- hand contact;
- initial resistance;
- stem movement;
- fluid response;
- pressure stabilization or warning.

## Weld/Seal

- tool spin-up;
- material contact;
- heat or cure transition;
- integrity tone;
- post-action leak check.

Haptics must reinforce, not replace, audio and visuals.

# 13. Accessibility Modes

## Interaction Assistance

- generous snap volumes;
- auto-align option;
- single-input staged interactions;
- reduced timing pressure;
- persistent labels;
- high-contrast affordances;
- hold/toggle selection.

## Motion

- head bob off;
- camera shake categories;
- camera roll off;
- reduced recoil camera motion;
- instant ladder mode if needed;
- stable horizon mode.

## Cognitive Load

- simplified Deck view;
- repeat last instruction;
- pin current causal question;
- separate immediate warning from historical context;
- pauseable evidence review in solo play;
- co-op role prompts.

## Motor Access

- full remapping;
- chord alternatives;
- adjustable hold duration;
- no mandatory rapid tapping;
- left/right hand mirroring where practical;
- controller, keyboard/mouse, and adaptive-device support.

# 14. Telemetry and Playtest Measures

Measure:

- time to first successful interaction;
- failed targeting attempts per object;
- cable insertion retries;
- cancellation rate during long actions;
- time spent reading Deck while unsafe;
- accidental tool/weapon switches;
- repair-quality distribution;
- motion-sickness reports;
- accessibility-option use;
- whether players attribute failure correctly.

Do not optimize only for speed. Some pauses are evidence of thought and should be preserved.

# 15. Acceptance Criteria

The embodied interaction layer is ready for the vertical slice when:

- walking and looking remain comfortable for a full playtest session;
- a first-time player opens and connects the Waterworks panel without pixel hunting;
- the cable visibly persists and responds to movement;
- the player can transition from diagnosis to danger and back without losing context;
- repair quality reflects understandable actions;
- heavy objects feel different from tools without becoming frustrating;
- imperfect repairs produce readable consequences;
- solo and co-op handoffs cannot duplicate critical items;
- all critical interactions have non-color feedback;
- testers describe the Field Deck as an object they used, not a menu they opened.

## Final Rule

> The simulation begins where the player’s hand meets resistance.

# 20. Mobility, Vehicles, and Piloting

## 20.1 Vehicle Entry

Entering a vehicle should communicate:

```text
ownership or permission
mass
seat role
available controls
machine condition
```

Avoid teleporting directly into an abstract driving camera where the vehicle body disappears.

## 20.2 Driving Feel

Ground vehicles need:

```text
suspension response
surface identity
traction loss
load effects
audible strain
damage feedback
```

Simulation should serve readable handling rather than punishing realism.

## 20.3 Crew Stations

Larger vehicles and spacecraft support embodied stations:

```text
driver or pilot
navigation
engineering
cargo
sensors
defense
care
```

Co-op stations must provide active decisions, not passive gauges.

# 21. Combat Embodiment

Combat requires:

```text
reliable aim
clear recoil
readable impacts
fast tool transitions
movement under pressure
injury feedback
rescue interactions
```

Weapons should not feel like secondary props attached to a repair simulator.

Industrial tools may cross into combat, but dedicated weapons still need distinct excellence.

# 22. Construction at Scale

Heavy construction should move through:

```text
survey
staging
positioning
lifting
alignment
assembly
commissioning
```

The player may use cranes, gantries, drones, vehicles, or teams.

At larger scales, embodiment comes from operating meaningful machines rather than manually welding every seam.

# 23. Environmental Bodies

Movement and interaction should change across:

```text
high gravity
low gravity
vacuum
deep water
toxic atmosphere
heat
cold
wind
pressure
alien surfaces
```

Environmental identity must alter route, equipment, animation, and sound.

# 24. Expressive Non-Work

The embodied system must support:

```text
sitting
eating
playing music
dancing
greeting
mourning
resting
watching
helping
```

Civilization cannot be represented only through labor animations.

# 25. Expanded Acceptance Test

The player-feel layer passes only when playtesters can name satisfying examples of:

```text
movement
tool use
combat
vehicle operation
construction
discovery
social presence
```

No single interaction should carry the whole game.
