# Symtropy Public Asset Automation Foundry v0.2

**Research and implementation strategy**  
**Project:** Symtropy / Luminous Dynamics  
**Engine target:** Bevy 0.18.x stable baseline  
**Date:** 2026-06-15  
**Status:** v0.2 refinement document, not legal advice

## Executive decision

Yes: Symtropy can automate a public/open game-art and assets library, but the automation must be **rights-aware**, **source-respecting**, **Bevy-export-oriented**, and **style-gated**. The correct strategy is not "download every free thing." It is a license-gated asset foundry that ingests only assets whose provenance, license, creator/source, source URL, access method, checksum, and transformation history are recorded.

The production baseline remains **CC0-first**. CC0 is the lowest-friction lane because it is intended as a "no rights reserved" alternative to Creative Commons licenses [S1]. Even then, CC0 does not remove every possible non-copyright issue, such as privacy, publicity, trademark, moral rights, cultural sensitivity, or lack of warranty [S2]. Every imported asset still needs a provenance record.

## The core rule

> Automate ingestion, conversion, tagging, thumbnailing, reporting, and export. Do not automate trust.

## v0.2 refinement summary

This revision applies the first review pass and hardens the docs for implementation:

1. Adds explicit CLI **dry-run**, **plan**, and **review-mode** behavior.
2. Defines the boundary between **clear AI provenance** and **unclear AI provenance**.
3. Turns the Symtropy style pass from an abstract idea into a decision process tied to the new **Symtropy Visual Vocabulary and Biome Palette Spec v0.1**.
4. Adds **Known Hard Parts / Technical Traps** to each implementation milestone.
5. Adds concrete Bevy/glTF conversion concerns: Blender headless fragility, material conversion, animation clip naming, LOD naming, and registry-export stability.
6. Preserves the original CC0-first ethical spine and four-state license gate.

## Recommended source tiers

| Tier | Use | Sources | Policy |
|---|---|---|---|
| Tier 0 | Production-safe starting lane | Kenney, ambientCG, Quaternius, KayKit, Smithsonian Open Access CC0 subset | CC0-first; attribution not legally required but still keep provenance. |
| Tier 1 | High-value but access constrained | Poly Haven | Assets are CC0, but the public API has separate commercial restrictions. Use manual/low-volume compliant downloads or obtain API permission/custom license/sponsorship for commercial-scale automation. |
| Tier 2 | Curated/import-by-review only | OpenGameArt, Sketchfab, Freesound, Wikimedia Commons, NASA media | Mixed licenses, attribution requirements, API constraints, or extra trademark/personality/cultural rights issues. Human review required before production use. |
| Blocked | Do not ingest | Fan art, ripped game assets, NC/ND works, unclear marketplace freebies, brand/trademark assets, unknown AI provenance | Quarantine or reject by default. |

## Source research matrix

| Source | Asset types | License/access finding | Automation recommendation |
|---|---|---|---|
| Kenney | 2D, UI, icons, sprites, 3D, prototyping packs | Kenney says game assets on asset pages are public-domain/CC0 and usable commercially; attribution is not required [S5]. | Use in v0.1/v0.2. Prefer pack-level downloads or explicit local source manifests. |
| ambientCG | PBR materials, textures, models, HDRIs | ambientCG states assets are CC0, including downloadable asset files and previews; raw files can be included in a video game [S9]. Its docs also expose creation-method metadata such as photogrammetry, procedural, HDRI, and approximated materials [S25]. | Use in v0.1/v0.2 for materials. Cache API snapshots and record creation method when available. |
| Quaternius | Low-poly models, animated rigs, props, environment packs | FAQ says assets are CC0, modifiable, commercial-use OK, and attribution is not required [S11]. | Strong Symtropy fit. Use seed manifests and manual pack downloads first. |
| KayKit | Low-poly characters, props, environments, animations | KayKit pages list free personal/commercial use, no attribution required, CC0, and often include GLTF/FBX [S12]. | Strong Symtropy fit. Use seed manifests and normalize to GLB/glTF. |
| Smithsonian Open Access | 2D images, 3D models, metadata | Open Access items are CC0; commercial use is allowed for CC0-designated assets, but third-party rights can still apply [S13]. | Good for references, museum artifacts, educational/history props. Use only CC0-designated records. |
| Poly Haven | HDRIs, PBR textures, 3D models | Assets are CC0 and may be redistributed, included in products, and used commercially [S6]. Its hosted API is separate: free for non-commercial/academic use, commercial API use requires custom licensing/sponsorship, and API calls need a unique user-agent/referer [S7][S8]. | Use carefully. Either use manual downloads, get permission, or support/sponsor API access before automated commercial-scale ingestion. |
| OpenGameArt | 2D, 3D, textures, audio, music | Mixed licenses. CC0 is easiest; CC-BY requires credit and change notices; CC-BY-SA imposes share-alike; GPL art is complex for non-GPL projects [S14]. Forum guidance warns that scraping may get an IP blocked if detrimental to site performance [S15]. | Do not bulk scrape. Use hand-curated CC0/OGA-BY items only after review. |
| Sketchfab | Downloadable 3D models | Download API requires user authentication unless otherwise arranged. Models are Creative Commons, but attribution and license must follow the model everywhere it is used [S16]. | Review-only. Good for individual hero references, not default automation. |
| Freesound | Sound effects, ambience, audio textures | API is free only for non-commercial purposes unless commercial terms are arranged; proper credit is required according to the sound licenses [S17]. | Review-only. Prefer CC0 sounds and local attribution ledger. |
| Wikimedia Commons | Images, audio, video, some models | Most content is reusable, but each file has different licensing requirements, and Wikimedia disclaims warranty; reuse can involve attribution, license links, share-alike, or other rights [S18]. | Review-only. Use only specific files with verified license snapshots. |
| NASA media | Space imagery, video, audio | NASA media has commercial and promotional-use restrictions around endorsement, logos, identifiable persons, merchandise, and NASA employees [S19]. | Use as research/reference; do not automate into game products without review. |

## License policy

### Approved by default

| License/status | Use | Conditions |
|---|---|---|
| CC0 | Production-safe default | Keep source URL, creator/source organization, license snapshot, retrieval timestamp, and checksum even though attribution is not required. |
| Public Domain Mark / clearly public domain | Allowed after review | Confirm that the source is authoritative and record the evidence. |
| Smithsonian Open Access CC0 | Allowed after review | Only if the item is explicitly CC0/Open Access. Watch for cultural sensitivity, trademarks, and third-party rights [S13]. |

### Allowed only with attribution ledger

| License/status | Use | Conditions |
|---|---|---|
| CC BY 4.0 | Can be used commercially and adapted, but requires appropriate credit, a license link, and change indication [S3]. | Must generate `ATTRIBUTION.md` and in-game credits. |
| OGA-BY | Similar to CC-BY but designed for OpenGameArt use cases; attribution required [S14]. | Must generate credits and record changes. |

### Quarantine by default

| License/status | Reason |
|---|---|
| CC BY-SA | Commercial/adaptation allowed, but derivatives must use the same license and attribution is required [S4]. May contaminate derivative art packs. |
| GPL/LGPL art | OpenGameArt itself notes GPL art is complex for non-GPL projects [S14]. |
| Mixed/unknown license | Cannot pass the provenance gate. |
| AI-generated or AI-assisted assets with unclear provenance | Legal status, creator consent, training-source provenance, platform disclosure duties, and output license may be unclear. Review manually. |

### Clear AI provenance policy

AI-generated or AI-assisted public assets are **not automatically blocked**, but they must clear a higher bar. The default state is `QUARANTINE_REVIEW` until the following evidence is present.

An AI-assisted asset may move to `APPROVED_CC0` or `APPROVED_ATTRIBUTION_REQUIRED` only when all applicable fields are known:

| Required evidence | Why it matters |
|---|---|
| Human or organizational publisher is known | The foundry needs an accountable source, not only a generated file. |
| Output license is explicit | "Generated" is not a license. The asset still needs CC0, CC-BY, or another policy-compatible status. |
| Generator/service/tool is known when disclosed | Helps evaluate platform terms, commercial-use terms, and repeatability. |
| Model/service terms permit commercial game use and redistribution of outputs | A source may allow use but not redistribution of raw files. |
| If image-to-image, texture-to-texture, or model-to-model was used, input asset licenses are known | The output may inherit risk from input references. |
| No living artist style prompt, fan-IP prompt, trademark prompt, or identifiable real-person likeness | Avoids reputational, trademark, publicity, and creator-consent risk. |
| Player-facing AI disclosure status is recorded | Steam's Content Survey requires detail for AI-created content that ships with the game and is consumed by players, including artwork, sound, narrative, and localization [S26]. |
| Human art-direction review has approved the asset | The foundry must preserve Symtropy coherence, not just legal compatibility. |

Practical decisions:

- A CC0 asset from a source that clearly states its license and creation method may pass review, but the manifest must record `ai_assisted: true/false/unknown`.
- A Stable Diffusion/Midjourney/unknown-generator output with no source, no model/service terms, no input provenance, and no publisher record remains quarantined.
- AI-generated concept art may be used as internal reference if labeled, but final shipping assets should prefer human-authored, CC0, procedural, photogrammetric, or hand-edited production files unless a release lead explicitly approves the AI asset.
- The foundry should generate a separate `AI_DISCLOSURE_REGISTER.md` for any player-facing AI-assisted asset.

### Blocked

| Asset type | Reason |
|---|---|
| NC licenses | Blocks commercial use. |
| ND licenses | Blocks derivative/adapted versions, incompatible with Symtropy style transformation. |
| Fan art / ripped assets / commercial game modifications | OpenGameArt explicitly excludes modifications of existing commercial game art and clearly non-free IP [S14]. |
| Logos, trademarks, identifiable real people | CC0 and public domain do not necessarily clear trademark, privacy, or publicity rights [S2][S13][S19]. |
| Assets from marketplaces that are merely "free" | Free price does not equal redistribution or open license. |

## Foundry architecture

```text
Source catalog / seed manifests
        |
        v
Plan / dry-run inventory
        |
        v
Source adapter layer
        |
        v
License + provenance gate
        |
        +--> Reject / quarantine / pending review
        |
        v
Raw immutable asset vault
        |
        v
Normalization and conversion workers
        |
        v
Technical QA: hash, dimensions, polycount, texture maps, audio length
        |
        v
Symtropy style/tag pass
        |
        v
Human review queue
        |
        v
Approved Bevy export library
```

## Major components

### 1. Source adapters

Adapters should be small, source-specific modules. They must know how to retrieve metadata without violating source access rules.

```text
asset_foundry/sources/
  kenney_seed.py
  ambientcg_api.py
  quaternius_seed.py
  kaykit_seed.py
  smithsonian_api.py
  polyhaven_manual_or_licensed_api.py
  opengameart_review_only.py
  sketchfab_review_only.py
  freesound_review_only.py
```

Rules:

- Prefer official APIs where terms allow the intended use.
- Prefer manual seed manifests when no stable API exists.
- Never scrape at high volume.
- Every network request should include a meaningful user-agent.
- Every imported asset should preserve source URL, source page snapshot path, license URL, creator/source organization, retrieval timestamp, and acquisition method.
- Access method is part of provenance: `manual_download`, `official_api`, `licensed_api`, `local_pack`, or `review_only`.

### 2. License and provenance gate

The gate returns one of four states:

```text
APPROVED_CC0
APPROVED_ATTRIBUTION_REQUIRED
QUARANTINE_REVIEW
REJECTED
```

Minimum checks:

- License is allowlisted.
- Source URL is present.
- Creator or source organization is known.
- Asset file checksum exists.
- License page or source metadata snapshot is stored locally.
- Commercial-use flag is explicit.
- Derivative/adaptation flag is explicit.
- Attribution requirement is explicit.
- API/server terms were not violated during acquisition.
- AI provenance state is one of `not_ai`, `clear_ai_provenance`, or `unknown_ai_provenance`; unknown AI provenance cannot be approved.

### 3. Raw immutable asset vault

Raw downloads should be treated as evidence, not working files.

```text
symtropy-assets/
  raw_vault/
    kenney/
    ambientcg/
    quaternius/
    kaykit/
    smithsonian/
  registry/
    assets.sqlite
    snapshots/
    license_report.md
    attribution.generated.md
    ai_disclosure_register.md
  pending_review/
  quarantine/
  processed/
  bevy_export/
```

Rules:

- Raw vault files are never edited in place.
- Every processed/exported asset points back to a raw file hash.
- Conversion logs are append-only provenance events.
- "Archive, do not delete" applies to rejected or deprecated assets: move them to quarantine/archive with reason codes.

### 4. Registry

Use SQLite first. It is enough for v0.1/v0.2 and is easy to inspect.

```sql
assets(
  id, title, type, source_name, source_url, creator,
  license_id, license_url, acquisition_method,
  ai_provenance_state, status, created_at, updated_at
)
files(id, asset_id, role, path, sha256, size_bytes, mime_type)
licenses(id, spdx_id, name, url, attribution_required, commercial_allowed, derivative_allowed, share_alike)
provenance_events(id, asset_id, event_type, timestamp, actor, details_json)
review_events(id, asset_id, reviewer, status, timestamp, notes)
exports(id, asset_id, target_engine, profile, output_path, timestamp, transform_log)
style_reviews(id, asset_id, palette_id, material_family, biome, branch, reviewer, status, notes)
ai_disclosures(id, asset_id, generator, model_or_service, prompt_hash, input_sources_json, platform_disclosure_required, notes)
```

## Manifest schema

Every asset should produce a human-readable YAML manifest.

```yaml
id: symtropy.env.fungal_bark.material.0001
title: Fungal bark material variant 0001
type: material
source:
  name: ambientCG
  source_url: https://ambientcg.com/a/example
  creator: ambientCG
  retrieved_at: 2026-06-15T00:00:00Z
  acquisition_method: official_api
license:
  id: CC0-1.0
  url: https://creativecommons.org/publicdomain/zero/1.0/
  attribution_required: false
  commercial_use_allowed: true
  derivatives_allowed: true
ai:
  ai_assisted: false
  provenance_state: not_ai
files:
  raw:
    path: raw_vault/ambientcg/example.zip
    sha256: ...
  processed:
    path: processed/materials/fungal_bark_0001/
style:
  visual_spec_version: symtropy_visual_vocab_v0.1
  palette_id: wetland_mycelial
  material_family: biospheric_tissue
  surface_archetype: bark_skin
  emissive_role: biospheric_signal
  art_status: pending_review
bevy:
  export_profile: desktop
  canonical_runtime_format: glb_or_material_bundle
  texture_budget: medium
  lod_policy: material_only
review:
  legal_status: approved_cc0
  technical_status: pending
  art_status: pending
```

## Bevy folder layout

```text
symtropy_game/
  assets/
    symtropy/
      manifest.index.ron
      attribution.generated.md
      ai_disclosure_register.md
      environments/
        biospheric_wetland/
          materials/
          models/
          decals/
          atmosphere/
      species/
      robotics/
      civic_infrastructure/
      ui/
      audio/
    third_party/
      cc0/
      attribution_required/
  tools/
    asset_foundry/
```

Recommended Bevy asset IDs:

```text
symtropy://env/biospheric_wetland/material/fungal_bark_0001
symtropy://robotics/field_station/model/solar_lora_node_0001
symtropy://ui/tech_tree/icon/water_reuse_0001
symtropy://audio/ambience/mistkin_morning_0001
```

## Symtropy style pass

Public assets become Symtropy assets only after a style layer. This prevents the world from looking like a collage of asset packs.

The style pass is now governed by **Symtropy Visual Vocabulary and Biome Palette Spec v0.1**. That document defines canonical HSL ranges, material-family parameters, emissive conventions, biome mood, and avoid-lists.

### Style pass decision process

```text
1. Identify asset role
   model / material / icon / audio / decal / animation / ambience

2. Identify world branch
   biospheric_intelligences / civic_infrastructure / robotics / habitats / xeno_translation / public_works / seedworks

3. Select biome palette
   wetland_mycelial / mist_forest / desert_spore / ocean_reef_mind / orbital_commons / subterranean_archive / civic_commons / red_bloom_hazard / ice_shell_ocean

4. Select material family
   living_infrastructure / biospheric_tissue / robotics_care_machine / governance_ritual_surface / hazard_boundary / habitat_shell / ui_holographic

5. Apply automated safe transforms
   scale/origin normalization, texture-size clamp, roughness/metallic remap, palette tint, thumbnail, turntable, metadata tags

6. Stage expressive transforms for review
   emissive veins, moss/lichen overlays, consent markings, civic decals, procedural spore layers, weathering, narrative scars

7. Human art-direction review
   approve, request revision, quarantine style, or archive
```

### Automated style operations

| Operation | Automated? | Notes |
|---|---:|---|
| Scale normalization | Yes | Enforce unit conventions and origin placement. |
| File naming and asset ID generation | Yes | Use stable IDs; never depend on upstream filenames alone. |
| Texture budget clamp | Yes | Downscale or variant-generate by export profile. |
| Roughness/metallic remap | Yes, bounded | Use material-family defaults; do not erase authored intent. |
| Palette harmonization | Semi-automated | Use biome HSL ranges; stage strong changes for review. |
| Emissive overlays | Semi-automated | Must encode meaning: signal, care, warning, memory, ritual, trust, hazard. |
| Moss/lichen/humidity/weathering decals | Semi-automated | Generate variants but require art approval. |
| Consent/civic markings | Human-approved | These are worldbuilding semantics, not random decals. |
| Creature/species morphology changes | Human-approved | Avoid accidental lore drift. |

### Material mapping into Bevy

For glTF/GLB assets, map source materials to Bevy `StandardMaterial` first. Bevy's glTF root exposes loaded named scenes, meshes, materials, nodes, skins, and named animations, so source naming matters for stable lookups [S27].

Minimum conversion rules:

- Principled BSDF base color -> base color/albedo.
- Metallic and roughness -> metallic/roughness channels or ORM map.
- Normal map -> tangent-space normal, with explicit normal convention check.
- Emissive map/color -> Bevy emissive fields, bounded by biome convention.
- Alpha mode -> explicit `opaque`, `mask`, or `blend`; avoid accidental blended materials.
- Unsupported Blender node networks -> bake to maps or quarantine for manual material authoring.
- Animation clips must use semantic names such as `idle`, `walk`, `repair_loop`, `scan_loop`, `open`, `close`, `deploy`, not `Take 001`.

Review rule:

> A public asset is not approved for final Symtropy use until it has both a legal status and an art-direction status.

## Tag taxonomy

```yaml
branches:
  - biospheric_intelligences
  - civic_infrastructure
  - robotics
  - habitats
  - xeno_translation
  - public_works
  - seedworks
biomes:
  - wetland_mycelial
  - mist_forest
  - desert_spore
  - ocean_reef_mind
  - orbital_commons
  - subterranean_archive
  - civic_commons
  - red_bloom_hazard
  - ice_shell_ocean
asset_roles:
  - hero_prop
  - set_dressing
  - material
  - decal
  - icon
  - animation
  - ambience
  - ui
status:
  - raw_imported
  - pending_review
  - legally_approved
  - style_approved
  - bevy_ready
  - deprecated
  - archived
```

## CLI specification

The CLI must surface human checkpoints explicitly.

```bash
# Validate source manifests without network or downloads
symtropy-assets validate sources/kenney.yaml

# Build a candidate inventory without downloading asset files
symtropy-assets plan sources/kenney.yaml --policy cc0-only --out review/kenney.plan.json

# Dry-run the full gate: parse metadata, evaluate licenses, estimate downloads, but write no registry state
symtropy-assets ingest sources/kenney.yaml --policy cc0-only --dry-run

# Review-mode downloads into pending_review and creates manifests, but approves nothing
symtropy-assets ingest sources/kenney.yaml --policy cc0-only --review-mode

# Limit large packs while testing
symtropy-assets ingest sources/kenney.yaml --policy cc0-only --review-mode --batch-limit 25

# Explicitly approve after legal/style review
symtropy-assets approve --asset symtropy.env.fungal_bark.material.0001 --legal approved_cc0
symtropy-assets style-approve --asset symtropy.env.fungal_bark.material.0001 --palette wetland_mycelial --material-family biospheric_tissue

# Query registry
symtropy-assets search --tag fungal --license CC0-1.0

# Generate previews and technical reports
symtropy-assets preview --asset symtropy.env.fungal_bark.material.0001
symtropy-assets audit --asset symtropy.env.fungal_bark.material.0001

# Export Bevy-ready assets only after legal + style approval
symtropy-assets export bevy --profile desktop --include-attribution
symtropy-assets export bevy --profile web --texture-profile ktx2-with-png-fallback

# Generate legal/credits/AI outputs
symtropy-assets credits --format markdown --out assets/symtropy/attribution.generated.md
symtropy-assets license-report --out registry/license_report.md
symtropy-assets ai-disclosure-report --out registry/ai_disclosure_register.md
```

### CLI behavior at pack boundaries

If a manifest points to a pack with 200 assets:

1. `validate` checks the manifest only.
2. `plan` inventories all candidates and prints counts by license/status/source/type.
3. `ingest --dry-run` evaluates the gate but writes nothing.
4. `ingest --review-mode` writes immutable raw files and per-asset manifests into `pending_review/`, but does not mark them approved.
5. `--batch-limit` is required during initial integration until the source adapter has proven stable.
6. `--confirm-import` or a review command is required before anything can enter `processed/` or `bevy_export/`.
7. `--auto-approve-cc0` may exist only for local CI fixtures and locally mirrored known-good packs, not for live web ingestion.

## V0.2 implementation plan with known hard parts

### Milestone A: provenance core

Deliverables:

- SQLite registry.
- YAML asset manifest schema.
- License policy file.
- Hashing and immutable raw-vault storage.
- Quarantine state machine.
- Generated `ATTRIBUTION.md`, `LICENSE_REPORT.md`, and `AI_DISCLOSURE_REGISTER.md`.

Known hard parts / technical traps:

- License names are inconsistent across sources: normalize to SPDX-style IDs where possible but keep original license text/snapshot.
- Source pages can change after download: store retrieval timestamp, URL, and a local metadata snapshot.
- Raw archive checksums and extracted-file checksums are both useful; record both when practical.
- API/server terms may differ from asset license; the registry needs `acquisition_method` and `access_terms_snapshot`.
- Do not delete rejected assets silently. Archive with rejection reason and source evidence.

### Milestone B: CC0 seed ingestion

Start with manually curated seed manifests for:

- Kenney.
- Quaternius.
- KayKit.
- ambientCG.
- Smithsonian Open Access CC0 3D/2D records.

Avoid Poly Haven API automation until commercial API permission/sponsorship is clarified. The assets are CC0, but the hosted API terms are separate [S6][S8].

Known hard parts / technical traps:

- Pack downloads often contain hundreds of files with weak internal metadata.
- Some sources expose license at pack level, not file level; the manifest must inherit pack-level evidence correctly.
- Duplicate assets across packs should collapse by checksum but preserve multiple provenance events.
- Manual seed manifests are slower at first but safer than scraping.
- Some "CC0" sources still include logos, humans, cultural artifacts, or brand-like designs that need review.

### Milestone C: conversion and QA

Deliverables:

- Blender CLI conversion pipeline for model cleanup and GLB export.
- Texture normalization to power-of-two sizes when needed.
- Optional KTX2 lane with PNG fallback.
- Thumbnail generation.
- Technical audit: triangle count, texture dimensions, missing maps, animation clips, file size, alpha mode, material family, LOD availability.

Known hard parts / technical traps:

- Blender headless mode is fragile: pin Blender version, run in a container/devshell, and treat conversion logs as artifacts.
- Blender node graphs do not always map cleanly to Bevy `StandardMaterial`; unsupported nodes must be baked or quarantined.
- Material channel conventions vary: roughness, metallic, AO, normal handedness, sRGB/linear color, and alpha mode must be checked.
- Animation clip names are often unstable (`Take 001`, `ArmatureAction`). Rename clips to semantic Bevy-facing names before export.
- Coordinate systems, scale, object origins, bone rolls, and transforms must be normalized.
- Do not overwrite source files; conversions write to `processed/` and record a transform log.

### Milestone D: Bevy export pack

Deliverables:

- `assets/symtropy/` export directory.
- `manifest.index.ron` or `manifest.index.json`.
- In-game attribution document.
- Bevy test scene that loads a sample model, material, UI icon, audio asset, animation, and particle asset.
- Hot-reload development profile using Bevy file watching and/or Bevy asset processing.

Known hard parts / technical traps:

- Bevy asset handles and glTF sub-assets can be index-sensitive; prefer stable named scenes/materials/animations where practical [S27].
- Hot reload is for development, not a substitute for deterministic export. Bevy's asset processor supports file watching and processing transforms, but processed outputs should remain reproducible from source assets and transform steps [S28].
- LOD groups must be exported with consistent origins and stable naming to avoid popping.
- Web builds need tighter texture, animation, and particle budgets than desktop builds.
- The export pack must fail closed: if license, attribution, AI disclosure, or art status is missing, block export.

## Review workflow

| Stage | Reviewer | Output |
|---|---|---|
| License review | Human or maintainer | approved, attribution-required, quarantine, rejected |
| AI provenance review | Human or maintainer | not-ai, clear-ai-provenance, unknown-ai-provenance, blocked |
| Technical review | Automation | pass/fail report |
| Art-direction review | Human/artist | Symtropy branch, biome, palette, role, material family, status |
| Bevy integration review | Developer | sample scene loads without warnings |

## Risk register

| Risk | Severity | Mitigation |
|---|---:|---|
| Treating public visibility as open license | Critical | Block import unless license is explicit and recorded. |
| Violating API/server terms | High | Use official APIs only within terms; manual seed manifests where needed. |
| Attribution loss | High | Generate attribution from registry; block export if required fields missing. |
| AI provenance ambiguity | High | Quarantine unless output license, source, tool/service terms, and platform disclosure status are clear. |
| Style incoherence | Medium | Require Visual Vocabulary style pass and tags before final approval. |
| Upstream takedown/license change | Medium | Store retrieval date, source snapshot, and checksums. |
| Trademark/personality/cultural-sensitivity issues | High | Special review lane for logos, NASA, Smithsonian cultural material, identifiable people. |
| Asset bloat | Medium | Store raw vault separately; export only reviewed and optimized runtime variants. |
| Engine format drift | Medium | Keep source files and regenerate Bevy exports when engine version changes. |
| Blender conversion instability | Medium | Pin Blender version, containerize/devshell, preserve logs, add golden sample tests. |

## Minimum viable repository structure

```text
symtropy-assets/
  README.md
  LICENSE_POLICY.md
  VISUAL_VOCABULARY.md
  sources/
    kenney.yaml
    quaternius.yaml
    kaykit.yaml
    ambientcg.yaml
    smithsonian.yaml
  registry/
    assets.sqlite
    license_report.md
    attribution.generated.md
    ai_disclosure_register.md
    snapshots/
  raw_vault/
  pending_review/
  quarantine/
  processed/
  bevy_export/
  tools/
    symtropy_assets/
      __init__.py
      cli.py
      registry.py
      license_gate.py
      ai_provenance.py
      style_gate.py
      hash_files.py
      sources/
      converters/
      exporters/
      reports/
  tests/
    test_license_gate.py
    test_ai_provenance_gate.py
    test_manifest_schema.py
    test_attribution_generation.py
    test_bevy_export_paths.py
    test_style_gate.py
```

## Acceptance tests for v0.2

The foundry v0.2 is successful when:

1. It can run `validate`, `plan`, `ingest --dry-run`, and `ingest --review-mode` against at least one seed manifest.
2. It ingests at least 100 CC0 assets from seed manifests into `pending_review/` without auto-approving them.
3. Every asset has a checksum, source URL, license ID, retrieval timestamp, acquisition method, and AI provenance state.
4. Unknown/mixed licenses are quarantined automatically.
5. Unknown AI provenance is quarantined automatically.
6. An attribution report is generated even when all assets are CC0.
7. An AI disclosure register is generated even when it is empty.
8. At least one 3D model, material, icon, audio asset, and animation asset loads in a Bevy test scene.
9. The Bevy export contains only legal-approved + style-approved assets.
10. The raw vault remains unchanged after conversion.
11. The same source manifest can regenerate the same asset registry state.
12. A conversion log exists for every processed model/material.
13. A style-review record exists for every approved asset.

## Implementation Progress (June 2026)

### Phase 1: Registration Loop (Completed)
- **Status**: The processing loop is closed.
- **Details**: Blender-normalized assets (`_normalized.glb`) are now automatically registered in the database with `role='optimized'`.
- **Export**: The `export_pack` command uses `COALESCE` to prioritize optimized assets over raw sources.

### Phase 2: Integrated Toolchain (Completed)
- **Status**: Unified via `justfile`.
- **Details**: Added `foundry-status`, `foundry-ingest`, `foundry-convert`, and `foundry-export`.
- **Environment**: All dependencies (including `jsonschema`) are bundled in `nix develop`.

### Phase 3: Engine-side Smart Ingestion (Completed)
- **Status**: Rust crate `symtropy-foundry` implemented.
- **Details**: Provides `FoundryPlugin` which automatically wires:
  - `_COLLISION`: Auto-hides mesh and adds a `symtropy-physics` sphere proxy.
  - `_LODn`: Assigns `VisibilityRange` based on engine strategy.

## Recommendation

Build the Symtropy Asset Foundry as a **CC0-first asset operating system**:

```text
Legal certainty first.
Provenance always.
AI provenance explicit.
Style transformation second.
Bevy export third.
Automation only after trust is established.
```

This turns public/open assets into a coherent, legally clean Symtropy world-library instead of a pile of downloads.

## Sources consulted

- [S1] Creative Commons: Public Domain and CC0 overview. https://creativecommons.org/public-domain/
- [S2] Creative Commons: CC0 1.0 deed, limitations and no warranties. https://creativecommons.org/publicdomain/zero/1.0/deed.en
- [S3] Creative Commons: CC BY 4.0 deed. https://creativecommons.org/licenses/by/4.0/deed.en
- [S4] Creative Commons: CC BY-SA 4.0 deed. https://creativecommons.org/licenses/by-sa/4.0/deed.en
- [S5] Kenney support page: game asset licensing. https://kenney.nl/support
- [S6] Poly Haven asset license. https://polyhaven.com/license
- [S7] Poly Haven API overview. https://polyhaven.com/our-api
- [S8] Poly Haven Public API terms of service. https://github.com/Poly-Haven/Public-API/blob/master/ToS.md
- [S9] ambientCG license documentation. https://docs.ambientcg.com/license/
- [S10] ambientCG API documentation. https://docs.ambientcg.com/api/
- [S11] Quaternius FAQ. https://quaternius.com/faq.html
- [S12] KayKit character asset page. https://kaylousberg.com/game-assets/characters-adventurers
- [S13] Smithsonian Open Access FAQ. https://www.si.edu/openaccess/faq
- [S14] OpenGameArt FAQ and license summaries. https://opengameart.org/content/faq
- [S15] OpenGameArt forum note on API/scraping caution. https://opengameart.org/forumtopic/opengameart-api
- [S16] Sketchfab Download API guidelines. https://sketchfab.com/developers/download-api/guidelines
- [S17] Freesound API terms of use. https://freesound.org/docs/api/terms_of_use.html
- [S18] Wikimedia Commons reuse guidance. https://commons.wikimedia.org/wiki/Commons:Reusing_content_outside_Wikimedia
- [S19] NASA image and media usage guidelines. https://www.nasa.gov/nasa-brand-center/images-and-media/
- [S20] Bevy glTF module documentation. https://docs.rs/bevy/latest/bevy/gltf/index.html
- [S21] Bevy cargo features documentation. https://github.com/bevyengine/bevy/blob/main/docs/cargo_features.md
- [S22] Bevy hot asset reloading example. https://github.com/bevyengine/bevy/blob/main/examples/asset/hot_asset_reloading.rs
- [S23] Khronos glTF overview. https://www.khronos.org/gltf/
- [S24] Khronos KTX overview. https://www.khronos.org/ktx/
- [S25] ambientCG creation methods documentation. https://docs.ambientcg.com/creation-methods/
- [S26] Steamworks Content Survey: Generative Artificial Intelligence Content. https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- [S27] Bevy `Gltf` docs: named scenes, meshes, materials, nodes, skins, animations. https://docs.rs/bevy/latest/bevy/gltf/struct.Gltf.html
- [S28] Bevy asset processor docs. https://docs.rs/bevy/latest/bevy/asset/processor/index.html
