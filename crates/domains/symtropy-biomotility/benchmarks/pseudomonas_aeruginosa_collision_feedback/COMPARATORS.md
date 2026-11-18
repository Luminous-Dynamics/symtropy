# Comparators — P. aeruginosa collision mechanosensory feedback

## Scientific anchor

Le Blanc et al., “Bacteria regulate collective behaviour by mechanosensing cell–cell collisions,” *Nature Microbiology* (published 6 October 2026), DOI 10.1038/s41564-026-02505-1.

The study reports that *P. aeruginosa* surface twitching cells can reverse after type-IV-pilus-mediated contact with another cell. A characteristic collision-response timescale is defined such that a perpendicular contact maintained for that time gives 50% reversal probability; the probability decreases linearly to zero as contact angle decreases from pi/2 to pi/6.

The paper reports a density-dependent increase in wild-type reversal frequency while per-contact reversal probability remains largely constant. In this benchmark, contact and reversal incidence are normalized by total tracked cell-time (cell-seconds), not wall-clock recording duration, so population-size differences do not masquerade as increased per-cell rates. A non-reversing Delta-pilH control becomes increasingly clustered and nematically ordered at high density; WT and Delta-pilG remain comparatively disordered, with Delta-pilG behaving as a constitutive reverser.

The exact biological parameter values and the raw source dataset are not admitted by this benchmark yet.

## Paradigm shift

Density must not directly set the per-contact reversal probability. It changes how often cells encounter one another through geometry, packing, shape, and kinematics. The sensory response law acts on each contact; population-level reversal frequency emerges from contact incidence multiplied by the per-contact response.

Keep these components separate:
1. contact-generation mechanics;
2. collision angle and contact-duration observation;
3. phenotype-specific sensory response law;
4. contact-independent constitutive reversals;
5. morphology and steric alignment;
6. collective order, spreading and maze escape.

## Matched benchmark arms

| Arm | Contact mechanics | Reversal response | Purpose |
|---|---|---|---|
| A | Repulsive rod contacts | No collision reversal | Classical self-propelled-rod baseline |
| B | Same contact schedule | Fixed per-contact reversal probability independent of contact geometry | Simple stochastic control |
| C | Same contact schedule | Angle-duration collision response | Mechanosensory feedback candidate |
| D | Same contact schedule | Non-reversing phenotype | Delta-pilH-like negative control |
| E | Same contact schedule | Constitutive Poisson reversals independent of contact | Delta-pilG-like control |
| F | Same contact schedule and varied aspect ratio | Contact-sensitive response held fixed | Separate morphology from sensory feedback |

All arms must share initial configurations, cell dimensions, surface coverage, contact traces, observation horizon, seeds and compute budget when comparing response laws. A separate contest is required when contact mechanics itself changes.

## Primary outcomes

- Total detected contact count and evidence-qualified contact count per total tracked cell-time (cell-seconds).
- Contact-angle and contact-duration distributions.
- Per-contact reversal fraction, stratified by evidence-qualified response labels.
- Evidence-qualified reversals per total tracked cell-time (cell-seconds).
- Global polar and nematic order parameters.
- Local nematic correlation and correlation length when positions and neighbour identity are available.
- Voronoi-cell area skewness and cluster statistics when complete spatial snapshots are available.
- Colony-edge spreading and micromaze escape only after a population and boundary mechanics model exists.
- Held-out calibration by density, aspect ratio, geometry motif and independent replicate.

## Acceptance fixtures

1. At contact angle pi/2 and duration tau_CR, the declared exponential duration response returns p=0.5 exactly; at any fixed positive duration, probability scales linearly with the angular gate from pi/6 to pi/2.
2. At or below pi/6, the response probability is zero.
3. Probability increases monotonically with contact duration.
4. A fixed per-contact response law and higher contact incidence increase the population reversal rate without changing the per-contact probability.
5. The non-reversing phenotype produces zero contact-triggered reversals.
6. Constitutive reversal times replay from a seed and are independent of contact schedules.
7. Nematic order is invariant to reversing a rod axis by pi; polar order remains direction-sensitive.
8. Empty/unknown heading sets produce undefined metrics, not zero.
9. Evidence-inferred or unresolved contact labels do not enter ground-truth response calibration.

## Evidence / rights / split rules

Source data and simulation code are listed at Zenodo DOIs 10.5281/zenodo.20430747 and 10.5281/zenodo.22231353. Their exact versions, file lists, licences, checksums and metadata must be verified before ingestion. The article's licence does not automatically determine the separately deposited data/code terms.

Split by independent experimental replicate, density condition, geometry motif or time-lapse recording as appropriate. Never split adjacent tracks/frames from the same replicate across train and confirmatory test. Report the number of independent replicates separately from cells, tracks and fields of view.

## Claim ceiling

This benchmark presently qualifies a synthetic event-response law and its controls. It does not generate physical contacts, simulate rod mechanics, model type-IV pilus force transduction, establish biological causality by itself, reproduce the paper, predict virulence, or establish antimicrobial efficacy.