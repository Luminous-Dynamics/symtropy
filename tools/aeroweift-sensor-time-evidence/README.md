# AeroWeft Sensor Temporal Evidence Oracle

Synthetic-first Rust structural evaluator for the temporal evidence behind heterogeneous sensor association and fusion.

It distinguishes independently hardware-bounded time, qualified shared-clock time, software acquisition-cycle association, and order-only/unknown records. It never promotes a sequence index or shared filename into a timestamp. Modality-specific results remain available when cross-modal timing is insufficient.

This crate does not implement radar/RF acquisition, a detector, track fusion, a clock service, common-mode independence graph, cryptographic provenance, sensor calibration, or operational DAA. It consumes those owner-qualified declarations and composes AeroWeft #5275/#5270/#5351 plus Mycelix #435/#5899/#5904.

Public TSMS-Drone is a controlled 2–30 m dataset, not an airborne coverage or flight-performance dataset. Its software-loop and index association should be classified as cycle-associated unless independent bounded time evidence is provided.
