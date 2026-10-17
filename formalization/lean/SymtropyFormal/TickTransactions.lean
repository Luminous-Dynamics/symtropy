import Mathlib

/-!
# Tick transaction lifecycle contract

This file formalizes a small *abstract* lifecycle model for deterministic
simulation ticks. It is intended to complement the Rust transaction primitive,
not to imply that the Rust implementation refines this model.

Digests are represented by Nat values so the transition logic stays independent
of a particular hash function. The model proves lifecycle and chain-linkage
properties only; it does not prove digest authenticity, physical correctness,
rollback, or floating-point determinism.
-/

namespace SymtropyFormal

inductive TickPhase where
  | begun
  | physicsApplied
  | finalized
  deriving DecidableEq, Repr

structure TickRecord where
  id : Nat
  phase : TickPhase
  inputDigest : Nat
  priorStateDigest : Nat
  resultStateDigest : Option Nat
  deriving DecidableEq, Repr

structure TickLedger where
  active : Option TickRecord
  lastFinalized : Option (Nat × Nat)
  deriving DecidableEq, Repr

def initialTickLedger : TickLedger := ⟨none, none⟩

/-- An absent active tick is ready; a present one must be finalized and agree
with the committed (tick ID, result digest) pair. This rejects malformed
states instead of silently treating them as idle. -/
def activeTickIsClosed (s : TickLedger) : Prop :=
  match s.active, s.lastFinalized with
  | none, _ => True
  | some tick, some (lastId, resultDigest) =>
      tick.phase = .finalized ∧
        tick.id = lastId ∧
        tick.resultStateDigest = some resultDigest
  | some _, none => False

/-- Runtime transition guard for an in-progress tick. Recheck the receipt-chain
link and monotonic ID at each mutation boundary, so malformed externally
constructed model states fail closed instead of relying only on beginTick. -/
def activeTickCanAdvance (s : TickLedger) (tick : TickRecord) : Prop :=
  tick.resultStateDigest = none ∧
    (match s.lastFinalized with
      | none => True
      | some (lastId, _) => lastId < tick.id) ∧
    (match s.lastFinalized with
      | none => True
      | some (_, resultDigest) => resultDigest = tick.priorStateDigest)

/-- Gate for opening a new tick: prior tick closed, ID strictly increasing,
and prior-state digest linked to the last committed result. -/
def beginAllowed (s : TickLedger) (id priorDigest : Nat) : Prop :=
  activeTickIsClosed s ∧
    (match s.lastFinalized with
      | none => True
      | some (lastId, _) => lastId < id) ∧
    (match s.lastFinalized with
      | none => True
      | some (_, resultDigest) => resultDigest = priorDigest)

def beginTick (s : TickLedger) (id inputDigest priorDigest : Nat) :
    Option TickLedger :=
  if beginAllowed s id priorDigest then
    some { s with
      active := some {
        id := id
        phase := .begun
        inputDigest := inputDigest
        priorStateDigest := priorDigest
        resultStateDigest := none
      }
    }
  else
    none

/-- Physics can be marked applied only for the currently active begun tick. -/
def applyPhysics (s : TickLedger) (id : Nat) : Option TickLedger :=
  match s.active with
  | none => none
  | some tick =>
      if tick.id = id ∧ tick.phase = .begun ∧ activeTickCanAdvance s tick then
        some { s with active := some { tick with phase := .physicsApplied } }
      else
        none

/-- Finalize once after physics. A same-result retry is idempotent; a different
result is rejected. Updating lastFinalized and the active receipt happens in
one abstract state transition. -/
def finalizeTick (s : TickLedger) (id resultDigest : Nat) :
    Option TickLedger :=
  match s.active with
  | none => none
  | some tick =>
      if tick.id = id then
        match tick.phase, tick.resultStateDigest with
        | .physicsApplied, none =>
            if activeTickCanAdvance s tick then
              some {
                s with
                active := some {
                  tick with
                  phase := .finalized
                  resultStateDigest := some resultDigest
                }
                lastFinalized := some (tick.id, resultDigest)
              }
            else
              none
        | .finalized, some previousResult =>
            if previousResult = resultDigest ∧
              s.lastFinalized = some (tick.id, previousResult) then some s else none
        | _, _ => none
      else
        none

/-! ## Negative controls and invariant lemmas -/

theorem begin_rejects_nonmonotonic_tick
    (s : TickLedger) (id inputDigest priorDigest lastId previousDigest : Nat)
    (hLast : s.lastFinalized = some (lastId, previousDigest))
    (hStale : id ≤ lastId) :
    beginTick s id inputDigest priorDigest = none := by
  have hNotFresh : ¬ lastId < id := Nat.not_lt_of_ge hStale
  simp [beginTick, beginAllowed, hLast, hNotFresh]

theorem begin_rejects_prior_state_mismatch
    (s : TickLedger) (id inputDigest priorDigest lastId previousDigest : Nat)
    (hLast : s.lastFinalized = some (lastId, previousDigest))
    (hMismatch : priorDigest ≠ previousDigest) :
    beginTick s id inputDigest priorDigest = none := by
  have hWrongLink : previousDigest ≠ priorDigest :=
    fun h => hMismatch h.symm
  simp [beginTick, beginAllowed, hLast, hWrongLink]

theorem applyPhysics_rejects_non_begun_phase
    (s : TickLedger) (tick : TickRecord)
    (hActive : s.active = some tick)
    (hPhase : tick.phase ≠ .begun) :
    applyPhysics s tick.id = none := by
  simp [applyPhysics, hActive, hPhase]

theorem finalize_rejects_before_physics
    (s : TickLedger) (tick : TickRecord) (resultDigest : Nat)
    (hActive : s.active = some tick)
    (hPhase : tick.phase = .begun)
    (hNoResult : tick.resultStateDigest = none) :
    finalizeTick s tick.id resultDigest = none := by
  simp [finalizeTick, hActive, hPhase, hNoResult]

theorem finalize_same_result_is_idempotent
    (s : TickLedger) (tick : TickRecord) (id resultDigest : Nat)
    (hActive : s.active = some tick)
    (hId : tick.id = id)
    (hPhase : tick.phase = .finalized)
    (hResult : tick.resultStateDigest = some resultDigest)
    (hLast : s.lastFinalized = some (tick.id, resultDigest)) :
    finalizeTick s id resultDigest = some s := by
  simp [finalizeTick, hActive, hId, hPhase, hResult, hLast]

theorem finalize_conflicting_result_is_rejected
    (s : TickLedger) (tick : TickRecord) (id previousResult newResult : Nat)
    (hActive : s.active = some tick)
    (hId : tick.id = id)
    (hPhase : tick.phase = .finalized)
    (hPrevious : tick.resultStateDigest = some previousResult)
    (hConflict : previousResult ≠ newResult) :
    finalizeTick s id newResult = none := by
  simp [finalizeTick, hActive, hId, hPhase, hPrevious, hConflict]

theorem applyPhysics_rejects_prepopulated_result
    (s : TickLedger) (tick : TickRecord) (previousResult : Nat)
    (hActive : s.active = some tick)
    (hPhase : tick.phase = .begun)
    (hResult : tick.resultStateDigest = some previousResult) :
    applyPhysics s tick.id = none := by
  simp [applyPhysics, hActive, hPhase, activeTickCanAdvance, hResult]

theorem finalize_rejects_unlinked_tick
    (s : TickLedger) (tick : TickRecord) (resultDigest lastId previousResult : Nat)
    (hActive : s.active = some tick)
    (hPhase : tick.phase = .physicsApplied)
    (hNoResult : tick.resultStateDigest = none)
    (hLast : s.lastFinalized = some (lastId, previousResult))
    (hMismatch : tick.priorStateDigest ≠ previousResult) :
    finalizeTick s tick.id resultDigest = none := by
  have hWrongLink : previousResult ≠ tick.priorStateDigest :=
    fun h => hMismatch h.symm
  simp [finalizeTick, hActive, hPhase, hNoResult, activeTickCanAdvance,
    hLast, hWrongLink]

end SymtropyFormal
