---
title: Road Choir Bridge Citizen Appeal Courts v0.1
status: canonical-draft
project: Symtropy
domain: Road Choirs / Mobile Accountability / Appeal Systems / Host-Right Network
recommended_path: docs/earth-atlas/southern-africa/ROAD_CHOIR_BRIDGE_CITIZEN_APPEAL_COURTS_V0_1.md
extends:
  - ROAD_CHOIR_MOBILE_ACCOUNTABILITY_MECHANICS_V0_1.md
  - ROAD_CHOIR_CONVOYS_INTERIOR_LIFE_V0_1.md
---

# Road Choir Bridge Citizen Appeal Courts v0.1

## Working Title

**Where the Road and the Town Can Both Be Heard**

## Purpose

The Road Choir accountability system defines:

```text
Host-Right Network
Route Credit Escrow
Vehicle Witness Bonds
Bridge Citizen Arbitration
```

This document specifies the appeal authority.

Problem:

```text
Road Choirs move.
Settlements stay.
Harm can happen between them.
Accountability must follow mobility without becoming captivity.
```

Core answer:

```text
Bridge Citizen Appeal Courts
```

These are temporary, mixed, evidence-driven hearings convened at anchor seasons, water exchanges, route disputes, stopping-right conflicts, and host-right violations.

---

# 1. Core Principle

```text
The road cannot be an alibi.
The town cannot be a cage.
```

Bridge Citizen Appeal Courts exist to prevent both failures.

They are not permanent courts.

They are not convoy-only councils.

They are not settlement courts with extra chairs.

They are **mobile-static hybrid hearings** with limited authority.

---

# 2. Convening Authority

An Appeal Court may be convened by any two of the following:

```text
Bridge Citizen
Basin Court Steward
Road Choir Route Elder
Mine-Scar Witness
Host Settlement Representative
Affected Party Representative
Vehicle Witness Bond Custodian
Archive Witness
Field Deck Chronicle packet
```

Emergency convening may occur with one authority if:

```text
child stopping right is overdue
contaminated delivery is active
convoy is being denied emergency water
host settlement is attempting unlawful vehicle seizure
violent retaliation risk is high
```

Emergency courts must receive retrospective witness review.

---

# 3. Court Composition

Minimum court:

```text
one mobile witness
one static witness
one bridge citizen
one affected-party representative
one evidence packet
```

Recommended court:

```text
Bridge Citizen Convener
Convoy Route Witness
Host Settlement Witness
Mine-Scar or technical witness if relevant
Child advocate if stopping rights are involved
Vehicle Witness Bond Custodian
Field Deck / Chronicle record
```

No single faction may hold majority authority.

---

# 4. Jurisdiction

Appeal Courts may hear:

```text
FalseRouteSafety
HostRightViolation
WaterDebtDispute
VehicleScarAlteration
ChildStoppingRightViolation
ContaminatedDelivery
EmergencyAbandonment
RepairNegligence
UnwitnessedRouteDeath
UnlawfulConvoyDetention
UnlawfulVehicleSeizure
HostNetworkBlacklistingDispute
```

They may not hear:

```text
ordinary criminal trials unrelated to route law
permanent settlement citizenship claims
military security proceedings
private revenge claims without evidence
collective guilt claims against all mobile people
```

---

# 5. Authority Limits

Appeal Courts can:

```text
downgrade or restore vehicle witness bonds
hold nonessential route credit in escrow
require route-song amendment
require vehicle scar ledger correction
require public apology or public correction
require restitution through repair or water delivery
require stopping-hearing completion
require host-right restoration
issue Chronicle notice
refer dead-authority or corporate capture evidence to Basin Court
```

Appeal Courts cannot:

```text
deny emergency water
force permanent settlement
confiscate identity records
seize a survival vehicle without substitute shelter
collectively punish children
ban a convoy permanently without appeal
erase route citizenship
force a child to remain with or leave a convoy without child-specific hearing
```

Design rule:

```text
The court may slow movement.
It may not turn movement into hostage status.
```

---

# 6. Appeal Court Schema

```rust
struct BridgeCitizenAppealCourt {
    court_id: CourtId,
    convening_reason: AppealReason,
    convened_at: SiteId,
    convened_during: Option<AnchorSeasonId>,

    mobile_witnesses: Vec<WitnessRef>,
    static_witnesses: Vec<WitnessRef>,
    bridge_citizens: Vec<BridgeCitizenRef>,
    affected_parties: Vec<ActorRef>,

    claim_set: Vec<RoadChoirClaim>,
    evidence_packet: EvidencePacket,
    field_deck_snapshot: FieldDeckSnapshot,

    emergency_status: bool,
    authority_limits: Vec<AuthorityLimit>,
    ruling: Option<AppealRuling>,
    chronicle_event: Option<EventId>,
}
```

---

# 7. Evidence Packet

```rust
struct AppealEvidencePacket {
    vehicle_scar_records: Vec<VehicleScarRecord>,
    route_song_variants: Vec<RouteSongRecord>,
    host_right_logs: Vec<HostRightLog>,
    water_credit_records: Vec<RouteCreditRecord>,
    toxic_samples: Vec<EvidenceRef>,
    child_preference_records: Vec<StoppingHearingRecord>,
    field_deck_scans: Vec<FieldDeckSnapshot>,
    witness_testimony: Vec<WitnessRef>,
}
```

Evidence priority:

```text
1. Living safety
2. Chain-of-custody samples
3. Vehicle scar ledger integrity
4. Route-song convergence
5. Host-right logs
6. Oral testimony
7. Machine records
8. Reputation claims
```

Reputation claims cannot be decisive without corroboration.

---

# 8. Ruling Types

```rust
enum AppealRulingType {
    ClaimUpheld,
    ClaimRejected,
    SharedFault,
    InsufficientEvidence,
    EmergencyProtectionOrder,
    StoppingHearingRequired,
    HostRightRestoration,
    VehicleBondCorrection,
    RouteCreditEscrow,
    RouteSongAmendment,
    ChronicleAddendumRequired,
}
```

## 8.1 Claim Upheld

The harm claim is validated.

Possible remedies:

```text
route credit escrow
vehicle bond downgrade
restitution delivery
route-song correction
public Chronicle notice
```

## 8.2 Shared Fault

Multiple actors contributed.

Example:

```text
convoy failed to update route song
settlement failed to mark new tailings plume
weather shifted faster than either system recorded
```

Remedy:

```text
shared restitution pool
route hazard update
public dust calendar amendment
```

## 8.3 Insufficient Evidence

No ruling yet.

Outcome:

```text
temporary safety order
required evidence collection
sealed Chronicle note
appeal window
```

## 8.4 Emergency Protection Order

Used when survival rights are threatened.

Examples:

```text
settlement denied emergency water to blacklisted convoy
convoy refusing child's stopping hearing while relying on child labor
```

---

# 9. Where Hearings Happen

Appeal Courts can be convened at:

```text
anchor season camps
Basin Court terraces
water exchange stations
repair villages
Road Choir circle camps
Mine-Scar Witness sampling halls
mobile Field Deck hearing channel
emergency roadside shelter
```

Preferred:

```text
anchor season
```

Reason:

```text
vehicles are stopped
children can attend
archives can be copied
host-rights can be reviewed
repair work can happen nearby
```

---

# 10. Player Role

The player may serve as:

```text
evidence carrier
Field Deck witness
repair expert
route-song verifier
toxic sampler
child advocate
mediator
appeal court participant
```

The player should not automatically be the judge.

Design rule:

```text
The player helps truth become portable.
The world decides what authority it grants that truth.
```

---

# 11. Mission Seed: The Blacklist That Became a Wall

## Setup

A convoy was blacklisted after a false route safety claim.

Now the settlement refuses it emergency water during a heat wave.

The harmed settlement says:

```text
They lied and left people coughing in the dust.
```

The convoy says:

```text
Your blacklist is killing children who did not make that route decision.
```

## Player Tasks

```text
audit the original claim
inspect vehicle witness bond
verify emergency water need
convene Bridge Citizen Appeal Court
separate enforcement from survival denial
issue temporary water protection
resolve or update route credit escrow
```

## Possible Outcomes

### Balanced Ruling

```text
Convoy receives emergency water.
Nonessential route credit remains escrowed.
Vehicle bond stays degraded until route-song correction.
Children protected.
Host settlement retains harm claim.
```

Chronicle:

```text
The Road Was Held, Not Caged
```

### Settlement Overreach

```text
Blacklist used as survival denial.
Host settlement loses trust.
Convoy radicalization risk rises.
```

Chronicle:

```text
The Host Network Became a Wall
```

### Convoy Evasion

```text
Convoy receives emergency water then flees appeal.
Vehicle bond downgraded globally.
Bridge Citizen trust strained.
```

Chronicle:

```text
The Road Ran Past the Hearing
```

---

# 12. Field Deck Readout

```sh
$ read /dev/sym/appeal/bridge_court/current

APPEAL TYPE:
HostNetworkBlacklistingDispute

SURVIVAL LIMIT:
Emergency water may not be denied.

ACTIVE CLAIMS:
FalseRouteSafety / unresolved
EmergencyWaterDenial / active

REQUIRED WITNESSES:
mobile witness: present
static witness: present
bridge citizen: pending
affected child advocate: required

CIVIC QUESTION:
How do you enforce memory without turning water into punishment?
```

---

# 13. Chronicle Interaction

Appeal Court rulings create Chronicle events.

Possible classes:

```text
MobileAccountabilityPrecedent
HostRightRestoration
VehicleBondCorrection
StoppingHearingRequired
ChronicleAddendumRequired
```

Escalation:

```text
One ruling is site-local.
Repeated rulings can regionalize host-right standards.
A ruling recognized by multiple Basin Courts becomes interregional mobile-law precedent.
```

---

# 14. Acceptance Tests

Bridge Citizen Appeal Courts are ready when:

```text
1. A mobile actor can appeal a host-right blacklist.
2. A harmed settlement can pursue a convoy after it moves.
3. Emergency water cannot be used as punishment.
4. Vehicle witness bonds can be downgraded or restored.
5. Children are protected from both convoy labor capture and settlement coercion.
6. The player can convene evidence but does not automatically own the ruling.
7. Rulings can become Chronicle precedents.
```

---

# 15. Final Line

```text
The road and the town both lie when they say only the other one can trap people.
```
