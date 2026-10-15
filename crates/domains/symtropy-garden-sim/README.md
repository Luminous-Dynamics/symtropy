# Symtropy Garden Infrastructure Scenario Kernel

This isolated Rust crate provides an exact integer accounting checker for one resource balance. It is a starter kernel for the proposed garden-infrastructure scenario described in [the garden-to-facility contract](https://github.com/Luminous-Dynamics/symtropy/pull/1630).

## Resource units

- Water: integer millilitres
- Electrical energy: integer watt-hours
- Material: integer grams

Every field in one balance must use the unit fixed by its resource kind. The checker does not perform unit conversion. Flow, addition, loss and tolerance fields are non-negative unsigned integers; the storage delta is signed. Expected balance is recomputed with wide integer arithmetic and the absolute error uses overflow-safe integer difference.

## Wire format

To preserve exactness across JSON clients that use IEEE-754 numeric values, all `u64`, `i128` and `u128` quantities are serialized as decimal strings (for example, `"18446744073709551615"`), not JSON numbers. The schema version remains a small integer. Consumers must parse these quantity strings with integer-safe handling and must not coerce them through floating-point numbers.

## Run

```sh
cargo fmt --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml -- --check
cargo test --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml
cargo clippy --manifest-path crates/domains/symtropy-garden-sim/Cargo.toml --all-targets -- -D warnings
```

## Exact nutrient contribution

The crate also evaluates one nutrient contribution from a source-pinned food-composition row and an already-calculated edible-delivery mass:

- Composition is scoped to an exact food record and food/preparation state, with a nutrient-definition reference, unit, source reference, evidence class and explicit decimal scale.
- Delivery is expressed as grams of edible food actually delivered, not gross harvest biomass, and is bound to an explicit community/population scope and time period. The upstream system must already account for edible fraction, loss and allocation.
- A missing composition value returns `NotAssessable`, not a zero contribution. A food identity, preparation-state, scale or provenance mismatch blocks calculation.
- The output retains the exact edible-delivered mass, beneficiary scope, period, source references and rational numerator/denominator in the stated nutrient unit. It avoids rounding and floating-point arithmetic; wide JSON integers are decimal strings.
- This calculation does not authenticate evidence, choose crops, infer yields or requirements, aggregate a diet, or prove nutrient adequacy.

The synthetic test fixture declares 250 mg per 100 g at scale 1,000 and 75 g of edible food delivered. The exact contribution is represented as (18{,}750{,}000 / 100{,}000 = 187.5) mg. These values test arithmetic only; they are not a claim about a real food.

The next step is not to invent a crop catalogue: add a licensed, versioned composition source registry, explicit population targets and a seasonal production portfolio contract before implementing any optimizer.

## Declared nutrient target comparison

`assess_nutrient_target` compares that exact contribution with an explicit minimum target bound to a community/population scope and time-period reference. It requires the same canonical nutrient definition, base unit and decimal scale. Its result is one of:

- `MeetsDeclaredMinimum`
- `ShortfallAgainstDeclaredMinimum`
- `NotAssessable` when composition is missing
- `Blocked` for mismatched definitions/units/scales or missing target/provenance metadata

Supply, target and shortfall are expressed as exact integer rational numerators over the same denominator. No float percentage or rounded adequacy score is emitted. The delivery scope and period must exactly match the target scope and period; cross-scope or cross-period comparisons block. The target must be independently sourced for that scope and interval; this function does not derive dietary requirements or check whether the target is clinically appropriate for individuals.

## Multi-food nutrient-supply aggregation

`evaluate_nutrient_supply_scenario` aggregates multiple source-pinned edible-delivery lines against one declared nutrient target. Every line must use the same nutrient definition, unit, decimal scale, beneficiary scope and period. The evaluator rejects duplicate delivery-source references, scenario/scope/period mismatches and arithmetic overflow. If one required composition value is missing, it withholds the total rather than treating that line as zero.

The portfolio receipt includes each food/preparation identity, delivered edible grams, source references and individual exact contribution alongside the aggregate total/shortfall. Repeated composition source refs are allowed when multiple food rows come from the same dataset artifact; delivery refs must not be repeated. This remains a nutrient-supply checker—not a crop chooser, full dietary optimizer or proof that individuals receive an adequate diet.



## Claim ceiling

The checker independently recomputes a declared interval balance from supplied quantities. It does not simulate fluid flow, infer water quality, predict crop yield, identify plants, model medicine stability, or prove physical validity. Inputs can still be wrong or fabricated; source references are not fetched or authenticated. A consistent accounting result is not proof that the physical model or external observation is correct.

The next integration step is to bind this contract into Symtropy's existing replay/scenario substrate after confirming workspace ownership and supported physics-model capabilities.
