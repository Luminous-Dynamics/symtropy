---
title: Lore Tone Variance, Human Mess, and Local Specificity Benchmark
version: 0.1
status: implementation-spec
scope: blind tone testing, naming density, rhetorical repetition, concept purity, diegetic artifacts, popular culture, historical noise, private-life depth, and nonhuman everyday life
owner: narrative/lore/production/research
related:
  - ../lore/NAMING_PLURALITY_LOCAL_REGISTER_AND_LINGUISTIC_HISTORY_ATLAS_V0_1.md
  - ../lore/CORPORATE_ASYMMETRY_MERGERS_BRANDS_AND_INSTITUTIONAL_FAILURES_ATLAS_V0_1.md
  - ../lore/GROUND_LEVEL_DIEGETIC_ARTIFACTS_AND_ORDINARY_VOICES_ANTHOLOGY_V0_1.md
  - ../lore/HUMAN_IRRATIONALITY_STATUS_RIVALRY_REVENGE_AND_MISCALCULATION_ATLAS_V0_1.md
  - ../lore/POPULAR_BAD_CULTURE_SPORT_KITSCH_CELEBRITY_AND_TRASH_MEDIA_ATLAS_V0_1.md
  - ../lore/HISTORICAL_NOISE_FALSE_LESSONS_FORGOTTEN_EVENTS_AND_BUREAUCRATIC_ACCIDENTS_ATLAS_V0_1.md
  - ../lore/CULTURAL_CROSS_CONTAMINATION_MINORITIES_MIXED_HOUSEHOLDS_AND_ORDINARY_NONPARTICIPANTS_ATLAS_V0_1.md
  - ../lore/HISTORICAL_FIGURE_PRIVATE_LIVES_HABITS_DEBTS_RIVALRIES_AND_UNRELIABLE_LEGACY_ADDENDUM_V0_1.md
  - ../lore/NONHUMAN_EVERYDAY_LIFE_HUMOR_PLAY_STATUS_AND_DOMESTICITY_ATLAS_V0_1.md
  - fixtures/lore_tone_v0_1/tone_entry.schema.json
  - fixtures/lore_tone_v0_1/blind_sample_manifest.example.json
  - fixtures/lore_tone_v0_1/scoring_profile.json
---

# Lore Tone Variance, Human Mess, and Local Specificity Benchmark

## Purpose

This benchmark tests whether Symtropy's lore remains ethically coherent without sounding as though every inhabitant, corporation, alien, city, historian, and criminal network was written by the same morally lucid narrator.

The benchmark is not a demand for cynicism, vulgarity, or arbitrary contradiction. It protects a more difficult target:

```text
shared constitutional boundaries
different local voices
causal history
uneven institutions
ordinary life
private limits
popular culture
irrational motives
historical error
nonhuman activity that does not revolve around humanity
```

> **The benchmark fails when proper nouns create most of the difference and the prose beneath them remains one polished template.**

## Claims under test

### Claim A — Local specificity survives anonymization

After removing proper nouns, a reader should still distinguish places through:

- physical conditions;
- work;
- material culture;
- food;
- slang structure;
- social institutions;
- ordinary conflicts;
- sensory and spatial detail.

### Claim B — Moral-template classification remains difficult

A reader should not be able to predict every entry's conflict structure from its first paragraph.

The corpus should not always follow:

```text
essential service
hidden coercion
hardliner
reformer
excluded population
public hearing
bounded compromise
aphoristic conclusion
```

### Claim C — Seriousness coexists with low culture

A region with memorial law and infrastructure politics should also contain entertainment, sport, bad taste, jokes, domestic frustration, and people uninterested in the central thesis.

### Claim D — History contains noise

Events should include uncertainty, unrelated consequences, false lessons, administrative residue, forgotten intervals, and regional ignorance.

### Claim E — Nonhuman depth exceeds contact philosophy

Nonhuman entries should contain maintenance, play-like activity, status, error, private boundaries, care, internal disagreement, and preferences not organized around human contact.

## Benchmark corpus

The full benchmark uses at least **60 excerpts** drawn from no fewer than twelve source families:

```text
corporate civilization
legendary place
regional campaign
diaspora
shadow network
historical event
historical figure
art or popular culture
diegetic artifact
ordinary institution
nonhuman everyday life
archive interpretation
```

Minimum source diversity:

- eight regions;
- six corporate or successor lineages;
- six mobile or stateless communities;
- six historical figures;
- four nonhuman agency structures;
- six different authoring templates or document families.

No more than two excerpts may come from the same named entry.

## Sample construction

Each source excerpt produces three views.

### View 1 — Full context

Retains title, place, names, and metadata.

Purpose: verify legibility and intentional identity.

### View 2 — Proper-noun masked

Replaces names, unique slogans, and explicit location labels with neutral tokens while preserving material and social detail.

Example:

```text
[SETTLEMENT] occupies three failed coastal-defense layers.
A furniture district on the second barrier employs thousands and appears in no tourism imagery.
```

Purpose: test whether identity survives beneath naming.

### View 3 — Rhetorical skeleton

Removes most nouns and replaces domain terms with category labels.

Example:

```text
[INSTITUTION] provides [SERVICE].
Its competence creates [DEPENDENCY].
A [REFORM GROUP] contests [HARDLINE GROUP].
```

Purpose: expose repeated templates.

The rhetorical skeleton is for diagnostic use, not player-facing content.

## Reviewer panels

Use at least three panels.

### Internal narrative panel

Understands Symtropy's design goals but does not receive source labels.

### Cross-discipline panel

Includes design, engineering, art, audio, localization, accessibility, and QA readers.

### External or cold-reader panel

Has no detailed knowledge of the setting.

A small panel is acceptable during early development, but results must report panel composition.

## Test 1 — Place recognition after proper-noun masking

Reviewers receive masked excerpts and choose among four candidate regions plus “insufficient information.”

### Target

```text
median correct region family: >= 60%
insufficient-information use: 10%–35%
confidence calibration: no worse than 0.20 absolute error
```

The goal is not perfect identification. Perfect scores may indicate stereotype or repeated exposition.

### Failure interpretation

Low recognition suggests names carry too much identity.

Very high recognition based on one repeated slogan suggests concept purity rather than depth.

## Test 2 — Template clustering

Reviewers group rhetorical-skeleton excerpts by perceived authoring pattern.

An automated pass may also compare:

- heading sequence;
- sentence-length distribution;
- repeated phrases;
- paragraph role;
- aphorism location;
- moral-resolution vocabulary;
- faction-count symmetry.

### Target

No single cluster should contain more than 35% of all entries unless the entries intentionally share one in-world institution or medium.

### Hard failure

More than half of major entries follow the same sequence of:

```text
virtue
hidden cost
balanced internal factions
scandal
Null endpoint
three successor outcomes
```

## Test 3 — Naming saturation

Count lexical families in titles, headings, and reference names.

Watch list includes, but is not limited to:

```text
quiet
lantern
archive
continuity
commons
meridian
witness
threshold
choir
orchard
glass
mercy
white
black
red
```

### Target

- no watch-list word appears in more than 4% of active named entities unless causally justified;
- at least 70% of flagship entities include two or more socially grounded alternate names;
- at least 35% include one mundane administrative or numbered name;
- poetic names identify a naming source in at least 75% of cases.

These are review thresholds, not immutable language law.

## Test 4 — Aphorism and direct-moral density

Identify sentences that function primarily as polished governing maxims.

### Target

For lore-facing atlas and campaign entries:

```text
<= 1 direct aphorism per 1,200 words
<= 20% of entries ending with an aphoristic summary
<= 25% of entries explicitly stating the intended moral lesson
```

Canonical contracts and internal standards are excluded because direct rules belong there.

### Hard failure

A lore entry explains the lesson before showing physical or social evidence.

## Test 5 — Concept-purity audit

For every flagship society, list institutions and inhabitants in four groups:

```text
signature-theme participants
practical participants
critics or reactionaries
ordinary nonparticipants
```

### Target

- at least three unrelated institutions per flagship settlement;
- at least one unrelated institution becomes causally important in a campaign;
- no more than 60% of named local NPCs primarily represent the signature civic thesis;
- at least one mixed household or cross-cultural institution per major region;
- at least one resident who cannot or will not explain the central philosophy.

## Test 6 — Human-motive coverage

Across the benchmark corpus, mark grounded motives:

```text
status
revenge
romantic attachment
family rivalry
faith
territorial pride
honor
boredom
fashion
aesthetic taste
miscalculation
incompetence
cruelty
kindness without competence
```

### Target

At least eight motive families appear in every regional campaign pack across major and minor actors.

No single motive family may explain more than 25% of consequential nonstructural decisions.

### Hard failure

Every conflict resolves when better evidence reaches a public hearing.

## Test 7 — Popular-culture coverage

Each flagship region should expose at least five of:

```text
sport
children's media
cheap food or chain
celebrity
bad drama
mascot
fashion
vulgar comedy
conspiracy entertainment
tourist kitsch
fan community
low-prestige hobby
```

At least one popular form must be loved without receiving designer approval.

At least one serious institution must borrow language or participation from popular culture.

## Test 8 — Diegetic artifact diversity

For each representative campaign, collect at least ten artifacts across different functions:

- menu;
- bill or notice;
- school exercise;
- joke;
- graffiti;
- obituary;
- advertisement;
- repair log;
- household message;
- low-prestige historical media.

### Targets

- at least one boring artifact;
- at least one inaccurate artifact;
- at least one artifact with affection;
- at least one artifact unrelated to the main campaign;
- at least four distinct social positions;
- no artifact exists solely to summarize backstory.

## Test 9 — Historical noise

Each major historical event should include at least three of:

```text
disputed chronology
misleading common name
false lesson
regional ignorance
administrative residue
quiet consequence
later political appropriation
inaccurate memorial
unrelated secondary consequence
source gap
```

### Hard failure

Every historical event produces exactly the reform implied by its premise.

## Test 10 — Historical figure privacy and ordinary life

For each major figure, reviewers check:

- one ordinary habit;
- one relationship not reducible to public legacy;
- one mistake or limitation;
- one private boundary;
- one erased collaborator or rival;
- one detail that does not symbolize the figure's historical role.

### Hard failure

A survivor, child, patient, or reconstituted person remains permanently available as public symbolism after requesting privacy.

## Test 11 — Corporate asymmetry

Across named corporate civilizations, verify variation in:

- founding pattern;
- coherence;
- competence;
- consumer culture;
- reform capacity;
- ownership;
- legal survival;
- public affection;
- failure mode;
- successor pattern.

### Hard failure

All corporations are essential utilities that become coercive through the same four internal factions.

## Test 12 — Nonhuman ordinary life

For each major nonhuman agency profile, test for:

```text
maintenance
play-like activity
internal disagreement
status or preference
care
error or obsolete habit
protected space
behavior not caused by humans
```

### Hard failure

The nonhuman entity appears only as a threat, treaty partner, metaphysical puzzle, or source of technology.

## Automated indicators

Automated analysis may flag, but not decide:

- repeated headings;
- repeated closing syntax;
- noun-family concentration;
- identical faction counts;
- repeated “X without Y” constructions;
- repeated moral vocabulary;
- proper noun density;
- average number of alternate names;
- artifact-type diversity;
- percentage of entries containing ordinary institutions;
- percentage of entries ending in a slogan or aphorism.

Human review remains necessary because lexical variety can hide conceptual sameness.

## Scoring

Each entry receives scores from 0–4.

| Dimension | 0 | 2 | 4 |
|---|---|---|---|
| Material specificity | generic | some grounded detail | identity survives names |
| Voice variance | same house voice | mixed | medium and speaker clearly shape prose |
| Institutional asymmetry | clean template | partial mess | uneven history and residue |
| Ordinary life | absent | decorative | causally present |
| Human motive range | purely structural | one private motive | several grounded motives |
| Historical noise | efficient myth | some uncertainty | errors, false lessons, residue |
| Popular culture | absent | one reference | lived network of low/high culture |
| Privacy and boundaries | symbolic person | stated boundary | boundary changes content access |
| Nonhuman everyday depth | contact-only | one ordinary behavior | autonomous social/ecological life |
| Worldline durability | cosmetic | some change | variation preserves causal ancestry |

### Promotion threshold

A flagship region requires:

```text
no hard failure
overall mean >= 2.8
no dimension below 2.0
material specificity >= 3.0
ordinary life >= 3.0
voice variance >= 2.5
```

The threshold should tighten only after real authored content exists.

## Benchmark procedure

1. Freeze source versions and hashes.
2. Select excerpts through declared sampling rules.
3. Produce full, masked, and skeleton views.
4. Randomize order.
5. Run reviewer panels independently.
6. Record confidence and free-text rationale.
7. Run automated indicators.
8. Compare disagreements rather than averaging them away.
9. Classify failures by source, template, or whole-corpus pattern.
10. Revise a bounded subset.
11. Repeat with held-out excerpts.
12. Publish the evidence bundle and limitations.

## Evidence bundle

```text
benchmark_manifest.json
source_hashes.json
masked_excerpts/
rhetorical_skeletons/
reviewer_protocol.md
anonymized_scores.csv
free_text_findings.md
automated_style_report.json
failure_registry.json
revision_diff/
final_summary.md
```

Reviewer identities and sensitive demographic details should not be published by default.

## Kill and cut criteria

Remove or rewrite a lore layer if:

- it increases proper nouns without increasing material distinctiveness;
- alternate names are mechanically complete but never used by characters;
- popular culture exists only as satire;
- private-life details become voyeurism;
- irrational motives become random action;
- historical noise destroys causal traceability;
- nonhuman everyday life becomes humans in unusual bodies;
- every entry remains recognizable as one template after two revision rounds.

## Representative proof

The first full benchmark should use **Nine Pumps and its connected region**, because the v1.8 playable-history proof already requires ordinary life, corporate residue, diaspora routes, informal services, archives, absence, and worldline variation.

The proof should include:

- Lower Basin Water and Power Cooperative 9;
- an Aureline successor branch;
- a retiree property association;
- workshop racers;
- one migrant household;
- one conservative or nonparticipating institution;
- local sport and cheap food;
- ten diegetic artifacts;
- two conflicting historical accounts;
- at least one badly remembered event;
- one nonhuman or ecological behavior unrelated to the campaign.

## Closing principle

> **Symtropy's values should be visible in the boundaries the game refuses to violate. Its inhabitants should remain free to be funny, vain, wrong, unfashionable, devout, cruel, ordinary, and difficult to summarize.**
