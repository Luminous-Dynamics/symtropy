# Executable experiment units: semantics, interoperability, and benchmarks

**Status:** Unit conversion prototype implemented in `symtropy-experiment-core`; Rust execution and measured performance are still pending independent verification.

## Why this is a paradigm shift

A unit label must affect computation, not merely decorate a number. Treat every quantity as a numeric value plus a dimension and a unit scale. Validate all three before execution, normalize values to a canonical basis for arithmetic, and render results in the declared output unit. The equation graph remains the single source of executable meaning; inspection, visualization, parameter sweeps, and later solver adapters should all consume that same typed graph.

This closes a consequential class of silent errors. For example, adding 100 cm to 1 m must produce 2 m, not 101 m; an input of 180 degrees must be treated as pi radians when used by an equation whose canonical angular unit is radian. Unknown units or units whose dimensions disagree with the variable declaration must reject the model instead of being assumed to have scale 1.

## Implemented prototype

The new Rust `units` module provides a deliberately limited, explicit registry and expression parser. It supports dimension vectors and scale-to-canonical conversion for selected SI base/derived units and common engineering units, plus expressions joined by `*` and `/` with signed integer powers such as `m/s^2` and `kg*m/s^2`. The experiment graph now checks that every variable's unit exists and resolves to its declared `Dimension`. The evaluator converts supplied and overridden values to canonical units, evaluates the graph there, then converts every returned value to that variable's declared unit.

The registry includes length, mass, time, electric current, temperature, amount of substance, luminous intensity, semantic angle, selected derived units (Hz, N, Pa, J, W, C, V, ohm, F, T), common metric scales, customary length units, and the prototype's dimensionless `tooth` token.

The registry exposes `UNIT_REGISTRY_VERSION = "0.1.0"`. The `inspect`, single-run, and sweep CLI envelopes carry this version, and visual projections carry it alongside the model. Any change to supported symbols, scale factors, or parsing semantics must bump this version. This is a manually maintained semantic version, not a content hash; reproducible archival runs should also retain the source revision and exact model/input digests. Angle remains a semantic eighth axis in this model: this is an explicit guardrail, not a claim that SI defines plane angle as a separate base dimension.

**Not implemented:** the full UCUM grammar or unit catalog; parentheses and implicit multiplication; affine conversions such as Celsius/Fahrenheit; uncertainty propagation through conversions; quantity-specific restrictions such as “tooth count cannot be added to a generic ratio”; cached parsed units; high-precision arithmetic; and a general physical solver. The current parser is a small supported subset, not UCUM-compliant. Unknown symbols fail closed.

## Research and standards path

- Prefer interoperable, explicitly versioned unit semantics rather than growing an unbounded hand-built alias list. UCUM describes machine-readable units and their combinations using unit atoms, expression syntax, conversion scales, and dimensional analysis: <https://ucum.org/ucum>.
- Anchor SI base and derived-unit semantics to NIST's public reference material: <https://www.nist.gov/pml/owm/metric-si/si-units> and <https://www.nist.gov/si-redefinition/definitions-si-base-units>.
- Treat unit correctness as one model-verification check, not as proof that an equation is scientifically true or that a simulation matches reality. NASA-STD-7009B is active guidance on credibility and acceptance criteria for models and simulations: <https://standards.nasa.gov/standard/nasa/nasa-std-7009>.
- Keep unit conversion separate from verification, empirical validation, uncertainty quantification, sensitivity analysis, calibration, and domain-specific physical constraints. Passing conversion tests cannot substitute for these other forms of evidence.

The long-term architecture should offer adapters with explicit capability and loss reports: if an imported model uses a unit, offset, semantic quantity, or uncertainty type that an adapter cannot preserve, the adapter should report the limitation or refuse the import. It must not silently coerce incompatible quantities.

## Regression benchmarks

These are proposed acceptance gates, not current achieved results.

| Gate | Benchmark | Target |
|---|---|---|
| Unit definition | Every supported atomic unit and representative compound expression | 100% match against hand-reviewed reference vectors and conversion factors |
| Unknown units | Unknown symbol, malformed expression, unsupported affine unit | 100% rejected; never implicitly interpreted as scale 1 |
| Dimension contract | Deliberately mismatched unit and declared dimension | 100% rejected before evaluation |
| Conversion semantics | Cross-scale add/subtract; scaled input, intermediate and output units; negative quantities | Known answers within declared binary64 tolerance (start with absolute error <= 1e-12 for bounded test cases) |
| Regression safety | Existing gear fixture and oscillator analysis | Existing known answers and convergence trends remain within their current tolerances |
| Reproducibility | Same graph, inputs, and unit registry version | Identical model identity and outputs within declared serialization/numerical contract |
| Runtime cost | Release-mode baseline versus unit-aware graph evaluation, 10/100/1,000 equation-node fixtures | Record p50, p95, and throughput; initial goal <= 25% median overhead on 100-node fixtures, then optimize based on measured profile |
| Interoperability | UCUM reference corpus and a second independent parser/adapter | Report supported/unsupported coverage and zero silent semantic loss; do not claim compatibility until exercised |

Performance tests must compare equal work, include validation and conversion costs explicitly, pin the compiler/target profile and fixture digest, and record raw measurements. No result is currently claimed for these performance targets.

## Next engineering steps

1. Add more independently checked unit vectors and generate positive/negative tests from a versioned table.
2. Decide whether the next step should be a proper UCUM-compatible adapter or a stable internal unit algebra with explicit UCUM import/export and capability reports; avoid claiming the present subset is standard-compliant.
3. Add affine-unit support as a separate offset-aware conversion type, not as a multiplicative scale.
4. Cache parsed unit definitions only after profiling; preserve fail-closed validation on deserialize and model revision changes.
5. Carry uncertainty intervals and source provenance through unit-aware transformations without confusing display rounding with the internal numeric representation.
6. Integrate solver adapters only when they declare units, state variables, time bases, tolerances, assumptions, validity domain, and loss report.
