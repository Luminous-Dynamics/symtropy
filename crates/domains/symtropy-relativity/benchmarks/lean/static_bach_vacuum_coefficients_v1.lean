import Mathlib

/-!
Exact rational/algebraic controls for the static Bach-vacuum classification.
This file does not formalize the differential integration or the generic
full-tensor-to-projection equivalence; those are tracked in issue #6048.
-/

namespace SymtropyStaticBachClassification

def vacuumConstraint (cMinus c0 c1 : ℝ) : ℝ :=
  c0 ^ 2 - 3 * c1 * cMinus - 1

def reducedMinus (q r : ℝ) : ℝ :=
  4 * q / (3 * r ^ 4)

def mkLapse (r : ℝ) : ℝ :=
  -17 / (10 * r) + 7 / 10 + r / 10 - r ^ 2 / 100

theorem schwarzschild_constraint :
    vacuumConstraint (-2) 1 0 = 0 := by
  norm_num [vacuumConstraint]

theorem mannheim_kazanas_constraint :
    vacuumConstraint (-17 / 10) (7 / 10) (1 / 10) = 0 := by
  norm_num [vacuumConstraint]

theorem off_shell_constraint :
    vacuumConstraint (-17 / 10) (11 / 10) (1 / 10) = 18 / 25 := by
  norm_num [vacuumConstraint]

theorem off_shell_reduced_residual_r3 :
    reducedMinus (18 / 25) 3 = 8 / 675 := by
  norm_num [reducedMinus]

theorem off_shell_reduced_residual_r6 :
    reducedMinus (18 / 25) 6 = 1 / 1350 := by
  norm_num [reducedMinus]

theorem off_shell_residual_ratio :
    reducedMinus (18 / 25) 3 / reducedMinus (18 / 25) 6 = 16 := by
  norm_num [reducedMinus]

/-- The conventional Mannheim--Kazanas parameterization satisfies the
coefficient constraint identically. The coefficient-level family remains
the primary statement because it also covers degenerate parameter branches. -/
theorem mannheim_kazanas_parameterization_vacuum (beta gamma : ℝ) :
    vacuumConstraint
      (-beta * (2 - 3 * beta * gamma))
      (1 - 3 * beta * gamma)
      gamma = 0 := by
  unfold vacuumConstraint
  ring

theorem mk_lapse_at_3 : mkLapse 3 = 103 / 300 := by
  norm_num [mkLapse]

theorem mk_lapse_at_6 : mkLapse 6 = 197 / 300 := by
  norm_num [mkLapse]

/-- The MK lapse is strictly positive throughout the closed annulus [3,6].
The identity below rewrites 100*r*B(r) as a manifestly positive polynomial. -/
theorem mk_lapse_positive_on_annulus {r : ℝ}
    (hLower : 3 ≤ r) (hUpper : r ≤ 6) :
    0 < mkLapse r := by
  have hrPositive : 0 < r := by linarith
  have hSquare : r ^ 2 ≤ 36 := by
    have hProduct :
        0 ≤ (6 - r) * (6 + r) :=
      mul_nonneg (sub_nonneg.mpr hUpper) (by linarith)
    nlinarith
  have hLinear : 21 ≤ 7 * r := by nlinarith
  have hFactor : 0 < -r ^ 2 + 7 * r + 91 := by
    nlinarith [hSquare, hLinear]
  have hRemainder : 0 ≤ r - 3 := sub_nonneg.mpr hLower
  have hPolynomial :
      0 < 103 + (r - 3) * (-r ^ 2 + 7 * r + 91) :=
    add_pos_of_pos_of_nonneg (by norm_num)
      (mul_nonneg hRemainder hFactor.le)
  have hIdentity :
      100 * r * mkLapse r =
        103 + (r - 3) * (-r ^ 2 + 7 * r + 91) := by
    unfold mkLapse
    field_simp [ne_of_gt hrPositive]
    ring
  have hScale : 0 ≤ 100 * r := by positivity
  by_contra hNotPositive
  have hNonpositive : mkLapse r ≤ 0 := le_of_not_gt hNotPositive
  have hProductNonpositive :
      100 * r * mkLapse r ≤ 0 :=
    mul_nonpos_of_nonneg_of_nonpos hScale hNonpositive
  linarith

#print axioms schwarzschild_constraint
#print axioms mannheim_kazanas_constraint
#print axioms off_shell_constraint
#print axioms off_shell_residual_ratio
#print axioms mannheim_kazanas_parameterization_vacuum
#print axioms mk_lapse_positive_on_annulus

end SymtropyStaticBachClassification
