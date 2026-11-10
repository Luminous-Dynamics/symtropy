# SAM road-option analytic fixture validator

This is a dependency-free Python 3.12 **research/test validator**, not a production vehicle component. The SAM runtime and evaluators remain Rust-first. The script validates the frozen analytic fixture's domain, timing, evidence-class consistency, action outcome completeness, query-delivery census and expected set calculations.

Run from the Symtropy repository root:

```bash
python -m unittest discover -s tools/road-option-fixture-validator/tests -v
python tools/road-option-fixture-validator/validate_fixture.py
```

The semantic validator is intentionally narrower than a general JSON Schema Draft 2020-12 engine. The schema is parsed and its draft is checked; the validator independently enforces the SAM-specific semantic invariants that a generic structural schema cannot express. It uses only the Python standard library.

A successful result means that the declared finite analytic fixture is internally coherent. It does not validate physical dynamics, human behavior, sensors, causal validity outside the fixture, collision risk, vehicle safety, roadworthiness or deployment.
