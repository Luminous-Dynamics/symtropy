# RNG alignment evidence: crossing-traffic smoke v0.1

The runner uses one deterministic linear-congruential stream per declared seed to generate the four agents' initial coordinate jitter. Every draw is consumed during scenario construction, in stable agent/axis order, before the treatment switches are applied. There are no random draws inside the integration loop. The four-agent start/goal layout, controller constants, fixed inputs, solver, and time step are identical across the eight F/R/E arms for a given seed and parameter condition.

The adapter records a SHA-256 digest of the complete initial state. For each seed it checks that this digest is identical across every factorial arm and every source-strength/radius variant. It then runs each configuration a second time from a fresh world and requires byte-identical serialized trace output. A mismatch makes the run unqualified.

This evidence establishes matched seeded initial conditions and within-process deterministic replay for this smoke scenario. It does not claim event-level common random numbers or stochastic-trajectory alignment: the simulation loop is deterministic and consumes no RNG after initialization. The same-seed alignment claim must not be generalized to other Symtropy tasks or RNG implementations.
