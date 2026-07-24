# Symtropy Documentation Patch Series v0.7

Base: `Symtropy_Docs_Integrated_2026-07-20_v0_6`

Apply the numbered patches in order with:

```sh
git apply --check patches/v0_7/0001-playable-activity-and-revisitability.patch
git apply patches/v0_7/0001-playable-activity-and-revisitability.patch
```

Repeat for `0002` through `0007`.

Alternatively apply the aggregate patch once:

```sh
git apply --check patches/v0_7/SYMTROPY_DOCS_V0_7_AGGREGATE.patch
git apply patches/v0_7/SYMTROPY_DOCS_V0_7_AGGREGATE.patch
```

## Series

1. **Playable activity and revisitability**  
   Adds the mission/event grammar and makes consequences visibly persist after objectives.

2. **Science, research, and discovery**  
   Replaces generic research points with observations, hypotheses, experiments, replication, uncertainty, and knowledge institutions.

3. **Player authorship and long-horizon play**  
   Defines sandbox modes, blueprint provenance, mod manifests, creator tooling, Great Works, mature worldlines, forks, migration, and non-terminal victory.

4. **Multiplayer social safety**  
   Defines conflict profiles, scoped consent, infrastructure permissions, grief recovery, moderation, appeals, anti-cheat boundaries, and inactive-authority recovery.

5. **Canon integration and prototype gates**  
   Connects the new contracts to the canonical spine, progression, session rhythm, playtesting, and roadmap.

6. **Active metadata and link normalization**  
   Repairs malformed front matter, completes ownership/version/scope metadata, and fixes the remaining internal documentation link.

7. **Registry, manifest, and quality audits**  
   Regenerates machine-readable document metadata and records validation results.

## Validation Result

```text
Markdown documents: 168
Approximate words: 358,887
Broken internal links: 0
Active metadata problems: 0
Duplicate active titles: 0
```
