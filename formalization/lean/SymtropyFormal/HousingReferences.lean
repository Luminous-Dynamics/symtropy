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

/-- Cross-sectional area for a rectangular section. -/
def rectangularSectionArea (breadth depth : ℝ) : ℝ :=
  breadth * depth

/-- Distance from a rectangular section's neutral axis to its extreme fiber. -/
def extremeFiberDistance (depth : ℝ) : ℝ :=
  depth / 2

/-- Positive axial stress magnitude for a prismatic bar. -/
def axialBarStress (force area : ℝ) : ℝ :=
  force / area

/-- Positive axial support-reaction magnitude for the one-dimensional bar. -/
def axialBarSupportReactionMagnitude (force : ℝ) : ℝ :=
  force

/-- Cantilever support bending-moment magnitude for a tip point load. -/
def cantileverMaximumBendingMoment (force span : ℝ) : ℝ :=
  force * span

/-- Elementary bending-stress magnitude from moment, fiber distance and inertia. -/
def beamBendingStress (moment fiberDistance inertia : ℝ) : ℝ :=
  moment * fiberDistance / inertia

/-- Cantilever support-reaction magnitude for a tip point load. -/
def cantileverSupportReactionMagnitude (force : ℝ) : ℝ :=
  force

/-- Maximum bending moment for a simply supported beam with a midpoint force. -/
def simplySupportedMidpointMaximumBendingMoment (force span : ℝ) : ℝ :=
  force * span / 4

/-- Support-reaction magnitude for either support under a midpoint point load. -/
def simplySupportedMidpointSupportReactionMagnitude (force : ℝ) : ℝ :=
  force / 2

/-- Maximum bending moment for a simply supported beam under uniform load. -/
def simplySupportedUniformMaximumBendingMoment (loadIntensity span : ℝ) : ℝ :=
  loadIntensity * span ^ 2 / 8

/-- Support-reaction magnitude for either support under uniform load. -/
def simplySupportedUniformSupportReactionMagnitude (loadIntensity span : ℝ) : ℝ :=
  loadIntensity * span / 2
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


/-! ## Exact rational checks for candidate benchmark fixtures

These values correspond to the candidate JSON fixtures on housing-evidence PR
#1653. They are exact real-number identities, not executions of the Rust code.
-/

theorem rectangularSecondMoment_cantilever_fixture_value :
    rectangularSecondMoment ((12 : ℝ) / 125) ((1 : ℝ) / 10) =
      (1 : ℝ) / 125000 := by
  norm_num [rectangularSecondMoment]

theorem rectangularSecondMoment_simplySupported_fixture_value :
    rectangularSecondMoment ((1 : ℝ) / 10) ((1 : ℝ) / 5) =
      (1 : ℝ) / 15000 := by
  norm_num [rectangularSecondMoment]

theorem axialBar_fixture_extension_value :
    axialExtension 12000 3 200000000000 ((1 : ℝ) / 200) =
      (9 : ℝ) / 250000 := by
  norm_num [axialExtension]

theorem cantilever_fixture_tip_deflection_value :
    cantileverTipDeflection 1000 2 200000000000 ((1 : ℝ) / 125000) =
      (1 : ℝ) / 600 := by
  norm_num [cantileverTipDeflection]

theorem simplySupportedMidpoint_fixture_deflection_value :
    simplySupportedMidpointDeflection 5000 4 200000000000
      ((1 : ℝ) / 15000) = (1 : ℝ) / 2000 := by
  norm_num [simplySupportedMidpointDeflection]

theorem simplySupportedUniformLoad_fixture_deflection_value :
    simplySupportedUniformLoadDeflection 1000 4 200000000000
      ((1 : ℝ) / 15000) = (1 : ℝ) / 4000 := by
  norm_num [simplySupportedUniformLoadDeflection]

/-! ## Exact rational checks for the complete contract-vector output ledger -/

theorem rectangularSectionArea_cantilever_fixture_value :
    rectangularSectionArea ((12 : ℝ) / 125) ((1 : ℝ) / 10) =
      (12 : ℝ) / 1250 := by
  norm_num [rectangularSectionArea]

theorem extremeFiberDistance_cantilever_fixture_value :
    extremeFiberDistance ((1 : ℝ) / 10) = (1 : ℝ) / 20 := by
  norm_num [extremeFiberDistance]

theorem rectangularSectionArea_axialBar_fixture_value :
    rectangularSectionArea ((1 : ℝ) / 20) ((1 : ℝ) / 10) =
      (1 : ℝ) / 200 := by
  norm_num [rectangularSectionArea]

theorem axialBarStress_fixture_value :
    axialBarStress 12000 ((1 : ℝ) / 200) = 2400000 := by
  norm_num [axialBarStress]

theorem axialBarSupportReaction_fixture_value :
    axialBarSupportReactionMagnitude 12000 = 12000 := by
  norm_num [axialBarSupportReactionMagnitude]

theorem cantileverMaximumBendingMoment_fixture_value :
    cantileverMaximumBendingMoment 1000 2 = 2000 := by
  norm_num [cantileverMaximumBendingMoment]

theorem cantileverMaximumBendingStress_fixture_value :
    beamBendingStress
      (cantileverMaximumBendingMoment 1000 2)
      (extremeFiberDistance ((1 : ℝ) / 10))
      (rectangularSecondMoment ((12 : ℝ) / 125) ((1 : ℝ) / 10)) =
      12500000 := by
  norm_num [beamBendingStress, cantileverMaximumBendingMoment,
    extremeFiberDistance, rectangularSecondMoment]

theorem cantileverSupportReaction_fixture_value :
    cantileverSupportReactionMagnitude 1000 = 1000 := by
  norm_num [cantileverSupportReactionMagnitude]

theorem rectangularSectionArea_simplySupported_fixture_value :
    rectangularSectionArea ((1 : ℝ) / 10) ((1 : ℝ) / 5) =
      (1 : ℝ) / 50 := by
  norm_num [rectangularSectionArea]

theorem extremeFiberDistance_simplySupported_fixture_value :
    extremeFiberDistance ((1 : ℝ) / 5) = (1 : ℝ) / 10 := by
  norm_num [extremeFiberDistance]

theorem simplySupportedMidpointMaximumBendingMoment_fixture_value :
    simplySupportedMidpointMaximumBendingMoment 5000 4 = 5000 := by
  norm_num [simplySupportedMidpointMaximumBendingMoment]

theorem simplySupportedMidpointMaximumBendingStress_fixture_value :
    beamBendingStress
      (simplySupportedMidpointMaximumBendingMoment 5000 4)
      (extremeFiberDistance ((1 : ℝ) / 5))
      (rectangularSecondMoment ((1 : ℝ) / 10) ((1 : ℝ) / 5)) =
      7500000 := by
  norm_num [beamBendingStress, simplySupportedMidpointMaximumBendingMoment,
    extremeFiberDistance, rectangularSecondMoment]

theorem simplySupportedMidpointSupportReaction_fixture_value :
    simplySupportedMidpointSupportReactionMagnitude 5000 = 2500 := by
  norm_num [simplySupportedMidpointSupportReactionMagnitude]

theorem simplySupportedUniformMaximumBendingMoment_fixture_value :
    simplySupportedUniformMaximumBendingMoment 1000 4 = 2000 := by
  norm_num [simplySupportedUniformMaximumBendingMoment]

theorem simplySupportedUniformMaximumBendingStress_fixture_value :
    beamBendingStress
      (simplySupportedUniformMaximumBendingMoment 1000 4)
      (extremeFiberDistance ((1 : ℝ) / 5))
      (rectangularSecondMoment ((1 : ℝ) / 10) ((1 : ℝ) / 5)) =
      3000000 := by
  norm_num [beamBendingStress, simplySupportedUniformMaximumBendingMoment,
    extremeFiberDistance, rectangularSecondMoment]

theorem simplySupportedUniformSupportReaction_fixture_value :
    simplySupportedUniformSupportReactionMagnitude 1000 4 = 2000 := by
  norm_num [simplySupportedUniformSupportReactionMagnitude]

end SymtropyFormal
