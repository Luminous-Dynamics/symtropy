# LOCO-ADAPTIVE-HORIZON-001 — viability-bounded action horizon

Owner: [Symtropy #4696](https://github.com/Luminous-Dynamics/symtropy/issues/4696)

The benchmark distinguishes queued open-loop execution from a split-timescale architecture with a continuously operating local action expert.

- [fixtures/horizon_composition_v0_1.json](./fixtures/horizon_composition_v0_1.json)
- [fixtures/horizon_composition_v0_1.schema.json](./fixtures/horizon_composition_v0_1.schema.json)

The corpus tests the minimum qualified horizon across observation freshness, chunk expiry, action deadline, certificate validity, contact-model validity, actuator envelope, thermal-to-limit and terminal recovery reserve. It checks whether queued inference can be safely covered (`K_min <= K_max`) and whether split-timescale local execution can continue without spanning slow world-context latency.

All cases are synthetic integer known answers; they do not estimate upstream horizons or authorize actuation.