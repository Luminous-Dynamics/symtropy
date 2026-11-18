import Mathlib
import RadiiPolynomial.Certification.RadiiPolynomialQ

/-!
Exact-rational theorem-arithmetic fixture using the pinned RadiiPolynomial
framework. The rational bounds below are synthetic plumbing values, NOT bounds
computed from a black-hole candidate, BVP residual or verified inverse.
-/

namespace SymtropyRadiiPolynomialFixture

open RadiiPolynomial

def Y0 : ℚ := 1 / 1000
def Z0 : ℚ := 1 / 100
def Z1 : ℚ := 1 / 100
def Z2 : ℚ := 1 / 10
def radius : ℚ := 1 / 100

theorem rational_radii_polynomial_negative :
    Z2 * radius ^ 2 - (1 - Z0 - Z1) * radius + Y0 < 0 := by
  norm_num [Y0, Z0, Z1, Z2, radius]

theorem rational_contraction_bound_less_than_one :
    Z0 + Z1 + Z2 * radius < 1 := by
  norm_num [Y0, Z0, Z1, Z2, radius]

theorem framework_real_radii_polynomial_negative :
    generalRadiiPolynomial (Y0 : ℝ) (Z0 : ℝ) (Z1 : ℝ)
      (fun _ => (Z2 : ℝ)) (radius : ℝ) < 0 :=
  generalRadiiPolynomial_neg_of_Q_const Y0 Z0 Z1 Z2 radius
    rational_radii_polynomial_negative

#print axioms rational_radii_polynomial_negative
#print axioms framework_real_radii_polynomial_negative

end SymtropyRadiiPolynomialFixture
