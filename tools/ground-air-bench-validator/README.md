# GroundAirBench semantic validator v0

The validator joins receipt references and checks required roles, field-configuration binding, evidence dispositions, validity intervals, receiver opportunity references and benchmark-only classifications.

It does not validate JSON Schema 2020-12, meter physical energy, allocate power, estimate physical pose, control equipment or issue an operational permit.

The stacked evaluator PR targets the `research/ground-air-bench-v0-20261108` branch so the parent workflow explicitly includes that base branch in its pull-request trigger filter.
