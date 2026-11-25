------------------------ MODULE RRVRTemporalClosure ------------------------
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    MaxAge,
    TransitBoundsFinite,
    RetentionBoundsFinite,
    OriginalAgePreserved,
    ReceiverExpiry,
    ClockValid,
    InventoryComplete,
    AllowQuietClosure,
    AllowLateEffect,
    AllowAgeRefresh,
    AllowHiddenEnqueue,
    AllowUnboundedClosure,
    AllowInvalidClockClosure,
    AllowIncompleteInventoryClosure

Messages == {"left", "right", "hidden"}
Stages == {"absent", "in_flight", "terminal", "expired", "effect", "cancelled"}
ProofPaths == {"none", "drained", "expiry"}

VARIABLES ingressFenced, clock, age, stage, proofPath, postClosureEffects

vars ==
    <<ingressFenced, clock, age, stage, proofPath, postClosureEffects>>

Init ==
    /\ ingressFenced = FALSE
    /\ clock = 0
    /\ age = [m \in Messages |-> 0]
    /\ stage =
        [m \in Messages |->
            IF m = "hidden" THEN "absent" ELSE "in_flight"]
    /\ proofPath = "none"
    /\ postClosureEffects = 0

FenceIngress ==
    /\ ~ingressFenced
    /\ ingressFenced' = TRUE
    /\ UNCHANGED <<clock, age, stage, proofPath, postClosureEffects>>

AdvanceTime ==
    /\ clock < MaxAge + 1
    /\ clock' = clock + 1
    /\ age' =
        [m \in Messages |->
            IF stage[m] \in {"absent", "expired", "effect", "cancelled"}
            THEN age[m]
            ELSE IF age[m] < MaxAge + 1 THEN age[m] + 1 ELSE age[m]]
    /\ UNCHANGED <<ingressFenced, stage, proofPath, postClosureEffects>>

DeliverToTerminal(m) ==
    /\ stage[m] = "in_flight"
    /\ (age[m] < MaxAge \/ AllowLateEffect \/ AllowAgeRefresh)
    /\ stage' = [stage EXCEPT ![m] = "terminal"]
    /\ age' =
        [age EXCEPT ![m] = IF AllowAgeRefresh THEN 0 ELSE age[m]]
    /\ UNCHANGED <<ingressFenced, clock, proofPath, postClosureEffects>>

RejectExpiredAtReceiver(m) ==
    /\ stage[m] \in {"in_flight", "terminal"}
    /\ age[m] >= MaxAge
    /\ ReceiverExpiry
    /\ ~AllowLateEffect
    /\ ~AllowAgeRefresh
    /\ stage' = [stage EXCEPT ![m] = "expired"]
    /\ UNCHANGED <<ingressFenced, clock, age, proofPath, postClosureEffects>>

Actuate(m) ==
    /\ stage[m] = "terminal"
    /\ (age[m] < MaxAge \/ AllowLateEffect)
    /\ stage' = [stage EXCEPT ![m] = "effect"]
    /\ postClosureEffects' =
        IF proofPath = "none" THEN postClosureEffects
        ELSE postClosureEffects + 1
    /\ UNCHANGED <<ingressFenced, clock, age, proofPath>>

CancelOldCommand(m) ==
    /\ ingressFenced
    /\ stage[m] \in {"in_flight", "terminal"}
    /\ stage' = [stage EXCEPT ![m] = "cancelled"]
    /\ UNCHANGED <<ingressFenced, clock, age, proofPath, postClosureEffects>>

EnqueueHiddenPath ==
    /\ AllowHiddenEnqueue
    /\ ingressFenced
    /\ stage["hidden"] = "absent"
    /\ stage' = [stage EXCEPT !["hidden"] = "in_flight"]
    /\ age' = [age EXCEPT !["hidden"] = 0]
    /\ UNCHANGED <<ingressFenced, clock, proofPath, postClosureEffects>>

CertifyFencedAndDrained ==
    /\ proofPath = "none"
    /\ ingressFenced
    /\ (InventoryComplete \/ AllowIncompleteInventoryClosure)
    /\ \A m \in Messages :
        stage[m] \in {"absent", "expired", "effect", "cancelled"}
    /\ proofPath' = "drained"
    /\ UNCHANGED <<ingressFenced, clock, age, stage, postClosureEffects>>

CertifyBoundedExpiry ==
    /\ proofPath = "none"
    /\ ingressFenced
    /\ clock >= MaxAge
    /\ (TransitBoundsFinite \/ AllowUnboundedClosure)
    /\ (RetentionBoundsFinite \/ AllowUnboundedClosure)
    /\ (OriginalAgePreserved \/ AllowAgeRefresh)
    /\ (ReceiverExpiry \/ AllowLateEffect)
    /\ (ClockValid \/ AllowInvalidClockClosure)
    /\ (InventoryComplete \/ AllowIncompleteInventoryClosure)
    /\ proofPath' = "expiry"
    /\ UNCHANGED <<ingressFenced, clock, age, stage, postClosureEffects>>

UnsafeCertifyQuietInterval ==
    /\ AllowQuietClosure
    /\ proofPath = "none"
    /\ ingressFenced
    /\ clock >= 1
    /\ proofPath' = "expiry"
    /\ UNCHANGED <<ingressFenced, clock, age, stage, postClosureEffects>>

Next ==
    \/ FenceIngress
    \/ AdvanceTime
    \/ \E m \in Messages : DeliverToTerminal(m)
    \/ \E m \in Messages : RejectExpiredAtReceiver(m)
    \/ \E m \in Messages : Actuate(m)
    \/ \E m \in Messages : CancelOldCommand(m)
    \/ EnqueueHiddenPath
    \/ CertifyFencedAndDrained
    \/ CertifyBoundedExpiry
    \/ UnsafeCertifyQuietInterval
    \/ UNCHANGED vars

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ ingressFenced \in BOOLEAN
    /\ clock \in 0..(MaxAge + 1)
    /\ age \in [Messages -> 0..(MaxAge + 1)]
    /\ stage \in [Messages -> Stages]
    /\ proofPath \in ProofPaths
    /\ postClosureEffects \in 0..Cardinality(Messages)

CertificateSound ==
    /\ (proofPath = "none"
        \/ (proofPath = "drained"
            /\ ingressFenced
            /\ InventoryComplete
            /\ \A m \in Messages :
                stage[m] \in {"absent", "expired", "effect", "cancelled"})
        \/ (proofPath = "expiry"
            /\ ingressFenced
            /\ clock >= MaxAge
            /\ TransitBoundsFinite
            /\ RetentionBoundsFinite
            /\ OriginalAgePreserved
            /\ ReceiverExpiry
            /\ ClockValid
            /\ InventoryComplete))

NoPhysicalEffectAfterClosure ==
    postClosureEffects = 0

=============================================================================
