import Mathlib

/-!
# Proof-carrying max-min allocation: exact waterline contract

The optimizer is not trusted to justify its own answer. A certificate supplies
an exact-rational waterline. The small specification checker recomputes each
recipient allocation as min(cap, waterline) and checks aggregate budget closure.
This is an abstract exact-arithmetic contract; it does not establish conformance
of the IEEE-754 Rust allocator or prove that a waterline finder is complete.
-/

namespace SymtropyFormal

/-- Exact equal-share / max-min allocation at a proposed waterline.
The caller provides caps in canonical stable-recipient-ID order. -/
def allocateAtWaterline (caps : List ℚ) (waterline : ℚ) : List ℚ :=
  caps.map (fun cap => min cap waterline)

/-- A candidate waterline certificate for one finite source budget.
The list order is part of the input contract and must be bound to stable IDs by
the calling layer. -/
structure WaterlineCertificate where
  supply : ℚ
  caps : List ℚ
  waterline : ℚ
  deriving Repr

/-- A certificate is accepted only when supply and waterline are non-negative,
every recipient cap is non-negative, and the exact allocations produced by the
waterline close to min(supply, total capacity). -/
def WaterlineCertificate.accepted (c : WaterlineCertificate) : Prop :=
  0 ≤ c.supply ∧
  0 ≤ c.waterline ∧
  (∀ cap ∈ c.caps, 0 ≤ cap) ∧
  (allocateAtWaterline c.caps c.waterline).sum = min c.supply c.caps.sum

/-- Each capped recipient allocation is non-negative under valid cap/waterline
domains. -/
theorem waterline_allocation_nonnegative
    (cap waterline : ℚ)
    (hcap : 0 ≤ cap)
    (hwaterline : 0 ≤ waterline) :
    0 ≤ min cap waterline := by
  exact le_min hcap hwaterline

/-- No recipient receives above its declared cap. -/
theorem waterline_allocation_respects_cap
    (cap waterline : ℚ) :
    min cap waterline ≤ cap := by
  exact min_le_left cap waterline

/-- If a recipient's cap is below the waterline, it saturates exactly at cap. -/
theorem waterline_saturates_constrained_recipient
    (cap waterline : ℚ)
    (hcap : cap ≤ waterline) :
    min cap waterline = cap := by
  exact min_eq_left hcap

/-- All recipients whose cap is strictly above the waterline receive the same
waterline allocation. This is the equal-share portion of the max-min policy. -/
theorem waterline_equalizes_unsaturated_recipient
    (cap waterline : ℚ)
    (hunsaturated : min cap waterline < cap) :
    min cap waterline = waterline := by
  by_cases hle : cap ≤ waterline
  · have heq : min cap waterline = cap := min_eq_left hle
    rw [heq] at hunsaturated
    exact (lt_irrefl cap hunsaturated).elim
  · have hlt : waterline < cap := lt_of_not_ge hle
    exact min_eq_right (le_of_lt hlt)

/-- A permuted input cap list yields exactly the corresponding permutation of
recipient allocations. With IDs bound to entries, this is order-independence of
per-recipient results rather than dependence on traversal position. -/
theorem waterline_allocation_permutation_invariant
    (caps caps' : List ℚ)
    (waterline : ℚ)
    (hperm : caps.Perm caps') :
    (allocateAtWaterline caps waterline).Perm
      (allocateAtWaterline caps' waterline) := by
  unfold allocateAtWaterline
  exact hperm.map (fun cap => min cap waterline)

/-- An accepted certificate cannot allocate more than the available supply. -/
theorem accepted_waterline_respects_supply
    (c : WaterlineCertificate)
    (haccepted : c.accepted) :
    (allocateAtWaterline c.caps c.waterline).sum ≤ c.supply := by
  rcases haccepted with ⟨_, _, _, hbudget⟩
  rw [hbudget]
  exact min_le_left c.supply c.caps.sum

/-- An accepted certificate cannot allocate more than total eligible capacity. -/
theorem accepted_waterline_respects_total_capacity
    (c : WaterlineCertificate)
    (haccepted : c.accepted) :
    (allocateAtWaterline c.caps c.waterline).sum ≤ c.caps.sum := by
  rcases haccepted with ⟨_, _, _, hbudget⟩
  rw [hbudget]
  exact min_le_right c.supply c.caps.sum

/-- Every amount emitted by an accepted certificate is non-negative. -/
theorem accepted_waterline_allocations_nonnegative
    (c : WaterlineCertificate)
    (haccepted : c.accepted) :
    ∀ amount ∈ allocateAtWaterline c.caps c.waterline, 0 ≤ amount := by
  rcases haccepted with ⟨_, hwaterline, hcaps, _⟩
  intro amount hmem
  rcases List.mem_map.mp hmem with ⟨cap, hcap, hvalue⟩
  rw [← hvalue]
  exact le_min (hcaps cap hcap) hwaterline

/-- Positive control: a constrained recipient saturates, and remaining supply
is shared at the waterline by the two unconstrained recipients. -/
def sampleWaterlineCertificate : WaterlineCertificate where
  supply := 5
  caps := [1, 4, 10]
  waterline := 2

theorem sample_waterline_certificate_is_accepted :
    sampleWaterlineCertificate.accepted := by
  norm_num [WaterlineCertificate.accepted, sampleWaterlineCertificate,
    allocateAtWaterline]

/-- Negative control: over-allocating beyond the source budget fails closure. -/
def overBudgetWaterlineCertificate : WaterlineCertificate where
  supply := 5
  caps := [1, 4, 10]
  waterline := 3

theorem over_budget_waterline_certificate_is_rejected :
    ¬ overBudgetWaterlineCertificate.accepted := by
  norm_num [WaterlineCertificate.accepted, overBudgetWaterlineCertificate,
    allocateAtWaterline]

end SymtropyFormal
