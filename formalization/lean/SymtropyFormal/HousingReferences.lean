import Mathlib

/-!
# Symtropy housing analytical references

These definitions encode elementary closed-form reference equations under their
stated modelling assumptions. They are a mathematical specification, not a
claim that a Rust implementation or general numerical solver has been verified.

Conventions:
* real-valued inputs stand for consistently unit-normalized quantities;
* beam deflections use elementary Euler--Bernoulli, linear-static assumptions;
* positive stiffness/geometry assumptions are stated on theorems, not hidden in
  the definitions;
* numerical floating-point error and physical validation are separate gates.
-/

namespace SymtropyFormal

/-- Second moment of area for a rectangular section about its centroidal axis. -/
def rectangularSecondMoment (breadth depth : ℝ) : ℝ :=
  breadth * depth ^ 3 / 12

/-- Axial extension of a uniform, linearly elastic bar: F L / (E A). -/
def axialExtension (force length youngsModulus area : ℝ) : ℝ :=
  force * length / (youngsModulus * area)

/-- Free-end deflection of a cantilever under a tip point load: F L^3 / (3 E I). -/
def cantileverTipDeflection (force length youngsModulus inertia : ℝ) : ℝ :=
  force * length ^ 3 / (3 * youngsModulus * inertia)

/-- Midspan deflection of a simply supported beam under a midpoint point load. -/
def simplySupportedMidpointDeflection
    (force length youngsModulus inertia : ℝ) : ℝ :=
  force * length ^ 3 / (48 * youngsModulus * inertia)

/-- Midspan deflection for a simply supported beam under uniform load intensity. -/
def simplySupportedUniformLoadDeflection
    (loadIntensity length youngsModulus inertia : ℝ) : ℝ :=
  5 * loadIntensity * length ^ 4 / (384 * youngsModulus * inertia)

/-! ## Geometry contracts -/

theorem rectangularSecondMoment_pos
    {breadth depth : ℝ}
    (hBreadth : 0 < breadth) (hDepth : 0 < depth) :
    0 < rectangularSecondMoment breadth depth := by
  unfold rectangularSecondMoment
  exact div_pos (mul_pos hBreadth (pow_pos hDepth 3)) (by norm_num)

theorem rectangularSecondMoment_depth_scaling
    (breadth depth scale : ℝ) :
    rectangularSecondMoment breadth (scale * depth) =
      scale ^ 3 * rectangularSecondMoment breadth depth := by
  unfold rectangularSecondMoment
  field_simp
  <;> ring

/-! ## Axial bar contracts -/

theorem axialExtension_pos
    {force length youngsModulus area : ℝ}
    (hForce : 0 < force) (hLength : 0 < length)
    (hModulus : 0 < youngsModulus) (hArea : 0 < area) :
    0 < axialExtension force length youngsModulus area := by
  unfold axialExtension
  exact div_pos (mul_pos hForce hLength) (mul_pos hModulus hArea)

theorem axialExtension_force_scaling
    (force length youngsModulus area scale : ℝ)
    (hModulus : 0 < youngsModulus) (hArea : 0 < area) :
    axialExtension (scale * force) length youngsModulus area =
      scale * axialExtension force length youngsModulus area := by
  unfold axialExtension
  have hden : youngsModulus * area ≠ 0 :=
    ne_of_gt (mul_pos hModulus hArea)
  field_simp [hden]
  <;> ring

theorem axialExtension_length_scaling
    (force length youngsModulus area scale : ℝ)
    (hModulus : 0 < youngsModulus) (hArea : 0 < area) :
    axialExtension force (scale * length) youngsModulus area =
      scale * axialExtension force length youngsModulus area := by
  unfold axialExtension
  have hden : youngsModulus * area ≠ 0 :=
    ne_of_gt (mul_pos hModulus hArea)
  field_simp [hden]
  <;> ring

/-! ## Cantilever contracts -/

theorem cantileverTipDeflection_pos
    {force length youngsModulus inertia : ℝ}
    (hForce : 0 < force) (hLength : 0 < length)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    0 < cantileverTipDeflection force length youngsModulus inertia := by
  unfold cantileverTipDeflection
  apply div_pos
  · exact mul_pos hForce (pow_pos hLength 3)
  · exact mul_pos (mul_pos (by norm_num : (0 : ℝ) < 3) hModulus) hInertia

theorem cantileverTipDeflection_force_scaling
    (force length youngsModulus inertia scale : ℝ)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    cantileverTipDeflection (scale * force) length youngsModulus inertia =
      scale * cantileverTipDeflection force length youngsModulus inertia := by
  unfold cantileverTipDeflection
  have hden : 3 * youngsModulus * inertia ≠ 0 :=
    ne_of_gt (mul_pos (mul_pos (by norm_num : (0 : ℝ) < 3) hModulus) hInertia)
  field_simp [hden]
  <;> ring

theorem cantileverTipDeflection_length_cubic_scaling
    (force length youngsModulus inertia scale : ℝ)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    cantileverTipDeflection force (scale * length) youngsModulus inertia =
      scale ^ 3 * cantileverTipDeflection force length youngsModulus inertia := by
  unfold cantileverTipDeflection
  have hden : 3 * youngsModulus * inertia ≠ 0 :=
    ne_of_gt (mul_pos (mul_pos (by norm_num : (0 : ℝ) < 3) hModulus) hInertia)
  field_simp [hden]
  <;> ring

/-! ## Simply supported beam contracts -/

theorem simplySupportedMidpointDeflection_pos
    {force length youngsModulus inertia : ℝ}
    (hForce : 0 < force) (hLength : 0 < length)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    0 < simplySupportedMidpointDeflection force length youngsModulus inertia := by
  unfold simplySupportedMidpointDeflection
  apply div_pos
  · exact mul_pos hForce (pow_pos hLength 3)
  · exact mul_pos (mul_pos (by norm_num : (0 : ℝ) < 48) hModulus) hInertia

theorem simplySupportedMidpointDeflection_force_scaling
    (force length youngsModulus inertia scale : ℝ)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    simplySupportedMidpointDeflection (scale * force) length youngsModulus inertia =
      scale * simplySupportedMidpointDeflection force length youngsModulus inertia := by
  unfold simplySupportedMidpointDeflection
  have hden : 48 * youngsModulus * inertia ≠ 0 :=
    ne_of_gt (mul_pos (mul_pos (by norm_num : (0 : ℝ) < 48) hModulus) hInertia)
  field_simp [hden]
  <;> ring

theorem simplySupportedMidpointDeflection_length_cubic_scaling
    (force length youngsModulus inertia scale : ℝ)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    simplySupportedMidpointDeflection force (scale * length) youngsModulus inertia =
      scale ^ 3 * simplySupportedMidpointDeflection force length youngsModulus inertia := by
  unfold simplySupportedMidpointDeflection
  have hden : 48 * youngsModulus * inertia ≠ 0 :=
    ne_of_gt (mul_pos (mul_pos (by norm_num : (0 : ℝ) < 48) hModulus) hInertia)
  field_simp [hden]
  <;> ring

theorem simplySupportedUniformLoadDeflection_pos
    {loadIntensity length youngsModulus inertia : ℝ}
    (hLoad : 0 < loadIntensity) (hLength : 0 < length)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    0 < simplySupportedUniformLoadDeflection
      loadIntensity length youngsModulus inertia := by
  unfold simplySupportedUniformLoadDeflection
  apply div_pos
  · exact mul_pos (mul_pos (by norm_num : (0 : ℝ) < 5) hLoad)
      (pow_pos hLength 4)
  · exact mul_pos (mul_pos (by norm_num : (0 : ℝ) < 384) hModulus) hInertia

theorem simplySupportedUniformLoadDeflection_load_scaling
    (loadIntensity length youngsModulus inertia scale : ℝ)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    simplySupportedUniformLoadDeflection
      (scale * loadIntensity) length youngsModulus inertia =
      scale * simplySupportedUniformLoadDeflection
        loadIntensity length youngsModulus inertia := by
  unfold simplySupportedUniformLoadDeflection
  have hden : 384 * youngsModulus * inertia ≠ 0 :=
    ne_of_gt (mul_pos (mul_pos (by norm_num : (0 : ℝ) < 384) hModulus) hInertia)
  field_simp [hden]
  <;> ring

theorem simplySupportedUniformLoadDeflection_length_quartic_scaling
    (loadIntensity length youngsModulus inertia scale : ℝ)
    (hModulus : 0 < youngsModulus) (hInertia : 0 < inertia) :
    simplySupportedUniformLoadDeflection
      loadIntensity (scale * length) youngsModulus inertia =
      scale ^ 4 * simplySupportedUniformLoadDeflection
        loadIntensity length youngsModulus inertia := by
  unfold simplySupportedUniformLoadDeflection
  have hden : 384 * youngsModulus * inertia ≠ 0 :=
    ne_of_gt (mul_pos (mul_pos (by norm_num : (0 : ℝ) < 384) hModulus) hInertia)
  field_simp [hden]
  <;> ring

end SymtropyFormal
