# LOCO-ASYNC-THERMAL-001 — persistent thermal asynchronous locomotion benchmark

Owner: [Symtropy #4689](https://github.com/Luminous-Dynamics/symtropy/issues/4689)

This folder contains the finite composition corpus for asynchronous action execution under persistent thermal and mission-reserve state.

- [fixtures/mission_interactions_v0_1.json](./fixtures/mission_interactions_v0_1.json)
- [fixtures/mission_interactions_v0_1.schema.json](./fixtures/mission_interactions_v0_1.schema.json)

The corpus checks model/observation/prefix identity, clock mapping, conservative observation age, expiry, deadline, per-node thermal limits, thermal predecessor lineage, cumulative heat carryover, compute-cost reserve, queue underflow, independent RTA veto and observation-constrained recovery reserve.

All values are synthetic integer known answers. They do not estimate physical thermal transitions or qualify hardware. A passing case means only eligible for independent projection; it never authorizes actuation.

Existing owners remain authoritative: #4610 thermal dynamics; #4549/#4555 persistent mission reserve; #4588 future options; #4117 certificate validity; #4382 projection; #2836 controller dispatch.

## Validation

A dedicated workflow, `.github/workflows/loco-async-thermal.yml`, validates the Draft 2020-12 schema and independently replays the finite semantic controls. It verifies the age upper bound, chunk expiry/deadline, cumulative two-event temperature arithmetic, per-node limit masking, reserve preservation, and fail-closed Unknown/RTA/queue dispositions. A workflow pass validates only this synthetic corpus, not the thermal model or hardware.
