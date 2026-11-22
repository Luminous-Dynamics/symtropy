# Hilti construction-site benchmark and trajectory-support evaluation status

**Snapshot date:** 22 November 2026
**Dataset candidate PR:** [#6779](https://github.com/Luminous-Dynamics/symtropy/pull/6779)
**Trajectory-support evaluator PR:** [#6781](https://github.com/Luminous-Dynamics/symtropy/pull/6781)
**Research track:** [NON-GNSS-CONSTRUCTION-INTEGRITY-TRACK.md](NON-GNSS-CONSTRUCTION-INTEGRITY-TRACK.md)

## External benchmark reference

The 2026 Hilti–Trimble–Oxford challenge is a public construction-site 360° visual-inertial SLAM and floor-plan localization benchmark. Its official leaderboard reports 2,410.0/2,500 for the leading SLAM submission and 2,196.3/2,400 for the leading localization submission. A published runner-up SLAM team reports 2,393.9/2,500, 0.109 m mean RMSE and 99.85% mean trajectory coverage.

The challenge uses a 99% reference-pose coverage gate and an exponential position-accuracy score. It evaluates 25 SLAM and 24 floor-plan localization runs. These are accuracy/completeness metrics; the official score does not measure protection-level coverage, calibrated mission risk, false-safe exposure, time-to-alert or recovery correctness.

References:

- Official leaderboard: https://hilti-challenge.com/leaderboard-2026
- Challenge rules: https://hilti-challenge.com/
- SPARO second-place result: https://sparolab.github.io/research/hilti_challenge/
- Dataset paper: https://doi.org/10.1109/LRA.2026.3747660

## Rights and candidate status

The dataset and benchmark are under CC BY-NC-SA 3.0. It is permitted for properly attributed, share-alike non-commercial research; commercial use is prohibited without separate authorization. The registry candidate is explicitly marked `research_use=permitted`, `commercial_use=prohibited`, `admission_scope=none`.

The current registry includes seven candidates and admits zero. The Hilti payload has not been downloaded, assigned immutable Hugging Face revision/file SHA-256s, or admitted. Its camera/IMU timing, floor-plan registration and LiDAR-inertial reference dependency require independent review.

## Exact-head software receipts

| PR | Exact-head workflow | Tests | Result |
|---|---|---:|---|
| #6757 PL equation fidelity and trace hardening | #161 | 83 | passed |
| #6760 finite mission risk budget and binomial evidence | #165 | 92 | passed |
| #6775 mission risk contract | #168 | 103 | passed |
| #6777 horizon-free alpha spending | #167 | 98 | passed |
| #6779 Hilti dataset candidate/rights registry | #170 | 94 | passed |
| #6781 trajectory support provenance evaluator | #172 | 100 | passed |

Counts are totals across the validator, isolated navigation research crate and MILUV ingest crate for each exact head; binaries with zero tests are not included. Each row is software validation only and does not imply that physical data have been admitted or a route benchmark has been executed.

## Trajectory support evaluator synthetic demonstration

Workflow #172 passed on #6781 head `2551a143f94fa1f2926190cd421f169b3a340bc9`. Its synthetic trace had:

- output coverage: 99.0%;
- evidence-backed coverage: 95.0%;
- interpolated/extrapolated output: 4.0%;
- challenge-compatible exponential score: 95.7005;
- NORMAL without qualified support: zero in the demo because filled/unavailable states were explicitly degraded/uncertain/unavailable.

This is a post-matched synthetic evaluation. It does not process Hilti ROS bags, synchronize clocks, apply calibration, validate reference truth or reproduce the official evaluation server.

## Competitive target

Use the official scores 2,410.0/2,500 (SLAM) and 2,196.3/2,400 (localization) as matched accuracy comparators. A Luminous claim should first reproduce an accepted baseline on the official split, then seek a statistically supported improvement while preserving trajectory completeness and reporting the additional integrity/service dimensions. A higher score alone is not an integrity or safety result.