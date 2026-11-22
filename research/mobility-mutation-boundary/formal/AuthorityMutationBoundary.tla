--------------------------- MODULE AuthorityMutationBoundary ---------------------------
EXTENDS Naturals, FiniteSets, Sequences

CONSTANTS
    Tickets,
    Bindings,
    Attempts,
    ClockDomains,
    BootEpochs,
    RecoveryPermits,
    NoPending,
    NoPermit,
    MaxTick,
    MaxEpoch,
    InitialBinding,
    InitialAttempt,
    InitialEpoch,
    InitialClock,
    InitialBoot,
    TicketBinding,
    TicketAttempt,
    TicketEpoch,
    TicketClock,
    TicketBoot,
    TicketIssued,
    TicketExpires,
    PermitAttempt,
    PermitEpoch,
    PermitPlanBinding

ASSUME
    /\ Tickets # {}
    /\ Bindings # {}
    /\ Attempts # {}
    /\ ClockDomains # {}
    /\ BootEpochs # {}
    /\ RecoveryPermits # {}
    /\ NoPending \notin Attempts
    /\ NoPermit \notin RecoveryPermits
    /\ MaxTick \in Nat
    /\ MaxEpoch \in Nat
    /\ DOMAIN TicketBinding = Tickets
    /\ DOMAIN TicketAttempt = Tickets
    /\ DOMAIN TicketEpoch = Tickets
    /\ DOMAIN TicketClock = Tickets
    /\ DOMAIN TicketBoot = Tickets
    /\ DOMAIN TicketIssued = Tickets
    /\ DOMAIN TicketExpires = Tickets
    /\ DOMAIN PermitAttempt = RecoveryPermits
    /\ DOMAIN PermitEpoch = RecoveryPermits
    /\ DOMAIN PermitPlanBinding = RecoveryPermits
    /\ RANGE PermitPlanBinding \subseteq Bindings
    /\ RANGE PermitAttempt \subseteq Attempts
    /\ RANGE PermitEpoch \subseteq 0..MaxEpoch
    /\ RANGE TicketBinding \subseteq Bindings
    /\ RANGE TicketAttempt \subseteq Attempts
    /\ RANGE TicketEpoch \subseteq 0..MaxEpoch
    /\ RANGE TicketClock \subseteq ClockDomains
    /\ RANGE TicketBoot \subseteq BootEpochs
    /\ RANGE TicketIssued \subseteq 0..MaxTick
    /\ RANGE TicketExpires \subseteq 0..MaxTick
    /\ \A t \in Tickets : TicketIssued[t] < TicketExpires[t]

VARIABLES
    authorityBinding,
    activeAttempt,
    authorityEpoch,
    clockDomain,
    bootEpoch,
    nowTicks,
    pendingAttempt,
    consumed,
    consumedPermits,
    applicationSeq,
    applicationLog,
    resolutionLog

vars ==
    << authorityBinding, activeAttempt, authorityEpoch, clockDomain,
       bootEpoch, nowTicks, pendingAttempt, consumed, consumedPermits,
       applicationSeq, applicationLog, resolutionLog >>

ApplicationRecord ==
    [ ticket : Tickets,
      binding : Bindings,
      attempt : Attempts,
      epoch : 0..MaxEpoch,
      clock : ClockDomains,
      boot : BootEpochs,
      tick : 0..MaxTick,
      seq : Nat ]

ResolutionRecord ==
    [ attempt : Attempts,
      kind : {"Satisfied", "NoEffect", "PartialEffect"},
      permit : RecoveryPermits \cup {NoPermit},
      authorityEpoch : 0..MaxEpoch,
      nextAttempt : Attempts,
      nextBinding : Bindings,
      nextEpoch : 0..MaxEpoch ]

Init ==
    /\ authorityBinding = InitialBinding
    /\ activeAttempt = InitialAttempt
    /\ authorityEpoch = InitialEpoch
    /\ clockDomain = InitialClock
    /\ bootEpoch = InitialBoot
    /\ nowTicks = 0
    /\ pendingAttempt = NoPending
    /\ consumed = {}
    /\ consumedPermits = {}
    /\ applicationSeq = 1
    /\ applicationLog = << >>
    /\ resolutionLog = << >>

Apply(t) ==
    /\ t \in Tickets
    /\ pendingAttempt = NoPending
    /\ t \notin consumed
    /\ TicketBinding[t] = authorityBinding
    /\ TicketAttempt[t] = activeAttempt
    /\ TicketEpoch[t] = authorityEpoch
    /\ TicketClock[t] = clockDomain
    /\ TicketBoot[t] = bootEpoch
    /\ TicketIssued[t] <= nowTicks
    /\ nowTicks < TicketExpires[t]
    /\ pendingAttempt' = TicketAttempt[t]
    /\ consumed' = consumed \cup {t}
    /\ applicationLog' =
          Append(applicationLog,
            [ ticket |-> t,
              binding |-> authorityBinding,
              attempt |-> activeAttempt,
              epoch |-> authorityEpoch,
              clock |-> clockDomain,
              boot |-> bootEpoch,
              tick |-> nowTicks,
              seq |-> applicationSeq ])
    /\ applicationSeq' = applicationSeq + 1
    /\ UNCHANGED
          << authorityBinding, activeAttempt, authorityEpoch, clockDomain,
             bootEpoch, nowTicks, resolutionLog, consumedPermits >>

ReplaceAuthority(nextBinding, nextAttempt, nextClock, nextBoot, nextEpoch) ==
    /\ nextBinding \in Bindings
    /\ nextAttempt \in Attempts
    /\ nextClock \in ClockDomains
    /\ nextBoot \in BootEpochs
    /\ nextEpoch \in 0..MaxEpoch
    /\ nextEpoch > authorityEpoch
    /\ authorityBinding' = nextBinding
    /\ activeAttempt' = nextAttempt
    /\ clockDomain' = nextClock
    /\ bootEpoch' = nextBoot
    /\ authorityEpoch' = nextEpoch
    /\ UNCHANGED
          << nowTicks, pendingAttempt, consumed, consumedPermits, applicationSeq,
             applicationLog, resolutionLog >>

Tick ==
    /\ nowTicks < MaxTick
    /\ nowTicks' = nowTicks + 1
    /\ UNCHANGED
          << authorityBinding, activeAttempt, authorityEpoch, clockDomain,
             bootEpoch, pendingAttempt, consumed, consumedPermits, applicationSeq,
             applicationLog, resolutionLog >>

ResolveSatisfied(nextBinding, nextAttempt, nextEpoch) ==
    /\ pendingAttempt # NoPending
    /\ nextBinding \in Bindings
    /\ nextAttempt \in Attempts
    /\ nextAttempt # pendingAttempt
    /\ nextEpoch \in 0..MaxEpoch
    /\ nextEpoch > authorityEpoch
    /\ nextBinding # authorityBinding
    /\ authorityBinding' = nextBinding
    /\ activeAttempt' = nextAttempt
    /\ authorityEpoch' = nextEpoch
    /\ pendingAttempt' = NoPending
    /\ resolutionLog' =
          Append(resolutionLog,
            [ attempt |-> pendingAttempt,
              kind |-> "Satisfied",
              permit |-> NoPermit,
              authorityEpoch |-> authorityEpoch,
              nextAttempt |-> nextAttempt,
              nextBinding |-> nextBinding,
              nextEpoch |-> nextEpoch ])
    /\ UNCHANGED
          << clockDomain, bootEpoch, nowTicks, consumed, consumedPermits,
             applicationSeq, applicationLog >>

ResolveNoEffect(nextBinding, nextAttempt, nextEpoch, permit) ==
    /\ pendingAttempt # NoPending
    /\ nextBinding \in Bindings
    /\ nextAttempt \in Attempts
    /\ nextAttempt # pendingAttempt
    /\ nextEpoch \in 0..MaxEpoch
    /\ nextEpoch > authorityEpoch
    /\ nextBinding # authorityBinding
    /\ permit \in RecoveryPermits
    /\ permit \notin consumedPermits
    /\ PermitAttempt[permit] = pendingAttempt
    /\ PermitEpoch[permit] = authorityEpoch
    /\ PermitPlanBinding[permit] = nextBinding
    /\ authorityBinding' = nextBinding
    /\ activeAttempt' = nextAttempt
    /\ authorityEpoch' = nextEpoch
    /\ pendingAttempt' = NoPending
    /\ consumedPermits' = consumedPermits \cup {permit}
    /\ resolutionLog' =
          Append(resolutionLog,
            [ attempt |-> pendingAttempt,
              kind |-> "NoEffect",
              permit |-> permit,
              authorityEpoch |-> authorityEpoch,
              nextAttempt |-> nextAttempt,
              nextBinding |-> nextBinding,
              nextEpoch |-> nextEpoch ])
    /\ UNCHANGED
          << clockDomain, bootEpoch, nowTicks, consumed,
             applicationSeq, applicationLog >>

ResolvePartial(nextBinding, nextAttempt, nextEpoch, permit) ==
    /\ pendingAttempt # NoPending
    /\ nextBinding \in Bindings
    /\ nextAttempt \in Attempts
    /\ nextAttempt # pendingAttempt
    /\ nextEpoch \in 0..MaxEpoch
    /\ nextEpoch > authorityEpoch
    /\ nextBinding # authorityBinding
    /\ permit \in RecoveryPermits
    /\ permit \notin consumedPermits
    /\ PermitAttempt[permit] = pendingAttempt
    /\ PermitEpoch[permit] = authorityEpoch
    /\ PermitPlanBinding[permit] = nextBinding
    /\ authorityBinding' = nextBinding
    /\ activeAttempt' = nextAttempt
    /\ authorityEpoch' = nextEpoch
    /\ pendingAttempt' = NoPending
    /\ consumedPermits' = consumedPermits \cup {permit}
    /\ resolutionLog' =
          Append(resolutionLog,
            [ attempt |-> pendingAttempt,
              kind |-> "PartialEffect",
              permit |-> permit,
              authorityEpoch |-> authorityEpoch,
              nextAttempt |-> nextAttempt,
              nextBinding |-> nextBinding,
              nextEpoch |-> nextEpoch ])
    /\ UNCHANGED
          << clockDomain, bootEpoch, nowTicks, consumed,
             applicationSeq, applicationLog >>

Next ==
    \/ \E t \in Tickets : Apply(t)
    \/ \E b \in Bindings, a \in Attempts, c \in ClockDomains,
          boot \in BootEpochs, e \in 0..MaxEpoch :
          ReplaceAuthority(b, a, c, boot, e)
    \/ Tick
    \/ \E b \in Bindings, a \in Attempts, e \in 0..MaxEpoch :
          ResolveSatisfied(b, a, e)
    \/ \E b \in Bindings, a \in Attempts, e \in 0..MaxEpoch,
          p \in RecoveryPermits :
          ResolveNoEffect(b, a, e, p)
    \/ \E b \in Bindings, a \in Attempts, e \in 0..MaxEpoch,
          p \in RecoveryPermits :
          ResolvePartial(b, a, e, p)

Spec == Init /\ [][Next]_vars

TypeOK ==
    /\ authorityBinding \in Bindings
    /\ activeAttempt \in Attempts
    /\ authorityEpoch \in 0..MaxEpoch
    /\ clockDomain \in ClockDomains
    /\ bootEpoch \in BootEpochs
    /\ nowTicks \in 0..MaxTick
    /\ pendingAttempt \in Attempts \cup {NoPending}
    /\ consumed \subseteq Tickets
    /\ consumedPermits \subseteq RecoveryPermits
    /\ applicationSeq \in Nat
    /\ applicationLog \in Seq(ApplicationRecord)
    /\ resolutionLog \in Seq(ResolutionRecord)

NoTicketReuse ==
    Len(applicationLog) =
      Cardinality({applicationLog[i].ticket : i \in 1..Len(applicationLog)})

ConsumedMatchesApplicationLog ==
    consumed =
      {applicationLog[i].ticket : i \in 1..Len(applicationLog)}

NoUnauthorizedApplication ==
    \A i \in 1..Len(applicationLog) :
      LET record == applicationLog[i]
      IN
        /\ TicketBinding[record.ticket] = record.binding
        /\ TicketAttempt[record.ticket] = record.attempt
        /\ TicketEpoch[record.ticket] = record.epoch
        /\ TicketClock[record.ticket] = record.clock
        /\ TicketBoot[record.ticket] = record.boot
        /\ TicketIssued[record.ticket] <= record.tick
        /\ record.tick < TicketExpires[record.ticket]

PendingSlotConsistent ==
    IF pendingAttempt = NoPending
    THEN Len(applicationLog) = Len(resolutionLog)
    ELSE
      /\ Len(applicationLog) = Len(resolutionLog) + 1
      /\ pendingAttempt = applicationLog[Len(applicationLog)].attempt

ApplicationSequenceConsistent ==
    /\ applicationSeq = Len(applicationLog) + 1
    /\ \A i \in 1..Len(applicationLog) :
         applicationLog[i].seq = i

ConsumedPermitsMatchResolutionLog ==
    consumedPermits =
      {resolutionLog[i].permit : i \in 1..Len(resolutionLog)} \ {NoPermit}

RecoveryPermitLineage ==
    \A i \in 1..Len(resolutionLog) :
      LET record == resolutionLog[i]
      IN
        IF record.kind = "Satisfied"
        THEN record.permit = NoPermit
        ELSE
          /\ record.permit \in RecoveryPermits
          /\ PermitAttempt[record.permit] = record.attempt
          /\ PermitEpoch[record.permit] = record.authorityEpoch
          /\ PermitPlanBinding[record.permit] = record.nextBinding

=============================================================================
