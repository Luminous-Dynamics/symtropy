# Symtropy Soil Water Kernel

A small, deterministic Rust reference kernel for accounting soil-water storage from supplied interval-integrated fluxes. This is a conservation contract, not a full hydrology or crop-growth simulator.

## Units and temporal semantics

- Storage is an equivalent water depth in millimetres over the horizontal control-volume footprint.
- Precipitation, irrigation, evapotranspiration, runoff and drainage are millimetres accumulated during one configured timestep. They are not rates such as mm/hour.
- Every depth shares the same horizontal control area. This makes the balance equation independent of area; a typed area is still required to convert depths into cubic metres.
- The timestep duration is recorded as seconds but does not rescale flux amounts. The caller must aggregate inputs to the configured interval before calling the step function.

## Accounting identity

    storage_next = storage_start
                 + precipitation + irrigation
                 - evapotranspiration - runoff - drainage

The kernel rejects invalid/non-finite/negative inputs, unexplained overdraw, and storage above the configured capacity. It never silently clamps negative storage or invents runoff to absorb excess. A successful report includes a balance residual and scale-aware floating-point tolerance.

## What this does not claim

This crate does not predict precipitation, evapotranspiration, runoff, drainage, infiltration, soil moisture profiles, crop yield, microbial dynamics, or amendment efficacy. It contains no fitted soil parameters or hidden empirical constants. Synthetic fixtures prove accounting invariants only; they are not soil observations, agronomic advice, or evidence of field efficacy.

A future model can supply calibrated flux generators, but each constitutive model must carry its own parameter provenance, applicability domain, uncertainty, and independent validation. Comparison with HYDRUS-1D, SWAP, or field data should happen only after units, boundary conditions, calibration splits, and model assumptions are matched.
