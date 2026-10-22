# MINE-REFERENCE-001 — benchmark-source registry v0.1

This initial commit is metadata-only. It does not ship mine data, parse MineLib instance files, or add another scheduling optimizer.

## Research references

- MineLib catalogue: https://mansci-web.uai.cl/minelib/Datasets.xhtml
- MineLib format description: https://mansci-web.uai.cl/minelib/minelib_format.pdf
- Lotsu, Bimpong & Boakye (2026), *An open-source optimization model for sustainable open-pit mine production scheduling*: https://doi.org/10.3389/frai.2026.1759758
- Separate research artifacts named by the paper: https://doi.org/10.5281/zenodo.19869809
- Jeuken & Forbes (2024), *The Value of Drilling*: https://doi.org/10.1007/s42461-024-01061-8
- Andrade & Dimitrakopoulos (2024), *Integrated Stochastic Underground Mine Planning with Long-Term Stockpiling*: https://doi.org/10.3390/min14020123

The 2026 scheduling study uses a 602-block, 12-period synthetic case and reports a public artifact record. Its metrics are not evidence of actual mine performance. A paper's license does not automatically grant rights for a separate dataset. Specific dataset rights stay unverified until separately checked.

**Published-result comparability caveat:** the paper's main synthetic schedule reports an NPV of about USD 674.83 million, while a separate environmental-penalty table presents normalized/subset scenarios around USD 3.99–4.11 million. The authors state those absolute figures are not directly comparable. Do not hardcode either headline as a universal known answer: align the exact input, valuation basis, discounting, constraints and solver settings before attempting reproduction.

## Invariants

- Every source has a unique ID, canonical URL, citation, upstream-version field, license status, redistribution flag, access state and review note.
- Unverified/restricted sources cannot assert redistribution permission.
- Metadata-only entries have no data-file hash. Downloaded files require a SHA-256 and explicit verified redistribution rights.
- Unknown JSON fields, duplicate IDs, missing citations, malformed URL/hash/date or inconsistent access and rights fields fail closed.
- The receipt binds registry bytes and exact source commit. No third-party mine dataset is included.
- Verified-open is registry metadata, not legal advice or an automated license opinion.

## Benchmarks

- 100% of entries have source identity and citation.
- 0 unverified-rights entries permitted to redistribute.
- 0 downloaded data admitted without exact file hash and verified rights.
- Later parser milestone: 100% agreement with hand-checked block counts, units, objective values, and precedence edges; zero silent coercions or dangling references.
- For tiny provable optimization fixtures, 100% independent feasibility agreement and 0% optimality gap. For larger instances, separately report best feasible objective, bound, gap and runtime.

This registry composes with MINE-SYSTEM-001 (#1933). It does not duplicate its evaluator, optimize a real operation, or authorize a physical activity.
