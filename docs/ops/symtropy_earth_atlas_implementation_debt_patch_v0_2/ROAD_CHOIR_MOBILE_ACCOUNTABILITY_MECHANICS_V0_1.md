---
title: Road Choir Mobile Accountability Mechanics v0.1
status: canonical-draft
project: Symtropy
domain: Earth Atlas / Road Choirs / Mobile Law / Accountability / Vehicle Systems
recommended_path: docs/earth-atlas/southern-africa/ROAD_CHOIR_MOBILE_ACCOUNTABILITY_MECHANICS_V0_1.md
patches:
  - ROAD_CHOIR_CONVOYS_INTERIOR_LIFE_V0_1.md
---

# Road Choir Mobile Accountability Mechanics v0.1

## Working Title

**The Road Cannot Be an Alibi**

## Purpose

The Road Choir interior-life doc specifies stopping rights, route memory, vehicle scar ledgers, and host rights.

This patch resolves the remaining hard problem:

```text
How can mobile actors be held accountable for harm without destroying mobility itself?
```

Road Choirs must not be allowed to use movement as jurisdictional evasion.

But static towns must not use accountability as a trap to immobilize mobile people.

---

# 1. Core Thesis

```text
Mobility is legitimate only when memory can catch up with it.
```

The problem is not that Road Choirs move.

The problem is when harm cannot follow them into a public record.

The reform target:

```text
portable accountability
not static captivity
```

---

# 2. Accountability Instruments

## 2.1 Host-Right Network

A distributed agreement among settlements, Basin Courts, Road Choirs, repair guilds, and Mine-Scar Witnesses.

It records:

```text
safe parking granted
water exchange
repair aid
harm claims
unresolved disputes
route debts
stopping-hearing obligations
vehicle scar ledger audits
```

A convoy that violates host rights may lose access to:

```text
safe parking
public filter exchange
repair bay priority
school anchor sponsorship
emergency passage credit
```

Design rule:

```text
Blacklisting should be appealable and specific, not permanent exile by rumor.
```

## 2.2 Route Credit Escrow

Convoys earn route credits by:

```text
delivering water
evacuating civilians
performing repairs
sharing route hazards
honoring stopping hearings
appearing at dispute hearings
```

Route credits can be held in escrow when harm claims are unresolved.

Use cases:

```text
damaged bridge claim
false route safety claim
unpaid host-water debt
child stopping-hearing violation
contaminated tanker delivery
```

Escrow cannot seize basic survival water.

It can restrict:

```text
bonus fuel priority
repair bay upgrades
preferred parking
long-route contracts
charter sponsorship
```

## 2.3 Vehicle Witness Bonds

Major convoy vehicles carry witness bonds.

A witness bond is a civic status attached to a vehicle's history.

```rust
struct VehicleWitnessBond {
    vehicle_id: VehicleId,
    bond_status: BondStatus,
    route_credit: f32,
    unresolved_claims: Vec<ClaimId>,
    scar_ledger_integrity: f32,
    host_network_trust: f32,
}
```

If a convoy flees accountability, its vehicle bond is marked:

```text
contested
hearing overdue
scar ledger disputed
host-right suspended
```

This follows the vehicle without immobilizing every person onboard.

## 2.4 Bridge Citizen Arbitration

Bridge citizens are recognized in both mobile and static systems.

They can convene:

```text
roadside hearings
remote testimony windows
anchor season dispute councils
vehicle scar audits
child stopping appeals
```

Failure mode:

```text
Bridge citizens may become over-powerful brokers if not rotated and witnessed.
```

---

# 3. Claim Types

```rust
enum RoadChoirClaimType {
    FalseRouteSafety,
    HostRightViolation,
    WaterDebtDispute,
    VehicleScarAlteration,
    ChildStoppingRightViolation,
    ContaminatedDelivery,
    EmergencyAbandonment,
    RepairNegligence,
    UnwitnessedRouteDeath,
}
```

Each claim has:

```text
claimant
convoy
vehicle
site
evidence
urgency
appeal route
possible restitution
```

---

# 4. Enforcement Without Captivity

Allowed enforcement:

```text
route credit escrow
host-right suspension
vehicle witness-bond downgrade
mandatory anchor hearing
repair priority delay
public Chronicle notice
bridge citizen arbitration
specific cargo inspection
```

Disallowed enforcement by healthy charters:

```text
collective punishment of children
denial of emergency water
permanent ban without appeal
vehicle seizure without survival alternative
forced settlement
identity confiscation
```

Design rule:

```text
Accountability may slow a convoy.
It may not turn mobility into hostage status.
```

---

# 5. False Route Safety Mission Mechanic

## Setup

A convoy sold a route as safe.

A smaller convoy followed the route and suffered dust exposure near an unmarked mine plume.

The accused convoy claims:

```text
The song was old.
The wind changed.
The settlement altered the road.
We cannot be blamed for every route.
```

The harmed party claims:

```text
The convoy removed a black ceramic warning bead from the vehicle ledger to keep the contract.
```

## Player Tasks

```text
inspect vehicle scar ledger
compare route song variants
read Dust Calendar wall
interview host settlement
sample road dust
recover removed bead or prove it never existed
convene mobile/static hearing
```

## Possible Findings

### Finding A — Fraud

The convoy knowingly sold false route safety.

Effects:

```text
route credit escrow
vehicle bond downgraded
restitution owed
captain authority challenged
host-right network warned
```

### Finding B — Negligent Memory

The convoy failed to update route song after new toxic data.

Effects:

```text
mandatory route archive update
partial restitution
Mine-Scar Witness training required
```

### Finding C — Static Actor Fault

A settlement or company changed conditions without marking them.

Effects:

```text
convoy cleared
settlement or corporate actor liable
host-right network updated
```

### Finding D — Ambiguous Weather

No one lied, but route risk was underestimated.

Effects:

```text
new dust hazard category
shared restitution pool
route song amendment
```

---

# 6. Stopping Rights Enforcement

If a child requests a Stopping Hearing and the convoy avoids anchor season, the Host-Right Network flags:

```text
stopping_hearing_overdue
child_labor_dependency_high
bridge_citizen_review_required
```

Enforcement:

```text
convoy may lose nonessential route contracts
anchor sponsorship is prioritized
substitute repair labor can be assigned
vehicle bond cannot upgrade until hearing occurs
```

Protection:

```text
the child is not removed by force unless direct harm exists
the convoy is not denied survival water
the hearing must include someone trusted by the child
```

---

# 7. Field Deck Readout

```sh
$ read /dev/sym/convoy/accountability/mercy_axle

CONVOY: MERCY-AXLE SOUTH LOOP
HOST-RIGHT STATUS: ACTIVE / CONTESTED
VEHICLE WITNESS BOND: DEGRADED
UNRESOLVED CLAIMS:
  - FalseRouteSafety / dust exposure
  - StoppingHearingOverdue / roadchild_17

ENFORCEMENT LIMIT:
Emergency water access may not be denied.
Nonessential route credit may be held in escrow.

CIVIC QUESTION:
How do you let the road keep moving without letting harm outrun memory?
```

---

# 8. Chronicle Outcomes

```text
The Road Was Not an Alibi
```

A convoy was held accountable without being trapped.

```text
The Host Network Became a Cage
```

A settlement used accountability claims to immobilize a convoy unfairly.

```text
The Missing Black Bead
```

A vehicle ledger was altered to hide danger.

```text
The Child's Anchor Season
```

A stopping hearing established bridge citizenship precedent.

---

# 9. Acceptance Test

This system succeeds if:

```text
1. A convoy can be held accountable after leaving a site.
2. The enforcement does not deny survival water.
3. Harm claims attach to vehicles, route credits, and witness records.
4. Static towns can also be held accountable for trapping convoys.
5. Children gain stopping-right protection without automatic family destruction.
6. The player can resolve a mobile dispute through evidence, not only combat.
```

---

# 10. Final Line

```text
A road is free only if the people harmed along it can still be heard.
```
