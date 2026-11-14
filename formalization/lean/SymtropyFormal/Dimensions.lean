import Mathlib

/-!
# Dimensional analysis for Symtropy engineering contracts

A physical dimension is a vector of integer exponents over the seven SI base
dimensions. Multiplication adds exponent vectors, division subtracts them, and
natural powers scale them. This file checks dimensional homogeneity of the
closed-form structural equations independently of their numeric values.

Important boundary: these theorems validate the *dimension equations*. The
existing housing functions still accept untyped real numbers. A future Rust
binding must map named input/output units to this catalogue and test unit
normalization at runtime or encode units in types.
-/

namespace SymtropyFormal

/-- Exponents of the seven SI base dimensions: L, M, T, I, Θ, N, J. -/
structure PhysicalDimension where
  lengthExponent : Int
  massExponent : Int
  timeExponent : Int
  currentExponent : Int
  temperatureExponent : Int
  amountExponent : Int
  luminousIntensityExponent : Int
  deriving DecidableEq, Repr

namespace PhysicalDimension

/-- Product of physical dimensions adds each base-dimension exponent. -/
def mul (a b : PhysicalDimension) : PhysicalDimension where
  lengthExponent := a.lengthExponent + b.lengthExponent
  massExponent := a.massExponent + b.massExponent
  timeExponent := a.timeExponent + b.timeExponent
  currentExponent := a.currentExponent + b.currentExponent
  temperatureExponent := a.temperatureExponent + b.temperatureExponent
  amountExponent := a.amountExponent + b.amountExponent
  luminousIntensityExponent := a.luminousIntensityExponent + b.luminousIntensityExponent

/-- Quotient of physical dimensions subtracts each base-dimension exponent. -/
def div (a b : PhysicalDimension) : PhysicalDimension where
  lengthExponent := a.lengthExponent - b.lengthExponent
  massExponent := a.massExponent - b.massExponent
  timeExponent := a.timeExponent - b.timeExponent
  currentExponent := a.currentExponent - b.currentExponent
  temperatureExponent := a.temperatureExponent - b.temperatureExponent
  amountExponent := a.amountExponent - b.amountExponent
  luminousIntensityExponent := a.luminousIntensityExponent - b.luminousIntensityExponent

/-- Natural power scales each exponent. -/
def pow (a : PhysicalDimension) (n : Nat) : PhysicalDimension where
  lengthExponent := (n : Int) * a.lengthExponent
  massExponent := (n : Int) * a.massExponent
  timeExponent := (n : Int) * a.timeExponent
  currentExponent := (n : Int) * a.currentExponent
  temperatureExponent := (n : Int) * a.temperatureExponent
  amountExponent := (n : Int) * a.amountExponent
  luminousIntensityExponent := (n : Int) * a.luminousIntensityExponent

end PhysicalDimension

open PhysicalDimension

def dimensionlessDim : PhysicalDimension := ⟨0, 0, 0, 0, 0, 0, 0⟩
def lengthDim : PhysicalDimension := ⟨1, 0, 0, 0, 0, 0, 0⟩
def massDim : PhysicalDimension := ⟨0, 1, 0, 0, 0, 0, 0⟩
def timeDim : PhysicalDimension := ⟨0, 0, 1, 0, 0, 0, 0⟩
def currentDim : PhysicalDimension := ⟨0, 0, 0, 1, 0, 0, 0⟩
def temperatureDim : PhysicalDimension := ⟨0, 0, 0, 0, 1, 0, 0⟩
def amountDim : PhysicalDimension := ⟨0, 0, 0, 0, 0, 1, 0⟩
def luminousIntensityDim : PhysicalDimension := ⟨0, 0, 0, 0, 0, 0, 1⟩

def areaDim : PhysicalDimension := PhysicalDimension.pow lengthDim 2
def secondMomentAreaDim : PhysicalDimension := PhysicalDimension.pow lengthDim 4
def forceDim : PhysicalDimension :=
  PhysicalDimension.div (PhysicalDimension.mul massDim lengthDim)
    (PhysicalDimension.pow timeDim 2)
def pressureDim : PhysicalDimension := PhysicalDimension.div forceDim areaDim
/-- SI dimension of elastic modulus / pressure: kg m^-1 s^-2. -/
def youngsModulusDim : PhysicalDimension := ⟨-1, 1, -2, 0, 0, 0, 0⟩
def stressDim : PhysicalDimension := PhysicalDimension.div forceDim areaDim
/-- SI dimension of moment: N m = kg m^2 s^-2. -/
def bendingMomentDim : PhysicalDimension := ⟨2, 1, -2, 0, 0, 0, 0⟩
def lineLoadDim : PhysicalDimension := PhysicalDimension.div forceDim lengthDim
/-- Independently declared SI dimension for a support reaction force. -/
def supportReactionDim : PhysicalDimension := ⟨1, 1, -2, 0, 0, 0, 0⟩

/-! ## Dimensional homogeneity for the engineering contracts -/

theorem rectangularSecondMoment_dimensions :
    PhysicalDimension.mul lengthDim (PhysicalDimension.pow lengthDim 3) =
      secondMomentAreaDim := by decide

theorem rectangularSectionArea_dimensions :
    PhysicalDimension.mul lengthDim lengthDim = areaDim := by decide

theorem axialExtension_dimensions :
    PhysicalDimension.div
      (PhysicalDimension.mul forceDim lengthDim)
      (PhysicalDimension.mul youngsModulusDim areaDim) = lengthDim := by decide

theorem cantileverTipDeflection_dimensions :
    PhysicalDimension.div
      (PhysicalDimension.mul forceDim (PhysicalDimension.pow lengthDim 3))
      (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim) =
        lengthDim := by decide

theorem simplySupportedMidpointDeflection_dimensions :
    PhysicalDimension.div
      (PhysicalDimension.mul forceDim (PhysicalDimension.pow lengthDim 3))
      (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim) =
        lengthDim := by decide

theorem simplySupportedUniformLoadDeflection_dimensions :
    PhysicalDimension.div
      (PhysicalDimension.mul lineLoadDim (PhysicalDimension.pow lengthDim 4))
      (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim) =
        lengthDim := by decide

theorem axialBarStress_formula_dimensions :
    PhysicalDimension.div forceDim areaDim = stressDim := by decide

theorem axialStress_dimensions :
    stressDim = youngsModulusDim := by decide

theorem bendingMoment_dimensions :
    PhysicalDimension.mul forceDim lengthDim = bendingMomentDim := by decide

theorem beamBendingStress_dimensions :
    PhysicalDimension.div
      (PhysicalDimension.mul bendingMomentDim lengthDim)
      secondMomentAreaDim = stressDim := by decide

theorem simplySupportedUniformMaximumBendingMoment_dimensions :
    PhysicalDimension.mul lineLoadDim (PhysicalDimension.pow lengthDim 2) =
      bendingMomentDim := by decide

theorem supportReaction_dimensions :
    forceDim = supportReactionDim := by decide

/-! ## Negative control: a missing length power must not masquerade as deflection -/

def cantileverMissingLengthPowerDim : PhysicalDimension :=
  PhysicalDimension.div (PhysicalDimension.mul forceDim lengthDim)
    (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim)

theorem cantilever_missing_length_power_is_not_length :
    cantileverMissingLengthPowerDim ≠ lengthDim := by decide

/-! ## Dimensionless groups for scale-transfer benchmarks

The groups below are dimensionally invariant. Their invariance is the basis for
cross-scale metamorphic tests: when geometry and loads are scaled to keep the
group fixed, normalized results should agree under the stated linear model.
This is a mathematical property of the reference equations, not yet a test of
the Rust implementation or a claim that real materials remain scale-similar.
-/

theorem rectangularSectionShapeFactor_dimensions :
    PhysicalDimension.div
      (PhysicalDimension.mul lengthDim (PhysicalDimension.pow lengthDim 3))
      (PhysicalDimension.pow lengthDim 4) = dimensionlessDim := by decide

theorem axialExtension_normalized_strain_dimensionless :
    PhysicalDimension.div
      (PhysicalDimension.div
        (PhysicalDimension.mul forceDim lengthDim)
        (PhysicalDimension.mul youngsModulusDim areaDim))
      lengthDim = dimensionlessDim := by decide

theorem cantilever_normalized_deflection_dimensionless :
    PhysicalDimension.div
      (PhysicalDimension.div
        (PhysicalDimension.mul forceDim (PhysicalDimension.pow lengthDim 3))
        (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim))
      lengthDim = dimensionlessDim := by decide

theorem simplySupportedMidpoint_normalized_deflection_dimensionless :
    PhysicalDimension.div
      (PhysicalDimension.div
        (PhysicalDimension.mul forceDim (PhysicalDimension.pow lengthDim 3))
        (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim))
      lengthDim = dimensionlessDim := by decide

theorem simplySupportedUniformLoad_normalized_deflection_dimensionless :
    PhysicalDimension.div
      (PhysicalDimension.div
        (PhysicalDimension.mul lineLoadDim (PhysicalDimension.pow lengthDim 4))
        (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim))
      lengthDim = dimensionlessDim := by decide

/-! ## Unit-catalogue anchors

These kernel-checked identities state the canonical exponent vector for each
unit label used in contracts.json. CI separately checks the manifest labels and
vector values, so unit-schema changes require a reviewed versioned update.
-/

theorem unit_m_vector :
    lengthDim = ⟨1, 0, 0, 0, 0, 0, 0⟩ := by decide

theorem unit_m2_vector :
    areaDim = ⟨2, 0, 0, 0, 0, 0, 0⟩ := by decide

theorem unit_m4_vector :
    secondMomentAreaDim = ⟨4, 0, 0, 0, 0, 0, 0⟩ := by decide

theorem unit_N_vector :
    forceDim = ⟨1, 1, -2, 0, 0, 0, 0⟩ := by decide

theorem unit_Pa_vector :
    youngsModulusDim = ⟨-1, 1, -2, 0, 0, 0, 0⟩ := by decide

theorem unit_Nm_vector :
    bendingMomentDim = ⟨2, 1, -2, 0, 0, 0, 0⟩ := by decide

theorem unit_N_per_m_vector :
    lineLoadDim = ⟨0, 1, -2, 0, 0, 0, 0⟩ := by decide

end SymtropyFormal
