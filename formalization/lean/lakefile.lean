import Lake
open Lake DSL

package symtropyFormal where
  version := v!"0.1.0"

-- Immutable Mathlib release commit matched to Lean v4.35.0.
require mathlib from git
  "https://github.com/leanprover-community/mathlib4" @ "56a005855ea17e0b0958f73ec801ae140b0a1679"

lean_lib SymtropyFormal
