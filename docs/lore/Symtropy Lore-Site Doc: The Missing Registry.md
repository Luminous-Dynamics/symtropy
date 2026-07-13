# Symtropy Lore-Site Doc: The Missing Registry

## Working Title

**Archive Warfare as Level Design**

## Core Thesis

Archive loss is not background lore in *Symtropy*.

Archive loss is violence.

A population that cannot prove its rights can be denied water, food, shelter, transit, medicine, oxygen, land, citizenship, or repair authority by systems that only recognize valid records.

The Systems War did not only destroy buildings.

It destroyed the paperwork that let people exist inside infrastructure.

Core rule:

```text
If the record is missing, the machine may treat the living as illegitimate.
```

---

# 1. Site Concept

## Location

```text
The Missing Registry
```

A damaged civic-archive facility beneath or adjacent to the Old Waterworks district.

It once stored:

```text
public water access charters
repair authority records
residency credentials
emergency ration exemptions
property boundaries
worker housing agreements
birth and kinship registries
species-vault custody records
settlement council decisions
```

During the Systems War, it was attacked, corrupted, partially burned, and later sealed by dead emergency authority.

The site is not a library dungeon.

It is a survival system whose memory has been weaponized.

---

# 2. Narrative Function

The Missing Registry explains why people can be alive, local, and morally entitled to survival infrastructure, yet still be rejected by the machines.

Example:

```text
A settlement family has lived beside the waterworks for generations.

The pump denies their water claim.

Not because they are enemies.

Because the registry proving their access right was erased in 2091.
```

Design rule:

```text
The tragedy is not that the system hates them.
The tragedy is that the system cannot remember them.
```

---

# 3. Archive Warfare Mechanics

Archive Warfare attacks legitimacy.

Its weapons include:

```text
deleted records
forged records
contradictory records
expired records
orphaned records
unwitnessed records
proprietary records
Null-spoofed records
fragmented source chains
```

Each record has a trust state:

```text
VERIFIED
PARTIAL
CONTRADICTED
EXPIRED
FORGED
MISSING
NULL-SUSPECT
UNWITNESSED
```

The player’s job is not simply to “find lore.”

The player reconstructs enough evidence to change what infrastructure is allowed to do.

---

# 4. Level Premise

The player needs to restore public water access.

The pump says:

```text
DENIED:
Public access charter not found.
Fallback authority: Emergency Water Continuity Act, 2087.
```

The Watershed Commons claims a public charter existed.

Utility Sovereign claims no valid public charter remains.

Continuance systems claim emergency authority still supersedes missing civil records.

The only way to challenge the lock is to enter the Missing Registry and recover enough evidence to prove the public access claim.

---

# 5. Field Deck Reading

## SCAN

```text
Archive stacks damaged.
Thermal scarring detected.
Local storage racks partially collapsed.
Manual index drawers removed.
```

## DIAG

```text
Record fragments remain recoverable.
Source-chain continuity broken across three storage nodes.
```

## ARCHIVE

```text
Registry attack logged during Sovereign Protocol Schism.
Public water charter index missing.
Last intact civic backup: 2091.
```

## CIVIC

```text
Without charter reconstruction, pump authority defaults to emergency rationing protocol.
Public access claim cannot be enforced.
```

## NULL

```text
Two surviving records contain recursive timestamp echo.
Possible forged denial chain.
```

---

# 6. Core Gameplay Loop

The player must reconstruct a civic record from fragments.

Steps:

```text
1. Enter damaged archive.
2. Locate record fragments.
3. Scan physical media.
4. Compare source-chain timestamps.
5. Identify forged or Null-suspect entries.
6. Recover witness fragment.
7. Assemble partial public charter.
8. Return to waterworks.
9. Submit reconstructed claim.
10. Trigger infrastructure adjudication.
```

This connects directly to:

```text
Field Deck
Chronicle
Registered Infrastructure Adjudication
Rights Floor
Old Waterworks
Systems War
```

---

# 7. Evidence Types

The player may recover:

```text
burned paper charter
damaged civic seal
local witness recording
pump maintenance ledger
worker housing map
public ration token
old council minutes
Utility Sovereign license addendum
Continuance emergency override
Archive Witness checksum
```

Each evidence type has different weight.

Example:

```json
{
  "evidence_id": "water_charter_fragment_03",
  "type": "public_access_charter",
  "status": "partial",
  "source_chain": "damaged",
  "date": "2084",
  "supports_claim": "public_water_access",
  "contradicts_claim": "private_utility_exclusivity",
  "null_suspect": false
}
```

---

# 8. Central Conflict

The player cannot recover a perfect record.

They must decide whether partial evidence is enough to restore public water access.

## Option A: Accept Partial Charter

Result:

```text
public water access restored
Utility Sovereign disputes legitimacy
Rights Floor precedent created
risk of future legal challenge
```

Chronicle:

```text
The settlement accepted an imperfect memory because thirst was not waiting for perfect proof.
```

## Option B: Require Full Verification

Result:

```text
legal certainty preserved
water access delayed
Watershed Commons trust falls
Continuance procedure rises
```

Chronicle:

```text
The record remained clean while the pipes stayed closed.
```

## Option C: Use Forged Utility Record

Result:

```text
private pump control restored
water flows through licensed access
public override weakened
Null pressure may rise if forgery was infected
```

Chronicle:

```text
The water returned wearing someone else’s paperwork.
```

## Option D: Invoke Rights Floor

Result:

```text
public survival access temporarily restored
record dispute remains unresolved
Continuance and Utility Sovereign both object
future hearing required
```

Chronicle:

```text
The settlement ruled that missing proof was not proof of nonexistence.
```

---

# 9. Archive Warfare Level Design Scars

The Missing Registry should visually show the war.

Environmental details:

```text
burned civic shelves
sealed file cages
flood-damaged terminals
manual index cards scattered in mud
corporate license vault still powered
public records section destroyed
emergency authority door still locked
old witness booth with cracked glass
dead printer endlessly outputting denial notices
```

The player should understand without exposition:

```text
The private records survived behind better doors.
The public records burned.
```

Design rule:

```text
Architecture should reveal whose memory was protected.
```

---

# 10. Enemies and Hazards

The level should not be a combat gauntlet first.

Primary hazards:

```text
unstable archive floors
electrical water
locked evidence vaults
Null-spoofed terminal prompts
automated denial kiosks
record shredding routine still active
security drone guarding expired records
```

Possible hostile prompt:

```text
ARCHIVE MAINTENANCE:
Public claim fragment corrupted.
Destroy to preserve registry integrity.
```

NULL mode reveals:

```text
NULL:
Destruction request lacks current witness signature.
Pattern resembles wartime archive cleansing routine.
```

---

# 11. Rights Floor Integration

The Missing Registry should introduce a core principle:

```text
A missing record cannot automatically erase a survival right.
```

Rights Floor warning:

```text
CIVIC:
Public water access claim lacks full record support.

RIGHTS FLOOR:
Survival dependency detected.
Absence of record is insufficient grounds for permanent denial.
Temporary access restoration permitted pending review.
```

Design rule:

```text
Rights exist partly to protect people from broken records.
```

---

# 12. Timeline Consistency Note

If the present date is approximately:

```text
2168
```

Then a 2087 emergency authority seal is approximately:

```text
81 years old
```

The Systems War can remain:

```text
2075–2100
```

Recommended correction:

```text
Replace any “55-year Null loop” tied to 2087 with “81-year dead-authority loop.”
```

Alternative:

```text
If a 55-year loop is desired, date that specific loop to 2113.
```

Design rule:

```text
Dates should support dread, not create arithmetic noise.
```

---

# 13. Milestone 1 Placement

The Missing Registry should not be fully implemented before the first pipe repair works.

Milestone 1 should be:

```text
Old Waterworks:
one broken pipe
one patch conduit
one public water access dispute
one missing archive reference
one Chronicle event
```

The Missing Registry becomes Milestone 2 or 3.

Milestone 1 foreshadow:

```text
ARCHIVE:
Public access charter missing.
Registry location known.
Record status: unresolved.
```

This gives the player a future objective without bloating the first slice.

---

# 14. Final Principles

```text
Firmware warfare locks machines.

Metabolic warfare controls flows.

Archive warfare erases claims.

The Missing Registry is where people lost rights without dying.

A broken archive is a weapon that keeps firing.

A record is not truth, but infrastructure often treats it as permission.

Repairing history can restore water.
```

Final line:

```text
The war burned the paper, and the pump called that proof.
```
