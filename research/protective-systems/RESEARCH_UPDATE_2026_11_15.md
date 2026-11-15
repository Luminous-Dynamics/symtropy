# Protective Systems Research Update — 2026-11-15

**Programme:** PROTECTIVE-SYSTEMS-001 (#5246)  
**Related owners:** PROTECT-DAMAGE-OBSERVABILITY-001 (#5267), PROTECT-VOI-001 (#5268)  
**Status:** source-verified research and benchmark-design update; no new model run, dataset ingestion, physical experiment or certification claim

## 1. New vehicle benchmark crosswalk: Euro NCAP 2026–2028

Added machine-readable crosswalk and refreshed standards snapshot:

- `standards/EURO_NCAP_2026_2028_CROSSWALK_2026_11_15.json`
- `standards/standards-register-snapshot-2026-11-15.json` (18 entries; supersedes the 14 November snapshot for current research use, while preserving the older snapshot as an immutable historical record)

Euro NCAP's 2026–2028 rating framework separates four stages:
1. Safe Driving
2. Crash Avoidance
3. Crash Protection
4. Post-Crash Safety

Each stage is scored out of 100. The published stage weights (20%, 20%, 50%, 10%) are for Best-in-Class ranking; the overall star rating depends on per-stage balance thresholds. The 2026 five-star minimums are 60/70/80/80, rising to 70/80/80/80 for 2027 and 80/80/80/80 in 2028. A weighted mean must not be substituted for these stage gates.

The 2026–2028 scheme also has explicit backstops, including red-body-region limits and a minimum VRU-impact score before points can be awarded for related pedestrian/cyclist crash-avoidance scenarios. The rating review allows at most five points of compensation between adjacent stages among the first three; Safe Driving cannot compensate Crash Protection directly.

This adds a key comparison principle: **keep stage floors, backstops and exact protocol versions visible**. Do not let an aggregate score conceal a weak crash-protection or post-crash result.

Sources:
- Euro NCAP Rating Review 2023, Rating Plan 2026–2028: https://www.euroncap.com/media/79890/euro-ncap-rating-review-2023.pdf
- Euro NCAP overall assessment and general 2026 protocols: https://www.euroncap.com/general-2026-onwards/
- Euro NCAP current 2026–2028 protocol index: https://www.euroncap.com/protocols/
- Euro NCAP post-crash rescue/extrication protocol v1.1 (September 2025; implementation January 2026): https://www.euroncap.com/media/91774/euro-ncap-protocol-post-crash-v11.pdf
- Euro NCAP description of 2026 changes: https://www.euroncap.com/what-s-new-from-2026/

## 2. SHM baseline examined at full-protocol level

The 5 November 2026 paper by Gao et al. reports three-day-training mean accuracy of LDA 84.2%, CNN 82.6%, SVM 82.5%, Random Forest 80.5% and MLP 78.0% on adjacent damage-severity classification (DOI: https://doi.org/10.32604/sdhm.2026.088197).

The paper's own limitations are material:
- one propagation path (6-2), one aluminium plate and one damage location;
- 13 adjacent binary tasks, not a general multi-class or open-set detector;
- roughly ten-day windows around individual damage transitions, not full seasonal or multi-year test generalization;
- one fixed seed (42), no repeated-seed significance testing;
- hyperparameters fixed, not tuned via a common search budget;
- only five classifiers included.

A subtle leakage issue is especially important. The fair-weather filtering autoencoder was fitted using the full five-day segment, which includes the later test windows, although the test examples themselves were not filtered. The authors identify this test-window exposure as a limitation. The benchmark must therefore separate:
1. faithful paper reproduction, labelled as using the published transductive filtering exposure;
2. a strict variant in which every learned preprocessing/filtering stage is fit on training windows only, then frozen and applied to test windows.

Those results must not be conflated or compared as if they had equal information access.

The article's 84.2% result is a useful, recent baseline for this narrow task—not a universal SHM SOTA, deployed alarm accuracy, false-alarm rate, cross-asset result or structural-integrity determination.

Sources:
- Full paper and abstract: https://www.techscience.com/sdhm/online/detail/28956
- DOI: https://doi.org/10.32604/sdhm.2026.088197
- Long-term dataset article: https://doi.org/10.1038/s41597-025-05300-5
- Loading code: https://github.com/SmartDATA-Lab/Long_Term_Guided_Waves

## 3. Dataset rights and version handling

OpenAIRE's DataCite-linked metadata view labels the guided-wave dataset CC BY and surfaces both the base DOI and a version-specific DOI (.v1). This is a useful licence signal, but the primary Figshare record/API was not accessible for direct inspection during this session. Before any data download or retention:
- confirm the exact primary Figshare record and version;
- pin its applicable licence and terms;
- pin file names, sizes and content hashes;
- distinguish the base DOI (which can resolve to the latest public version) from a version-specific DOI.

No measurement files were downloaded or redistributed in this update. The adapter remains metadata-first and bounded-streaming. It must not eagerly load the complete corpus, and it must not deserialize untrusted pickle files in a privileged or network-connected process.

Sources:
- OpenAIRE/DataCite-linked metadata: https://oamonitor.ireland.openaire.eu/national/search/dataset?pid=10.6084/m9.figshare.28112504
- Figshare versioning: https://info.figshare.com/user-guide/how-versioning-works/

## 4. Consequences for the research strategy

### A. From one leaderboard number to an evidence budget

Every method comparison must account for:
- labels and training data available at decision time;
- preprocessing fit scope and any test-time exposure;
- number of model/threshold/hyperparameter trials;
- compute, wall time, sensor/inspection budget and confirmation cost;
- failed runs, retries and human review;
- held-out data access and the point at which the test set is unsealed.

This is the only fair way to compare rule systems, classical models, deep models, Symthaea and human experts.

### B. From window accuracy to operational decisions

For SHM, keep classification quality separate from:
- false alarms per asset-day and path-day;
- time-to-detection and detection delay;
- missed-damage intervals;
- calibration and risk-versus-coverage;
- inspection workload and downtime;
- performance by environmental, path and damage-severity stratum.

A model that scores well on adjacent known states but cannot recognize an unseen state or a sensor fault is not an operational monitoring solution.

### C. From a vehicle aggregate to explicit weakest-stage gates

The Euro NCAP example shows that mature industry ratings already use stage-level thresholds and backstops. Luminous should not claim novelty for multi-stage vehicle safety. Its potential advantage is preserving the full configuration, protocol version, evidence provenance, uncertainty and lifecycle consequences across design/repair changes—then benchmarking better decisions under matched boundaries.

### D. From generic resilience to locally observed service

Keep housing outcomes as distinct trajectories: safe resident-hours, heat/hot-night recovery, ventilation/IAQ, water and power continuity, moisture, egress/accessibility, local repair delay and distribution of burden. Do not let a composite score trade away a hard floor. Synthetic, model, assembly-test, monitored-dwelling and resident-reported evidence remain separate.

## 5. Research baselines and falsification

The 2025 *Nature Computational Science* MFBO work tests how the informativeness and cost of approximate sources affect the value of multi-fidelity search, and compares MFBO with single-fidelity alternatives. The 2026 *Digital Discovery* perspective warns that active-learning choices can induce hidden biases and can underperform random sampling when the surrogate's inductive assumptions are poorly matched.

The benchmark should therefore include deceptive low-fidelity proxies, ranking reversals, cost uncertainty, failed experiments, random/stratified sampling, DOE, evolutionary multi-objective search, single- and multi-fidelity BO, and ablations. Publish cases where Symthaea loses.

Sources:
- Sabanza-Gil et al. (2025): https://doi.org/10.1038/s43588-025-00822-9
- *A critical examination of active learning workflows in materials science* (2026): https://doi.org/10.1039/d6dd00081a

## 6. Immediate runner-independent execution

1. Extend the SHM adapter to pin source-paper training/test boundaries, the faithful versus training-only autoencoder variants, and a temporal/path holdout manifest.
2. Add a machine-readable Euro NCAP crosswalk consumer; preserve stage thresholds, backstops and protocol versions.
3. Keep the B0 verifier and VOI/damage oracles isolated; add more metamorphic adversaries before broadening their authority.
4. Prepare a local metadata manifest for the guided-wave dataset; do not ingest until primary-version rights and hashes are confirmed.
5. Reproduce the public classifier baseline before proposing a competing architecture. Add multi-path fusion, open-set abstention and domain shift only as separately scored extensions.
6. Select a benign physical demonstrator only after a competent engineer reviews the specimen, inspection method, measurement uncertainty and safety controls.

## Claim ceiling

This update reports literature review, protocol crosswalks and software benchmark design. It does not report Rust compilation, CI success, dataset ingestion, model training, simulation, laboratory testing, physical damage-detection capability, vehicle safety improvement, housing outcomes, certification or SOTA superiority.


## 7. Risk-sensitive co-design under nonstationarity — 15 November 2026

A further research gap is now tracked in [PROTECT-ROBUST-001 #5317](https://github.com/Luminous-Dynamics/symtropy/issues/5317): whether protective designs remain feasible when hazard distributions, material properties, operating contexts and infrastructure dependencies shift. This is distinct from simply optimizing an expected value on a fixed scenario distribution.

The benchmark separates aleatory variability, epistemic parameter uncertainty, model-form uncertainty, distribution shift and genuinely unknown/unbounded conditions. Unknown is not a distribution and must not be encoded as zero variance. Risk metrics are reported separately: worst-case loss, CVaR, distributionally robust bounds, hard-floor violations and recovery trajectories. None is a universal safety score.

The research register now includes a small executable exact-oracle prototype:
- [Finite ambiguity-set fixture](https://github.com/Luminous-Dynamics/symtropy/blob/research/protective-systems-benchmark-v0-1/research/protective-systems/robust/fixtures/finite-ambiguity-v0.1.json)
- [Oracle specification and hand-checkable results](https://github.com/Luminous-Dynamics/symtropy/blob/research/protective-systems-benchmark-v0-1/research/protective-systems/robust/FINITE_AMBIGUITY_ORACLE_V0_1.md)
- [Rust implementation](https://github.com/Luminous-Dynamics/symtropy/blob/research/protective-systems-benchmark-v0-1/research/protective-systems/verifier/src/robust.rs)

It compares a nominal optimum, robust design and cheap fragile design across nominal, compound-hazard and shifted regimes. The fragile design is excluded by a hard floor, the robust design wins the finite ambiguity-set objective, an incomplete uncertainty set returns ABSTAIN, and a case with no feasible candidate returns NO_ROBUST_FEASIBLE_DESIGN.

Relevant published baselines include variance-constrained robust Bayesian optimization (Zhou et al., *Reliability Engineering & System Safety*, October 2026, DOI https://doi.org/10.1016/j.ress.2026.112359), multi-stage adaptive robust multi-objective decision-making under deep uncertainty (Shavazipour & Stewart, *Omega*, January 2026, DOI https://doi.org/10.1016/j.omega.2025.103405), and a 1 November 2026 study combining BIM, EnergyPlus, NSGA-II and AI for outage-resilient envelope optimization across 14 climate zones and four housing typologies, with 16,856 simulations (DOI https://doi.org/10.1016/j.scs.2026.107977). These are named comparators to reproduce, not results we claim to have surpassed.

No robust-design physical result, model run, or superiority claim is made by this exact finite oracle.


## 8. Add NASA's 2026 materials BO benchmark as a compulsory B2 comparator

A high-value industry/agency baseline is NASA Technical Memorandum NASA/TM-20260001102, *Benchmarking Bayesian Optimization Frameworks and Acquisition Strategies for Materials Discovery and Autonomous Laboratories*, published 1 March 2026 (DOI: https://doi.org/10.64631/ZKVT6687; official NTRS record: https://ntrs.nasa.gov/citations/20260001102).

The NASA study compares five surrogate/framework combinations across three benchmark problems: discrete solid-electrolyte composition, hybrid categorical/continuous laminate-composite design coupled to micromechanics, and the Ishigami analytic function. It pairs posterior mean, probability of improvement and expected improvement acquisitions, uses 100 trials from randomized initial samples, includes uniform random search as a control, and reports five random seeds per setting. Its abstract reports BayBE Gaussian Process + Expected Improvement reaching at least 95% of the known optimum in the fewest evaluations overall, while Lolopy random forests matched or exceeded GP in some categorical/mixed spaces at higher computational cost. The paper also notes posterior-mean stagnation on some landscapes.

**Implication:** Symthaea's protective-material discovery track should not compare only against NSGA-II and a generic BO baseline. B2 must include this exact NASA baseline family (Ax GP, BayBE GP/Monte-Carlo NN, Lolopy RF, Hyperopt TPE), the three matching problem classes where code/data permit, uniform random control, and the same 100-evaluation / five-seed discipline. Add hard constraints, multi-objective feasibility, cost uncertainty, held-out task families and a separate confirmation budget as extensions, not as changes to the source benchmark reproduction.

The benchmark result is not a claim that Bayesian optimization is uniformly best or that Symthaea should copy one acquisition function. It establishes a demanding reproducible starting point. We should test whether uncertainty-aware, hypothesis-discriminating, robust-feasibility-aware policies reduce **cost to an independently confirmed feasible design** under matched budgets, including categorical material/process variables. Do not compare against the NASA paper by quoting only its abstract; reproduce its code, problem definitions, seed policy and evaluation budget first.

A 2026 batch-BO study also shows that performance depends strongly on noise level and landscape shape, including needle-in-a-haystack versus smooth functions; therefore add controlled measurement noise, sparse feasible regions and predeclared batch acquisition baselines rather than evaluating only smooth noiseless objectives (Mia et al., *Journal of Materials Research*, 26 February 2026, DOI: https://doi.org/10.1557/s43578-026-01803-y).

## 9. Executed software qualification and source corrections — 15 November 2026

The latest exact-head CI run for the draft branch completed successfully on Ubuntu, Windows and macOS. The Linux-only isolated Protective Systems step passed:
- Rust formatting check;
- Clippy with warnings denied;
- 40 unit tests across B0, damage/repair lifecycle, VOI and robust-design oracles;
- B0 CLI (10/10 fixture dispositions; deterministic input receipt);
- VOI CLI (anti-correlated proxy selected; explicit no-experiment baseline);
- robust-design CLI (ROBUST_DESIGN selected; incomplete uncertainty abstention and non-compensatory hard floors).

The standard repository workflow formatting, Clippy, physics tests and workspace build also passed on all three operating systems. The isolated verifier step intentionally runs on Ubuntu only. This qualifies the exact synthetic software tranche; it does not qualify any material, vehicle, PPE or building physically.

The SHM adapter was corrected after inspecting the source repository README and loader. The paper's apparent 8.6-second versus ~176-second cadence difference is explained by taking one in twenty original acquisitions for the public benchmark; it is not an unexplained paper-internal conflict. The current SmartDATA Lab GitHub README also says the original Figshare download is no longer available and points to a Dropbox folder. The Dropbox payload's version/licence/hash equivalence to the cited DOI remains unverified, so no dataset files were downloaded or ingested. The public landing page's HDF5/14-sensor/30-second description remains separate from the paper/repository's eight-path monthly-pickle description until the data owner confirms version mapping.

**Claim ceiling remains unchanged:** no SHM model has been trained, no physical experiment has been run, no performance improvement over the industry or SOTA has been demonstrated, and no protection certification or safety claim is made.
