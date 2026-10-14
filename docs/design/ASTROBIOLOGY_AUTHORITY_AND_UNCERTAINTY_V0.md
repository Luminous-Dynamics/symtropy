# Astrobiology Authority and Uncertainty Contract v0.1

Status: design contract only. This document creates no runtime authority and makes no biological, habitability, biosignature, civilization, or life-detection claim by itself.

## Purpose

Symtropy may eventually simulate planetary environments, origins-of-life hypotheses, evolving biospheres, intelligence, cultures, technological civilizations, biosignatures, and technosignatures. The system must remain able to explore speculative possibility spaces without silently presenting speculation as empirical or validated science.

This contract defines the epistemic and authority boundary for that work before implementation.

## Core theorem

`simulated outcome != scientific prediction != empirical observation != validated model result`.

A simulated alien organism, biosphere, civilization, biosignature, or technosignature is meaningful only relative to the exact assumptions, model implementations, applicability domains, calibration/validation evidence, initial state, forcing history, and stochastic lineage that produced it.

No runtime layer may erase those distinctions for convenience or presentation.

## Composition rule

Astrobiology is a composition profile, not a new mega-authority.

The intended dependency direction is:

```text
stellar/orbital forcing
        -> planetary physical state
        -> chemistry / energy opportunity
        -> living-world ecology
        -> heredity / development / evolution
        -> cognition / social learning
        -> civilization authorities
        -> technology / planetary feedback
        -> observer-limited biosignature / technosignature evidence
```

Existing domain owners retain their authority:

- physical and material truth remains owned by the relevant physics/matter authorities;
- planetary geography and body-cell identity remain owned by `symtropy-world` contracts;
- ecological truth and representation/fidelity authority remain owned by Living World/LifeSim;
- persistent individual identity remains distinct from population/cohort representation;
- observation, belief, assertion, and institutional record remain separate epistemic states;
- civilization institutions, ownership, economy, communications, diplomacy, conflict, and other social authorities remain independent;
- rendering, narrative, UI, and authored flavor never become scientific or canonical authority.

Future `symtropy-planet-core` and `symtropy-evolution-core` should own only genuinely missing planetary-forcing and evolutionary semantics rather than duplicating the authorities above.

## Evidence classes

Every astrobiology-relevant model family that can materially affect a scientific or plausibility claim must declare one evidence class.

### E0 — Empirical Earth

Directly constrained by reproducible terrestrial observation or experiment within a declared domain.

Examples may include measured thermodynamic properties, terrestrial biological reaction ranges, or experimentally characterized population-genetic relationships.

E0 does not imply universal applicability beyond its calibrated domain.

### E1 — Empirical Planetary

Constrained by direct astronomical, planetary, or solar-system observations but not necessarily by biological observations.

Examples may include stellar spectra, measured planetary masses/radii, atmospheric composition constraints, or solar-system geochemical observations.

### E2 — Validated Model

A computational or mathematical model with a declared validation profile, comparison targets, error semantics, and applicability domain.

A model is not E2 merely because it is physically motivated or published. Symtropy must bind the exact implementation/model identity and the evidence used to validate the outputs that downstream authority consumes.

### E3 — Grounded Extrapolation

A model extrapolates established mechanisms outside directly validated conditions while preserving explicit physical/chemical/biological constraints.

Examples could include unfamiliar but chemically plausible metabolisms or morphology under non-Earth gravity when the relevant mechanics are modeled but empirical biological calibration is absent.

### E4 — Speculative Hypothesis

A scientifically discussable hypothesis lacking sufficient evidence or validation for predictive treatment.

Examples may include poorly constrained alternative biochemistries or origin-of-life mechanisms.

### E5 — Fictional Extension

A deliberately imaginative mechanism whose purpose is worldbuilding, gameplay, or exploratory thought rather than a current scientific claim.

The simulator may execute E5 models, but no downstream product may relabel their outputs as validated astrobiology.

## Evidence monotonicity

A downstream result inherits the weakest materially causal evidence class unless a separate qualification theorem justifies a stronger bounded claim.

For example:

```text
E2 climate model
+ E4 alternative biochemistry
+ E2 ecology
-> no stronger than E4 for the biological outcome
```

Rendering plausibility, narrative coherence, model agreement with itself, or large ensemble size cannot promote E4/E5 assumptions into E0-E2 evidence.

## Model claim record

A future runtime representation should be equivalent in semantics to:

```text
ModelClaim
- stable model-family identity
- exact model/version/implementation identity
- domain and process identity
- evidence class
- applicability domain
- calibration/validation profile reference when applicable
- observable/metric semantics when applicable
- uncertainty/error representation
- required forcing/state information
- provenance/evidence lineage
- supersession/revocation state
```

The exact Rust type and storage location remain implementation decisions.

A caller-authored `validated = true`, confidence scalar, or free-form citation string is not authority.

## Applicability domains

Every nontrivial model must state where it may be used.

Applicable dimensions may include:

- temperature and pressure range;
- solvent/phase regime;
- atmospheric or ocean composition;
- gravity;
- stellar spectral/radiation regime;
- spatial and temporal resolution;
- population size and structure;
- organism/developmental regime;
- ecological interaction class;
- evolutionary timescale;
- technological or social regime.

Extrapolation beyond a validated applicability domain must either fail closed for a validated-science profile or explicitly downgrade the evidence class for an exploratory profile.

## Assumption manifest

Every reproducible astrobiology experiment must bind a canonical assumption manifest containing all materially future-bearing assumptions, including as applicable:

- stellar and orbital forcing identities;
- planetary/interior/climate/geochemical model identities;
- chemistry/life-chemistry profile;
- origin-of-life hypothesis/profile;
- heredity, mutation, recombination, and developmental model versions;
- ecological process-set authority;
- selected representation/fidelity policies;
- closure qualification and validation-anchor identities;
- cognition/social-learning model versions;
- civilization integration profiles;
- observation/instrument models;
- canonical seed/lineage identity;
- source and target simulation instants;
- software/toolchain/execution capsule identities required by the evidence policy.

Changing a materially causal assumption creates a different experiment identity.

## Life chemistry profile

Symtropy must not hard-code `life == Earth animal` while also avoiding unsupported claims that arbitrary chemistry is equally plausible.

A future life-chemistry profile should distinguish at least:

```text
solvent / environmental medium
structural chemistry
energy coupling / metabolism family
information-carrier model
compartment / individuality model
catalysis model
environmental stability constraints
required gradients/resources
model evidence class
```

Carbon/water biology should be the first calibrated reference profile.

Alternative-solvent, alternative-information-carrier, atmospheric-life, plasma-like, or other exotic profiles remain explicitly lower-confidence until independently supported.

## Abiogenesis boundary

`habitable != inhabited`.

No generic habitability flag may automatically create life.

Origin-of-life modeling must be an explicit hypothesis-bearing process consuming declared chemical/environmental opportunity. Different abiogenesis models may produce different outcomes from the same planet and must retain independent evidence classes.

Until a mechanism is modeled, product lanes may seed life explicitly as an authored initial condition, but the resulting world must record that life was seeded rather than claiming simulated abiogenesis.

## Evolution boundary

Evolutionary authority should eventually distinguish:

- hereditary state;
- parentage and lineage;
- mutation/recombination semantics;
- demography and population structure;
- selection processes;
- gene flow/migration;
- drift;
- speciation/extinction;
- major transitions in individuality;
- genotype/development/phenotype relationships;
- ecological feedback and niche construction.

`PhenotypeSeed` or render/body-generation coordinates must never silently become genome, ancestry, or parentage authority.

Population genetics must be qualified first against analytical expectations and selected established reference simulators before scientific-use claims are made.

## Major transitions and individuality

Alien life must not assume one genome == one body == one evolutionary individual.

The architecture should permit, when corresponding biological models exist:

- unicellular individuals;
- multicellular organisms;
- colonial/modular organisms;
- obligate symbiotic composites;
- networked organisms;
- superorganisms;
- temporary reproductive collectives.

Transitions between these organizational levels are evolutionary events, not cosmetic body-generation choices.

## Ecology-evolution coupling

Evolution may alter ecological state and ecological state may alter evolution.

No generic scalar `fitness` should be treated as an unexplained world-truth field when fitness can instead be derived from explicit survival, reproduction, resource, competition, mutualism, predation, disease, environmental tolerance, or other qualified processes.

Coarse fitness closures are permitted only under the existing Living World information/closure/fidelity authority rules.

## Development and morphology

The preferred long-term biological pipeline is:

```text
heredity
 -> regulation/development
 -> morphology/physiology
 -> perception/behavior
 -> ecological consequence
 -> reproductive outcome
```

Final morphology should not be the sole mutable genome representation if a developmental model exists.

Physical morphology may be evaluated through higher-fidelity physics or biomechanics trials. Such trials produce measured phenotype evidence; they do not rewrite hereditary history.

## Cognition and culture

`cognition threshold != civilization`.

The route from intelligent behavior to civilization should remain causal and may require:

- memory;
- learning;
- social learning;
- communication;
- innovation;
- retained/transmitted knowledge;
- cumulative culture;
- coordination/institutionalization.

Cultural inheritance is distinct from genetic inheritance and may operate at different timescales and through non-parental transmission.

When a population becomes civilization-capable, integration should project it into the existing civilization authorities rather than construct a parallel `AlienCivilization` state object with global government/technology/aggression scalars.

## Anti-anthropocentrism rule

Alien-life generation must be tested for hidden Earth/human defaults.

Required adversarial experiment families should eventually include:

- gravity change without forced Earth body-plan retention;
- stellar-spectrum change with biologically relevant phototrophic/sensory response where modeled;
- all-ocean/no-dry-land worlds;
- environments without accessible combustion;
- unfamiliar dominant communication media;
- unusual reproductive/individuality models;
- multiple independently evolving intelligent lineages;
- cases where complex cognition does not yield technological civilization;
- cases where technology develops along materially different capability paths.

Failure of an output to change under a materially changed forcing variable is evidence of missing coupling, not evidence of universal convergence, unless an independent invariance theorem supports it.

## Observer boundary

`world truth != observer evidence`.

Life-detection and civilization-detection experiments must consume instrument-mediated observations rather than privileged canonical flags.

Potential observation products may include, when corresponding models are available:

- spectra;
- atmospheric composition and disequilibrium;
- surface/seasonal reflectance;
- temporal variation;
- chemical/isotopic patterns;
- radio/radar/laser emissions;
- artificial illumination;
- industrial atmospheric species;
- waste heat;
- orbital infrastructure;
- archaeological/fossil records.

An observation may become epistemic evidence. It does not directly become belief, public assertion, institutional record, or world truth.

False-positive and false-negative profiles must be explicit for any life/technology detector claiming calibrated inference.

## Biosignature and technosignature semantics

A biosignature/technosignature is an observation or derived evidence record under a specific instrument/model context, not a hidden boolean property of a world.

Every detection claim should bind:

- source observation(s);
- instrument model/version;
- observation geometry and time;
- noise/systematics profile;
- inference model and evidence class;
- alternative abiotic/non-technological hypotheses considered where applicable;
- confidence/error semantics;
- provenance lineage.

## Multiscale and deep-time rule

Long-timescale planetary/evolutionary history must reuse Symtropy's existing time, continuation, fidelity, closure, and inactive-world evolution contracts.

Do not create an independent astrobiology clock or camera-driven biological LOD.

Coarse evolution may become authoritative only through domain-approved representation/closure semantics with explicit evidence horizons, validation anchors, and conservation/information contracts where relevant.

When an accepted closure horizon expires, the system must revalidate/refine rather than silently extend approximate authority indefinitely.

## Ensemble semantics

One stochastic world history does not establish likelihood.

Astrobiology research profiles should support deterministic ensembles over explicit seed sets and report distributions for observables such as:

- life-establishment rate under a stated abiogenesis model;
- time to persistent biosphere;
- diversification/extinction patterns;
- frequency/timing of major evolutionary transitions;
- convergence/contingency metrics;
- emergence of complex cognition or cumulative culture under stated models;
- civilization persistence under stated social/technological models;
- biosignature and technosignature detectability windows.

The result is always conditional on the exact model/assumption manifest.

Ensemble count does not repair model misspecification.

## Counterfactual semantics

A counterfactual run is a fork from a declared canonical state/manifest with an explicit changed assumption or event.

It must preserve ancestry to the source world while receiving a distinct world/experiment identity.

Counterfactual differences may support statements of the form:

`under model M and assumptions A, changing X produced distributional effect Y`

They do not establish real-world causal truth without external validation.

## Scientific claim levels

Products and documentation should distinguish at least:

1. implemented/static;
2. executable-qualified;
3. numerically/reference-validated;
4. empirically calibrated;
5. independently reproduced/validated.

A visually compelling result is none of these by itself.

No README, UI, paper, or marketing surface should collapse these levels.

## Initial qualification program

### Q-ASTRO-0 — authority discipline

- evidence classes are typed and cannot be caller-upgraded;
- applicability-domain violation fails or downgrades according to explicit profile;
- changed model implementation/evidence status stales derived claim authority;
- assumption-manifest ordering is canonical;
- speculative inputs remain visible through derived outputs.

### Q-ASTRO-1 — terrestrial reference evolution

- analytical population-genetic fixtures;
- deterministic heredity/recombination fixtures;
- selected reference-simulator differential tests;
- save/reload and representation-transition equivalence within declared metrics.

### Q-ASTRO-2 — coupled eco-evolution

- ecology changes selection;
- evolved traits alter ecology;
- process-information authority promotes fidelity before a process consumes unavailable information;
- long-horizon coarse use requires valid closure evidence/anchors.

### Q-ASTRO-3 — planetary forcing

- planetary state responds to declared stellar/orbital/interior forcings;
- state/forcing identity is continuation-significant;
- no implicit Earth defaults are treated as extrasolar truth;
- coarse planetary closures are bounded by validation evidence.

### Q-ASTRO-4 — observer realism

- hidden truth is inaccessible to observation consumers;
- instrument limits/noise affect evidence;
- false-positive scenarios remain representable;
- belief/inference remains distinct from observations.

### Q-ASTRO-5 — anti-Earth-bias ensembles

- controlled changes in physical/environmental assumptions alter the relevant biological/civilizational distributions when the model predicts coupling;
- outputs do not collapse to fixed humanoid/Earth-history archetypes;
- convergence claims are statistical and model-conditional.

## Initial product milestone

Do not begin with a galaxy or a technological species.

After the existing terrestrial Living World Cell becomes sufficiently qualified, build one bounded `Alien Biosphere Cell` with:

- one explicit stellar/orbital forcing profile;
- one explicit planetary surface/atmosphere/hydrology profile;
- one carbon/water reference life chemistry;
- at least two independent evolutionary seed lineages;
- coupled producer/consumer/decomposer ecology;
- heredity and developmental persistence;
- at least one morphology whose performance can be evaluated at higher physical fidelity;
- observer-limited biosignature output;
- paired convergence/contingency ensemble experiments;
- no civilization requirement.

Civilization integration is a later milestone after biological/cultural transition semantics are qualified.

## Deliberate non-goals for v0.1

This contract does not implement:

- a climate solver;
- abiogenesis;
- genomes;
- evolution;
- alien morphology generation;
- alternative chemistry;
- cognition;
- civilization;
- biosignature inference;
- technosignature inference;
- SETI prediction;
- a Drake-equation prior;
- a universal habitability score;
- a universal intelligence or civilization scalar.

It creates only the authority/epistemic rules that future implementations must preserve.

## Success criterion

Symtropy succeeds when it can generate worlds that are both imaginative and inspectably causal while remaining explicit about what is measured, validated, extrapolated, speculative, or fictional.

The desired claim is not:

> Symtropy predicts alien life.

It is:

> Symtropy can execute reproducible, evidence-classified experiments over explicitly declared planetary, biological, evolutionary, cultural, and observational assumptions, with stronger claims admitted only when corresponding validation evidence exists.
