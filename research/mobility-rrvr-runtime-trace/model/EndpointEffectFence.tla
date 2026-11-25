--------------------------- MODULE EndpointEffectFence ---------------------------
EXTENDS Naturals, Integers, TLC

CONSTANTS DurableEndpointNvm,
          AllowUnsafeReplay,
          AllowStaleEpoch

VARIABLES deviceUp,
          deviceBoot,
          authorityEpoch,
          endpointFenceEpoch,
          requestEpoch,
          requestPayload,
          acceptedPayload,
          tokenState,
          historyLost,
          reply,
          effectInvocations,
          physicalEffects,
          staleEffectCount

vars ==
    <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch, requestEpoch,
      requestPayload, acceptedPayload, tokenState, historyLost, reply,
      effectInvocations, physicalEffects, staleEffectCount>>

TokenStates == {"none", "reserved", "applied", "unknown"}
Payloads == {"none", "payload-a", "payload-b"}
Replies == {"none", "reserved", "duplicate_in_progress",
            "prior_applied", "unknown", "payload_conflict", "stale_epoch"}

Init ==
    /\ deviceUp = TRUE
    /\ deviceBoot = 0
    /\ authorityEpoch = 0
    /\ endpointFenceEpoch = 0
    /\ requestEpoch = 0
    /\ requestPayload = "payload-a"
    /\ acceptedPayload = "none"
    /\ tokenState = "none"
    /\ historyLost = FALSE
    /\ reply = "none"
    /\ effectInvocations = 0
    /\ physicalEffects = 0
    /\ staleEffectCount = 0

AdvanceAuthorityEpoch ==
    /\ authorityEpoch = 0
    /\ authorityEpoch' = 1
    /\ UNCHANGED <<deviceUp, deviceBoot, endpointFenceEpoch, requestEpoch,
                   requestPayload, acceptedPayload, tokenState, historyLost,
                   reply, effectInvocations, physicalEffects, staleEffectCount>>

SetRequestToAuthorityEpoch ==
    /\ requestEpoch # authorityEpoch
    /\ requestEpoch' = authorityEpoch
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestPayload, acceptedPayload, tokenState, historyLost,
                   reply, effectInvocations, physicalEffects, staleEffectCount>>

SetRequestToOldEpoch ==
    /\ authorityEpoch = 1
    /\ requestEpoch = 1
    /\ requestEpoch' = 0
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestPayload, acceptedPayload, tokenState, historyLost,
                   reply, effectInvocations, physicalEffects, staleEffectCount>>

SetAlternatePayload ==
    /\ requestPayload = "payload-a"
    /\ requestPayload' = "payload-b"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, acceptedPayload, tokenState, historyLost,
                   reply, effectInvocations, physicalEffects, staleEffectCount>>

FenceEndpointEpoch ==
    /\ deviceUp
    /\ endpointFenceEpoch < authorityEpoch
    /\ endpointFenceEpoch' = authorityEpoch
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, requestEpoch,
                   requestPayload, acceptedPayload, tokenState, historyLost,
                   reply, effectInvocations, physicalEffects, staleEffectCount>>

SubmitNewToken ==
    /\ deviceUp
    /\ ~historyLost
    /\ tokenState = "none"
    /\ requestEpoch = authorityEpoch
    /\ requestEpoch = endpointFenceEpoch
    /\ acceptedPayload' = requestPayload
    /\ tokenState' = "reserved"
    /\ reply' = "reserved"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, historyLost,
                   effectInvocations, physicalEffects, staleEffectCount>>

ReplaySameToken ==
    /\ deviceUp
    /\ tokenState # "none"
    /\ requestEpoch = endpointFenceEpoch
    /\ requestPayload = acceptedPayload
    /\ reply' =
         IF tokenState = "applied" THEN "prior_applied"
         ELSE IF tokenState = "unknown" THEN "unknown"
              ELSE "duplicate_in_progress"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, acceptedPayload, tokenState,
                   historyLost, effectInvocations, physicalEffects,
                   staleEffectCount>>

RejectChangedPayload ==
    /\ deviceUp
    /\ tokenState # "none"
    /\ requestEpoch = endpointFenceEpoch
    /\ requestPayload # acceptedPayload
    /\ reply' = "payload_conflict"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, acceptedPayload, tokenState,
                   historyLost, effectInvocations, physicalEffects,
                   staleEffectCount>>

RejectStaleEpoch ==
    /\ deviceUp
    /\ requestEpoch < endpointFenceEpoch
    /\ reply' = "stale_epoch"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, acceptedPayload, tokenState,
                   historyLost, effectInvocations, physicalEffects,
                   staleEffectCount>>

ExecuteReservedToken ==
    /\ deviceUp
    /\ tokenState = "reserved"
    /\ effectInvocations = 0
    /\ effectInvocations' = effectInvocations + 1
    /\ physicalEffects' = physicalEffects + 1
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, acceptedPayload, tokenState,
                   historyLost, reply, staleEffectCount>>

PersistAppliedOutcome ==
    /\ deviceUp
    /\ tokenState = "reserved"
    /\ effectInvocations = 1
    /\ tokenState' = "applied"
    /\ reply' = "prior_applied"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, acceptedPayload, historyLost,
                   effectInvocations, physicalEffects, staleEffectCount>>

DeviceCrash ==
    /\ deviceUp
    /\ deviceBoot = 0
    /\ deviceUp' = FALSE
    /\ IF DurableEndpointNvm
          THEN
            /\ UNCHANGED <<endpointFenceEpoch, acceptedPayload,
                           tokenState, historyLost>>
          ELSE
            /\ endpointFenceEpoch' = -1
            /\ acceptedPayload' = "none"
            /\ tokenState' = "none"
            /\ historyLost' = TRUE
    /\ UNCHANGED <<deviceBoot, authorityEpoch, requestEpoch, requestPayload,
                   reply, effectInvocations, physicalEffects, staleEffectCount>>

DeviceRestart ==
    /\ ~deviceUp
    /\ deviceBoot = 0
    /\ deviceUp' = TRUE
    /\ deviceBoot' = deviceBoot + 1
    /\ IF DurableEndpointNvm /\ tokenState = "reserved"
          THEN tokenState' = "unknown"
          ELSE UNCHANGED tokenState
    /\ UNCHANGED <<authorityEpoch, endpointFenceEpoch, requestEpoch,
                   requestPayload, acceptedPayload, historyLost, reply,
                   effectInvocations, physicalEffects, staleEffectCount>>

UnsafeReplayAfterNvmLoss ==
    /\ AllowUnsafeReplay
    /\ deviceUp
    /\ historyLost
    /\ tokenState = "none"
    /\ physicalEffects > 0
    /\ effectInvocations < 2
    /\ effectInvocations' = effectInvocations + 1
    /\ physicalEffects' = physicalEffects + 1
    /\ tokenState' = "reserved"
    /\ acceptedPayload' = requestPayload
    /\ reply' = "reserved"
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, historyLost, staleEffectCount>>

UnsafeStaleEpochEffect ==
    /\ AllowStaleEpoch
    /\ deviceUp
    /\ requestEpoch < endpointFenceEpoch
    /\ staleEffectCount = 0
    /\ staleEffectCount' = staleEffectCount + 1
    /\ UNCHANGED <<deviceUp, deviceBoot, authorityEpoch, endpointFenceEpoch,
                   requestEpoch, requestPayload, acceptedPayload, tokenState,
                   historyLost, reply, effectInvocations, physicalEffects>>

TerminalIdle ==
    UNCHANGED vars

Next ==
    \/ AdvanceAuthorityEpoch
    \/ SetRequestToAuthorityEpoch
    \/ SetRequestToOldEpoch
    \/ SetAlternatePayload
    \/ FenceEndpointEpoch
    \/ SubmitNewToken
    \/ ReplaySameToken
    \/ RejectChangedPayload
    \/ RejectStaleEpoch
    \/ ExecuteReservedToken
    \/ PersistAppliedOutcome
    \/ DeviceCrash
    \/ DeviceRestart
    \/ UnsafeReplayAfterNvmLoss
    \/ UnsafeStaleEpochEffect
    \/ TerminalIdle

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ deviceUp \in BOOLEAN
    /\ deviceBoot \in 0..1
    /\ authorityEpoch \in 0..1
    /\ endpointFenceEpoch \in {-1, 0, 1}
    /\ requestEpoch \in 0..1
    /\ requestPayload \in {"payload-a", "payload-b"}
    /\ acceptedPayload \in Payloads
    /\ tokenState \in TokenStates
    /\ historyLost \in BOOLEAN
    /\ reply \in Replies
    /\ effectInvocations \in 0..2
    /\ physicalEffects \in 0..2
    /\ staleEffectCount \in 0..1

AtMostOnePhysicalEffectPerToken ==
    physicalEffects <= 1

AtMostOneEndpointInvocationPerToken ==
    effectInvocations <= 1

NoOldEpochEffectAfterFence ==
    staleEffectCount = 0

EffectRequiresAcceptedToken ==
    effectInvocations = 0 \/ acceptedPayload # "none" \/ historyLost

UnknownTokenIsNotReExecutable ==
    tokenState = "unknown" => effectInvocations <= 1

=============================================================================
