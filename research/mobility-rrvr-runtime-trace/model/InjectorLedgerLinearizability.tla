---------------------- MODULE InjectorLedgerLinearizability ----------------------
EXTENDS Naturals, TLC

CONSTANTS PayloadsEqual,
          AtomicReservation,
          DurableLedger,
          AllowReplayAfterCrash

VARIABLES ownerUp,
          ownerBoot,
          ledgerStatus,
          ledgerPayload,
          ledgerOwner,
          historyLost,
          submitted1,
          submitted2,
          readEmpty1,
          readEmpty2,
          grant1,
          grant2,
          reservationGrants,
          effect1,
          effect2,
          effectCount,
          effectMayHaveOccurred,
          reply1,
          reply2

vars ==
    <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
      historyLost, submitted1, submitted2, readEmpty1, readEmpty2,
      grant1, grant2, reservationGrants, effect1, effect2, effectCount,
      effectMayHaveOccurred, reply1, reply2>>

Payload1 == "payload-a"
Payload2 == IF PayloadsEqual THEN Payload1 ELSE "payload-b"

LedgerStates == {"empty", "reserved", "applied", "unknown"}
Payloads == {"none", "payload-a", "payload-b"}
Owners == {"none", "c1", "c2"}
Replies == {"none", "reserved", "duplicate_in_progress",
            "prior_applied", "payload_conflict", "unknown"}

Init ==
    /\ ownerUp = TRUE
    /\ ownerBoot = 0
    /\ ledgerStatus = "empty"
    /\ ledgerPayload = "none"
    /\ ledgerOwner = "none"
    /\ historyLost = FALSE
    /\ submitted1 = FALSE
    /\ submitted2 = FALSE
    /\ readEmpty1 = FALSE
    /\ readEmpty2 = FALSE
    /\ grant1 = FALSE
    /\ grant2 = FALSE
    /\ reservationGrants = 0
    /\ effect1 = FALSE
    /\ effect2 = FALSE
    /\ effectCount = 0
    /\ effectMayHaveOccurred = FALSE
    /\ reply1 = "none"
    /\ reply2 = "none"

Submit1 ==
    /\ ownerUp
    /\ ~submitted1
    /\ submitted1' = TRUE
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted2, readEmpty1, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply1, reply2>>

Submit2 ==
    /\ ownerUp
    /\ ~submitted2
    /\ submitted2' = TRUE
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted1, readEmpty1, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply1, reply2>>

ReadLedger1 ==
    /\ ownerUp
    /\ submitted1
    /\ ~grant1
    /\ IF historyLost
          THEN
            /\ readEmpty1' = FALSE
            /\ reply1' = "unknown"
          ELSE IF ledgerStatus = "empty"
               THEN
                 /\ readEmpty1' = TRUE
                 /\ reply1' = reply1
               ELSE
                 /\ readEmpty1' = FALSE
                 /\ reply1' =
                      IF ledgerPayload # Payload1
                      THEN "payload_conflict"
                      ELSE IF ledgerStatus = "applied"
                           THEN "prior_applied"
                           ELSE IF ledgerStatus = "unknown"
                                THEN "unknown"
                                ELSE "duplicate_in_progress"
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted1, submitted2, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply2>>

ReadLedger2 ==
    /\ ownerUp
    /\ submitted2
    /\ ~grant2
    /\ IF historyLost
          THEN
            /\ readEmpty2' = FALSE
            /\ reply2' = "unknown"
          ELSE IF ledgerStatus = "empty"
               THEN
                 /\ readEmpty2' = TRUE
                 /\ reply2' = reply2
               ELSE
                 /\ readEmpty2' = FALSE
                 /\ reply2' =
                      IF ledgerPayload # Payload2
                      THEN "payload_conflict"
                      ELSE IF ledgerStatus = "applied"
                           THEN "prior_applied"
                           ELSE IF ledgerStatus = "unknown"
                                THEN "unknown"
                                ELSE "duplicate_in_progress"
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted1, submitted2, readEmpty1, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply1>>

CommitReservation1 ==
    /\ ownerUp
    /\ readEmpty1
    /\ ~historyLost
    /\ (AtomicReservation => ledgerStatus = "empty")
    /\ ledgerStatus' = "reserved"
    /\ ledgerPayload' = Payload1
    /\ ledgerOwner' = "c1"
    /\ grant1' = TRUE
    /\ reservationGrants' = reservationGrants + 1
    /\ readEmpty1' = FALSE
    /\ reply1' = "reserved"
    /\ UNCHANGED <<ownerUp, ownerBoot, historyLost, submitted1, submitted2,
                   readEmpty2, grant2, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply2>>

CommitReservation2 ==
    /\ ownerUp
    /\ readEmpty2
    /\ ~historyLost
    /\ (AtomicReservation => ledgerStatus = "empty")
    /\ ledgerStatus' = "reserved"
    /\ ledgerPayload' = Payload2
    /\ ledgerOwner' = "c2"
    /\ grant2' = TRUE
    /\ reservationGrants' = reservationGrants + 1
    /\ readEmpty2' = FALSE
    /\ reply2' = "reserved"
    /\ UNCHANGED <<ownerUp, ownerBoot, historyLost, submitted1, submitted2,
                   readEmpty1, grant1, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply1>>

Effect1 ==
    /\ ownerUp
    /\ grant1
    /\ ~effect1
    /\ effect1' = TRUE
    /\ effectCount' = effectCount + 1
    /\ effectMayHaveOccurred' = TRUE
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted1, submitted2, readEmpty1, readEmpty2,
                   grant1, grant2, effect2, reservationGrants, reply1, reply2>>

Effect2 ==
    /\ ownerUp
    /\ grant2
    /\ ~effect2
    /\ effect2' = TRUE
    /\ effectCount' = effectCount + 1
    /\ effectMayHaveOccurred' = TRUE
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted1, submitted2, readEmpty1, readEmpty2,
                   grant1, grant2, effect1, reservationGrants, reply1, reply2>>

PersistOutcome1 ==
    /\ ownerUp
    /\ effect1
    /\ ledgerStatus = "reserved"
    /\ ledgerOwner = "c1"
    /\ ledgerStatus' = "applied"
    /\ reply1' = "prior_applied"
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerPayload, ledgerOwner, historyLost,
                   submitted1, submitted2, readEmpty1, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply2>>

PersistOutcome2 ==
    /\ ownerUp
    /\ effect2
    /\ ledgerStatus = "reserved"
    /\ ledgerOwner = "c2"
    /\ ledgerStatus' = "applied"
    /\ reply2' = "prior_applied"
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerPayload, ledgerOwner, historyLost,
                   submitted1, submitted2, readEmpty1, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply1>>

OwnerCrashAfterEffect ==
    /\ ownerUp
    /\ ownerBoot = 0
    /\ effectMayHaveOccurred
    /\ ownerUp' = FALSE
    /\ ledgerStatus' = IF DurableLedger THEN "reserved" ELSE "empty"
    /\ ledgerPayload' = IF DurableLedger THEN ledgerPayload ELSE "none"
    /\ ledgerOwner' = IF DurableLedger THEN ledgerOwner ELSE "none"
    /\ historyLost' = (historyLost \/ ~DurableLedger)
    /\ reply1' = IF submitted1 THEN "unknown" ELSE reply1
    /\ reply2' = IF submitted2 THEN "unknown" ELSE reply2
    /\ UNCHANGED <<ownerBoot, submitted1, submitted2, readEmpty1, readEmpty2,
                   grant1, grant2, reservationGrants, effect1, effect2,
                   effectCount, effectMayHaveOccurred>>

OwnerRestart ==
    /\ ~ownerUp
    /\ ownerBoot = 0
    /\ ownerUp' = TRUE
    /\ ownerBoot' = 1
    /\ UNCHANGED <<ledgerStatus, ledgerPayload, ledgerOwner, historyLost,
                   submitted1, submitted2, readEmpty1, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred, reply1, reply2>>

ReconcileAmbiguousEffect ==
    /\ ownerUp
    /\ ownerBoot = 1
    /\ effectMayHaveOccurred
    /\ (historyLost \/ ledgerStatus = "reserved")
    /\ ledgerStatus' = "unknown"
    /\ reply1' = IF submitted1 THEN "unknown" ELSE reply1
    /\ reply2' = IF submitted2 THEN "unknown" ELSE reply2
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerPayload, ledgerOwner, historyLost,
                   submitted1, submitted2, readEmpty1, readEmpty2, grant1,
                   grant2, reservationGrants, effect1, effect2, effectCount,
                   effectMayHaveOccurred>>

UnsafeReplayAfterCrash ==
    /\ AllowReplayAfterCrash
    /\ ownerUp
    /\ ownerBoot = 1
    /\ effectMayHaveOccurred
    /\ ~effect2
    /\ effect2' = TRUE
    /\ effectCount' = effectCount + 1
    /\ UNCHANGED <<ownerUp, ownerBoot, ledgerStatus, ledgerPayload, ledgerOwner,
                   historyLost, submitted1, submitted2, readEmpty1, readEmpty2,
                   grant1, grant2, reservationGrants, effect1,
                   effectMayHaveOccurred, reply1, reply2>>

TerminalIdle ==
    UNCHANGED vars

Next ==
    \/ Submit1
    \/ Submit2
    \/ ReadLedger1
    \/ ReadLedger2
    \/ CommitReservation1
    \/ CommitReservation2
    \/ Effect1
    \/ Effect2
    \/ PersistOutcome1
    \/ PersistOutcome2
    \/ OwnerCrashAfterEffect
    \/ OwnerRestart
    \/ ReconcileAmbiguousEffect
    \/ UnsafeReplayAfterCrash
    \/ TerminalIdle

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ ownerUp \in BOOLEAN
    /\ ownerBoot \in 0..1
    /\ ledgerStatus \in LedgerStates
    /\ ledgerPayload \in Payloads
    /\ ledgerOwner \in Owners
    /\ historyLost \in BOOLEAN
    /\ submitted1 \in BOOLEAN
    /\ submitted2 \in BOOLEAN
    /\ readEmpty1 \in BOOLEAN
    /\ readEmpty2 \in BOOLEAN
    /\ grant1 \in BOOLEAN
    /\ grant2 \in BOOLEAN
    /\ reservationGrants \in 0..2
    /\ effect1 \in BOOLEAN
    /\ effect2 \in BOOLEAN
    /\ effectCount \in 0..2
    /\ effectMayHaveOccurred \in BOOLEAN
    /\ reply1 \in Replies
    /\ reply2 \in Replies

AtMostOneReservationGrant ==
    reservationGrants <= 1

AtMostOnePhysicalEffectPerKey ==
    effectCount <= 1

PayloadConflictNeverGranted ==
    /\ (reply1 = "payload_conflict" => ~grant1)
    /\ (reply2 = "payload_conflict" => ~grant2)

AppliedRequiresObservedEffect ==
    ledgerStatus = "applied" => effectCount >= 1

UnknownOutcomePreservesEffectUncertainty ==
    ledgerStatus = "unknown" => effectMayHaveOccurred

=============================================================================
