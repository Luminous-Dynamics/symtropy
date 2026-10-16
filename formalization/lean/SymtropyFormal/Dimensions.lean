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

theorem axialStress_dimensions :
    stressDim = youngsModulusDim := by decide

theorem bendingMoment_dimensions :
    PhysicalDimension.mul forceDim lengthDim = bendingMomentDim := by decide

theorem supportReaction_dimensions :
    forceDim = supportReactionDim := by decide

/-! ## Negative control: a missing length power must not masquerade as deflection -/

def cantileverMissingLengthPowerDim : PhysicalDimension :=
  PhysicalDimension.div (PhysicalDimension.mul forceDim lengthDim)
    (PhysicalDimension.mul youngsModulusDim secondMomentAreaDim)

theorem cantilever_missing_length_power_is_not_length :
    cantileverMissingLengthPowerDim ≠ lengthDim := by decide

end SymtropyFormal
