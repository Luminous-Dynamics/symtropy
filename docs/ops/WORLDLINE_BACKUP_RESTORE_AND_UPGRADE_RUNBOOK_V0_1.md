---
title: Worldline Backup, Restore, and Upgrade Runbook
version: 0.1
status: implementation-spec
scope: operator procedures for backup, verification, restore, migration, rollback, incident response, and server transfer
owner: operations/engine/security
related:
  - tech/WORLDLINE_PERSISTENCE_MIGRATION_AND_DISASTER_RECOVERY_PROTOCOL_V0_1.md
  - tech/MULTIPLAYER_TRUTH_MODEL.md
  - ops/PLAYTEST_RESEARCH_PROGRAM_V0_2.md
---

# Worldline Backup, Restore, and Upgrade Runbook

## Purpose

This runbook defines repeatable operator procedures. Commands are placeholders until the canonical CLI exists, but the sequence and evidence requirements are normative.

# 1. Required CLI Surface

```text
symtropy-world inspect <path>
symtropy-world verify <path>
symtropy-world checkpoint --world <id>
symtropy-world backup --world <id> --destination <uri>
symtropy-world restore --backup <uri> --target <path>
symtropy-world migrate --source <path> --target-version <version>
symtropy-world replay --checkpoint <id> --to <cursor>
symtropy-world fork --world <id> --checkpoint <id> --child <id>
symtropy-world recovery-report <path>
```

# 2. Routine Backup

1. Check worldline health and free storage.
2. Request a consistent checkpoint.
3. Verify checkpoint hashes and domain invariants.
4. Copy to off-host encrypted storage.
5. Re-read the backup manifest from destination.
6. Record backup ID, Chronicle cursor, authority epoch, size, and verification result.
7. Prune only according to retention policy.

Evidence:

```text
backup manifest
verification report
storage destination receipt
operator or automated job identity
```

# 3. Pre-Upgrade Procedure

1. Announce maintenance and expected compatibility boundary.
2. Block new durable transactions or enter controlled drain mode.
3. Complete a verified checkpoint.
4. Complete an off-host immutable backup.
5. Export content, schema, and mod locks.
6. Run migration in a new target directory.
7. Validate invariants and run smoke scenarios.
8. Start a private validation shard.
9. Approve activation.
10. Atomically switch the active worldline pointer.
11. Retain the old bundle for the rollback window.

Do not upgrade directly against the active directory.

# 4. Restore After Crash

1. Preserve the crashed directory and logs.
2. Inspect the last manifest and journal segments.
3. Verify the last valid checkpoint.
4. Replay journal to the last verified event.
5. Quarantine invalid tail or state.
6. Run economic, civic, identity, and reference invariants.
7. Increment authority epoch if any committed future is discarded.
8. Publish a recovery report.
9. Resume in restricted mode if confidence is incomplete.

# 5. Disaster Restore

Use when the active host or storage is unavailable.

1. Provision a clean compatible host.
2. Restore keys through approved recovery procedure.
3. Fetch the newest verified off-host backup.
4. Verify before extraction and again after extraction.
5. Restore pending external synchronization cursors.
6. Start without public clients.
7. Run replay and invariants.
8. Confirm worldline ID and authority epoch.
9. Reopen gradually and monitor reconciliation.

# 6. Administrative Rollback

Rollback is exceptional.

Required decision record:

```text
incident
selected checkpoint
lost time window
affected authority domains
asset and contract implications
player communication
appeal or compensation process
```

Procedure:

1. Stop durable writes.
2. Create an incident fork preserving the abandoned future.
3. Restore selected checkpoint into a new authority epoch.
4. Reconcile external civic and identity systems.
5. Run cross-domain invariants.
6. Publish exact rollback scope.
7. Reopen after review.

# 7. Mod Addition or Removal

## Addition

```text
verify content trust and hashes
verify schema ownership
create pre-change backup
install in validation shard
run migration and smoke tests
activate with locked version
```

## Removal

```text
require uninstall migration for persistent state
otherwise preserve state opaque and open recovery review
never silently strip components
```

# 8. Server Transfer

1. Drain and checkpoint source.
2. Verify and back up.
3. Copy bundle and content lock.
4. Verify destination architecture and supported determinism profile.
5. Restore private shard.
6. Compare invariant and replay reports.
7. Update endpoint trust and connection metadata.
8. Activate destination.
9. Retain source read-only during rollback window.

# 9. Recovery Communication

A public incident summary should state:

```text
what failed
which world time may be lost
whether custody, identity, or Chronicle truth was affected
what recovery path was used
what remains uncertain
what players need to do
```

Do not hide uncertainty behind a generic “maintenance complete” notice.

# 10. Quarterly Recovery Drill

At least quarterly for an active long-lived worldline:

```text
restore one off-host backup to a clean environment
measure RTO
verify economic and civic invariants
load representative player profiles
replay recent journal segments
record discrepancies
```

# 11. Release Gate

No production persistence release is complete until:

- a clean backup and restore has succeeded;
- a crash-tail recovery has succeeded;
- the oldest supported save has migrated;
- a failed migration has left the source untouched;
- asset custody and currency supply reconcile;
- a mod removal path has been tested;
- operator documentation matches the actual CLI.
