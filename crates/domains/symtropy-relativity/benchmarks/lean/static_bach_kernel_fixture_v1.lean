import Mathlib

/-!
Exact rational arithmetic for the linearized Mannheim--Kazanas static Bach
solution-family constraint.

This file formalizes the finite-dimensional tangent constraint only. It does
not formalize the differential equation, its integration, the full Bach tensor,
or a radii-polynomial existence theorem.
-/

namespace SymtropyStaticBachKernel

def mkTangentConstraint (dci dc0 dc1 : ℚ) : Prop :=
  14 * dc0 - 3 * dci + 51 * dc1 = 0

theorem mk_inverse_basis_direction :
    mkTangentConstraint 1 (3 / 14) 0 := by
  norm_num [mkTangentConstraint]

theorem mk_linear_basis_direction :
    mkTangentConstraint 0 (-51 / 14) 1 := by
  norm_num [mkTangentConstraint]

theorem mk_quadratic_basis_direction :
    mkTangentConstraint 0 0 0 := by
  norm_num [mkTangentConstraint]

theorem tangent_constraint_iff_parameterization
    (dci dc0 dc1 : ℚ) :
    mkTangentConstraint dci dc0 dc1
      ↔ dc0 = 3 * dci / 14 - 51 * dc1 / 14 := by
  constructor
  · intro h
    unfold mkTangentConstraint at h
    linarith
  · intro h
    unfold mkTangentConstraint
    rw [h]
    ring

theorem every_mk_tangent_direction_has_three_free_coordinates
    (dci dc0 dc1 dc2 : ℚ)
    (h : mkTangentConstraint dci dc0 dc1) :
    ∃ x y z : ℚ,
      x = dci ∧
      dc0 = 3 * x / 14 - 51 * y / 14 ∧
      y = dc1 ∧ z = dc2 := by
  refine ⟨dci, dc1, dc2, rfl, ?_, rfl, rfl⟩
  exact (tangent_constraint_iff_parameterization dci dc0 dc1).1 h

#print axioms mk_inverse_basis_direction
#print axioms mk_linear_basis_direction
#print axioms mk_quadratic_basis_direction
#print axioms tangent_constraint_iff_parameterization
#print axioms every_mk_tangent_direction_has_three_free_coordinates

end SymtropyStaticBachKernel
