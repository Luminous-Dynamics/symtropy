# Strict CalculiX 2.22 .dat parser

This crate parses the output of NODE PRINT for the PHYS-CCX-MMS-P1-001 workflow.

**Important semantic distinction:** the forces (fx,fy,fz) section requested by RF contains external nodal forces (applied loads plus reactions where applicable); it is not universally a reaction-only block. CalculiX documents this distinction in its RF manual and the separate reaction-only RR output on versions that support it. This parser targets the pinned CCX 2.22 profile and intentionally does not substitute a reaction forces block for the required forces block.

References:
- https://www.feacluster.com/CalculiX/ccx_2.18/doc/ccx/node307.html
- https://calculix.discourse.group/t/output-reaction-forces-only/3809

The parser requires exactly one target-time displacement block and one target-time external-force block for the expected set, checks every node ID against the generated inventory, rejects duplicate/missing/unexpected nodes, malformed rows, non-finite fields, invalid times, and ambiguous matching blocks. It accepts either E or D exponents in numeric fields.

The included .dat fixture is SYNTHETIC, not output from a real CalculiX process. It tests parser semantics only. Pinned-CCX integration remains a qualification gate; no solver execution or numerical result is claimed by this crate.
