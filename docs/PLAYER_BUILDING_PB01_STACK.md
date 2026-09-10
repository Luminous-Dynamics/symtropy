# PB-01 stack relationship

PB-01 is stacked on PB-00 (`docs/player-building-authority-v0.1`) for review clarity.

```text
main
  -> PB-00 authority/freedom/home contract
      -> PB-01 proposal IR
          -> future PB-02 physical-authority adapter
```

PB-01 should be reviewed and qualified against its exact head, but it should not be retargeted directly onto an unrelated moving Fabrication/Construction hardening branch. PB-02 is the deliberate convergence point after the relevant physical-authority product heads have qualified.

This keeps three questions independently reviewable:

1. What should player building mean? — PB-00.
2. How do we represent player intent safely and deterministically? — PB-01.
3. How does an accepted proposal cross into singular physical reality? — PB-02.
