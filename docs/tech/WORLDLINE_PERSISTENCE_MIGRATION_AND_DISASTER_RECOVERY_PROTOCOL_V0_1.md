---
title: Worldline Persistence, Migration, and Disaster Recovery Protocol
version: 0.1
status: implementation-spec
scope: saves, snapshots, event journals, schema migration, mod compatibility, rollback, worldline forks, backups, recovery, and retention
owner: engine/networking/data/security
supersedes:
  - tech/WORLD_PERSISTENCE_PROTOCOL.md
related:
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - tech/CHRONICLE_EVENT_SCHEMA.md
  - canon/PLAYER_AUTHORSHIP_SANDBOX_AND_MODDING_CONTRACT_V0_1.md
  - canon/WORLDLINE_LONG_HORIZON_AND_ENDGAME_CONTRACT_V0_1.md
  - tech/ECONOMIC_LEDGER_MARKET_AND_INTEGRITY_RUNTIME_V0_1.md
  - ops/WORLDLINE_BACKUP_RESTORE_AND_UPGRADE_RUNBOOK_V0_1.md
---

# Worldline Persistence, Migration, and Disaster Recovery Protocol

## Owned Question

**How does Symtropy preserve a worldline for years across crashes, upgrades, mods, forks, server moves, corruption, and partial network failure without duplicating authority or discarding the history that gives the world meaning?**

## Core Thesis

A Symtropy save is not one serialized ECS blob.

It is a versioned recovery bundle containing:

```text
world state checkpoints
a causal event journal
content and schema identities
player and institutional authority state
Chronicle history
external or agent-centric synchronization cursors
migration evidence
```

```text
Snapshots make recovery fast.
Journals make recovery explainable.
Schemas make recovery evolvable.
Backups make recovery survivable.
```

## Persistence Prime Directives

1. **No opaque latest.snapshot as the only copy of a world.** A valid save includes integrity metadata and recoverable history.
2. **No schema upgrade without a migration or explicit incompatibility boundary.**
3. **No rollback of one authority domain while dependent economic, civic, or identity state remains in the future.**
4. **No mod removal that silently deletes unknown state.** Unknown components are preserved, quarantined, transformed, or explicitly abandoned with evidence.
5. **No worldline fork that duplicates portable scarce assets back into the same authority domain without a confluence rule.**
6. **No recovery process that rewrites Chronicle history to hide corruption or operator error.** Recovery actions are themselves recorded.
7. **No server owner monopoly over player-exportable identity and authored content where the worldline profile promises portability.**

# 1. Persistence Domains

Symtropy persists several domains with different guarantees.

## 1.1 Local Embodied State

```text
entity transforms
body and vehicle state
active projectiles or transient hazards
local device state
scene streaming state
```

Most transient combat objects may be omitted or normalized at a safe checkpoint.

## 1.2 Regional Simulation State

```text
settlements
routes
resource stocks and flows
ecologies
factions
NPC summaries
campaigns
weather and hazards
```

## 1.3 Device and Construction State

```text
Device Bus topology
automation programs
machine configuration
faults
blueprint ancestry
construction progress
```

## 1.4 Economic and Custody State

```text
assets
batches
rights bundles
currency ledgers
contracts
market escrow
```

This domain requires strict conservation and idempotency.

## 1.5 Civic and Chronicle State

```text
charters
votes
credentials
witness records
laws
public precedents
worldline ancestry
```

## 1.6 Player State

```text
identity references
source-chain cursor
body and equipment
private settings and accessibility
personal maps and notes
relationships and obligations
authored blueprints or content
```

Private player data may have separate encryption and export policies.

# 2. Recovery Bundle Layout

Recommended package:

```text
worldline/
  manifest.json
  checkpoints/
    checkpoint-000042/
      region-index.bin
      regions/
      devices/
      economy/
      civic/
      players/
      checkpoint.hashes
  journal/
    segment-000001.events
    segment-000002.events
  content/
    content-lock.json
    schema-lock.json
    mod-lock.json
  migrations/
    migration-ledger.jsonl
  chronicle/
    chronicle.cursor
    local-mirror/
  recovery/
    incident-log.jsonl
```

Large regions may use independent checkpoint chunks so unchanged areas do not need full rewrites.

# 3. Manifest

```json
{
  "worldline_id": "firstlight.community.001",
  "worldline_ancestry": ["seedworks.template.0"],
  "save_format_version": 1,
  "engine_build": "0.8.0-dev+abcdef",
  "checkpoint_id": 42,
  "journal_head": "segment-000118:932",
  "chronicle_cursor": "event:7d2...",
  "authority_epoch": 19,
  "content_lock_hash": "sha256:...",
  "schema_lock_hash": "sha256:...",
  "created_at": "2168-04-18T20:41:00Z",
  "clean_shutdown": true
}
```

The manifest is written last through atomic rename after all referenced files are durable.

# 4. Checkpoints and Journal

## 4.1 Checkpoint

A checkpoint is a self-consistent state at one authority epoch and Chronicle cursor.

Checkpoint requirements:

```text
chunk hashes
schema versions
content identities
cross-domain consistency marker
authority epoch
journal start cursor
```

## 4.2 Event Journal

The journal records accepted mutations after the checkpoint.

```rust
struct PersistedEventEnvelope {
    event_id: EventId,
    authority_domain: AuthorityDomainId,
    authority_epoch: u64,
    schema_id: SchemaId,
    schema_version: u32,
    causal_parent: Option<EventId>,
    idempotency_key: Option<IdempotencyKey>,
    payload_hash: Hash,
    payload: Bytes,
}
```

Journals are segmented, checksummed, and append-only. Corrupt tails can be truncated to the last verified event with an incident record.

## 4.3 Checkpoint Cadence

Checkpoint triggers may include:

```text
elapsed time
journal size
major Chronicle event
server shutdown
before migration
before mod-set change
before administrative rollback
```

Cadence should balance recovery time, storage, and write cost.

# 5. Crash Consistency

Write sequence:

```text
1. flush domain journals
2. freeze or copy a consistent state view
3. write checkpoint chunks to temporary paths
4. verify hashes and cross-domain invariants
5. write checkpoint metadata
6. atomically update manifest
7. prune only after backup policy permits
```

On boot:

```text
load last valid manifest
verify checkpoint
replay verified journal events
reconcile domains
quarantine invalid assets or unknown components
publish recovery report
```

# 6. Schema Identity and Versioning

Every persisted component and event has:

```text
stable schema ID
major/minor version
owning subsystem
compatibility policy
migration function or retirement rule
```

Changing a Rust type name is not a migration strategy.

## Compatibility Classes

```text
read-compatible      — old data loads without conversion
migrate-compatible   — deterministic migration exists
preserve-opaque      — unknown data can be retained but not interpreted
fork-required        — world may continue only as a new incompatible branch
unsupported          — load refused with recovery guidance
```

# 7. Migration Pipeline

```text
inspect
  → resolve content and schema graph
  → plan ordered migrations
  → create pre-migration backup
  → migrate into a new bundle
  → validate invariants
  → produce migration report
  → activate through atomic pointer change
```

Never migrate the only copy in place.

```rust
trait StateMigration {
    fn from(&self) -> SchemaVersion;
    fn to(&self) -> SchemaVersion;
    fn dependencies(&self) -> &[MigrationId];
    fn migrate(&self, input: MigrationInput) -> Result<MigrationOutput, MigrationError>;
    fn validate(&self, output: &MigrationOutput) -> ValidationReport;
}
```

Migration records include hashes of input and output, tool version, warnings, quarantined state, and operator decision.

# 8. Content and Mod Compatibility

The content lock records:

```text
core content version
mod IDs and versions
content hashes
load order
schema providers
license and trust metadata
```

On missing content:

```text
cosmetic-only content may use a placeholder
behavioral content is preserved opaque or disables affected entities
critical simulation content blocks load or requires an approved migration
```

Removing a mod requires a declared uninstall migration for state it owns. Otherwise the world opens in recovery mode with affected state quarantined.

# 9. Authority Epochs and Rollback

Authority epochs prevent old events from being replayed after administrative restoration.

A restore creates a new epoch:

```text
old future retained as an incident branch
selected checkpoint restored
new authority epoch issued
clients and external bridges resynchronize
compensating or invalidation records published where required
```

## Cross-Domain Rollback

A rollback boundary includes every domain affected by the chosen events.

Example:

```text
Restoring a destroyed convoy also affects:
- cargo custody
- contracts
- campaign supply
- NPC injuries and deaths
- Chronicle outcome
```

The system must restore or compensate all of them, or refuse the rollback.

# 10. Worldline Forks

A fork creates a new ancestry branch with copied state and distinct future authority.

```rust
struct WorldlineForkRecord {
    parent: WorldlineId,
    child: WorldlineId,
    fork_checkpoint: CheckpointId,
    fork_chronicle_cursor: ChronicleCursor,
    portability_policy: PortabilityPolicy,
    scarce_asset_policy: ScarceAssetForkPolicy,
}
```

Worldline-local assets may exist independently in descendants. Portable cross-world assets require ancestry-aware import rules so siblings cannot re-merge duplicated custody accidentally.

# 11. Confluence and Merge

Merging worlds is not generic save-file union.

Confluence requires domain-specific policy:

```text
terrain and construction conflict resolution
NPC identity continuity
asset custody ancestry
charter and law compatibility
Chronicle preservation
mod and schema compatibility
```

Some worlds can exchange history or blueprints without merging physical state.

# 12. External Civic / Agent-Centric Synchronization

The local bundle stores synchronization cursors and pending outbound events, not a fake copy of external consensus.

During outage:

```text
local actions allowed by policy continue
outbound durable events queue idempotently
remote-dependent permissions may degrade
reconnection validates ancestry and authorization
```

# 13. Player Export and Portability

Depending on worldline policy, players may export:

```text
identity references
personal settings
accessibility profile
private map notes
source-chain entries
portable authored blueprints
character appearance
relationship summary with consent constraints
```

Exports include schema and content identities. Private information about other players is minimized or redacted.

# 14. Backup Policy

Recommended tiers:

```text
local rotating checkpoints
same-host separate-volume backup
off-host encrypted backup
periodic verified archival snapshot
pre-upgrade immutable backup
```

Backups are useless until restoration is tested.

Define:

```text
RPO — acceptable lost world time
RTO — acceptable restoration time
retention — number and age of checkpoints
verification cadence
key recovery policy
```

# 15. Corruption and Recovery Mode

Recovery mode may:

```text
load read-only
skip a corrupt journal tail
quarantine invalid assets
replace noncritical missing cosmetics
isolate a damaged region
export diagnostics
fork from last valid state
```

It may not silently invent missing civic signatures, asset custody, or NPC continuity.

# 16. Deletion and Retention

Worldline deletion should distinguish:

```text
active removal
soft-delete retention window
player export period
legal or moderation hold
cryptographic key destruction
public Chronicle preservation
```

A community may end a worldline while preserving its history and player-authored works.

# 17. Security and Privacy

Required protections:

```text
hash verification
signed or authenticated manifests where appropriate
encryption for private player backups
least-privilege operator access
redacted support bundles
secret rotation without rewriting history
```

Do not store raw authentication secrets in snapshots.

# 18. Validation Invariants

After load or migration:

```text
all entity references resolve or are quarantined
asset custody is unique
currency supply reconciles
Chronicle cursor is consistent
journal events are ordered and verified
route and region indices match chunks
player identity references are valid or degraded explicitly
mod-owned state has a provider or opaque preservation record
```

# 19. Test Matrix

Required tests:

```text
crash during checkpoint write
corrupt final journal event
missing region chunk
upgrade across every supported schema version
remove cosmetic mod
remove simulation mod with and without uninstall migration
restore after committed market transaction
restore after worldline fork
server move across operating systems
replay under supported determinism profile
key loss recovery drill
full disaster restore from off-host backup
```

# 20. Acceptance Gates

- a crash loses no more than the declared RPO and produces a recovery report;
- supported old saves migrate through deterministic tested steps;
- migration never overwrites the only copy;
- asset and currency invariants survive restore and rollback;
- unknown mod state is not silently deleted;
- worldline forks preserve ancestry and prevent accidental scarce-asset re-merge;
- operators can restore from an off-host backup within the declared RTO;
- players can export promised portable data;
- recovery actions remain visible in operational history.

## Final Rule

```text
A worldline is a community's memory in executable form.
Treat its continuity as a first-class feature, not a serialization detail.
```
