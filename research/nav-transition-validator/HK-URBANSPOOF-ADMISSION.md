# HK-UrbanSpoof candidate admission note

**Status:** `candidate_not_admitted`; use scope `none`.  
**Pinned upstream:** `Fangjingxiaotao/HK-UrbanSpoof@95def2b6a42eee6299bbc1a5b880a9f3bfc070d6`.  
**Issue:** [NAV-HK-URBANSPOOF-001 #6709](https://github.com/Luminous-Dynamics/symtropy/issues/6709).

## What is present

The pinned repository root contains the ION GNSS+ 2026 paper PDF, README, figures and S2/S3 GNSS/INS text files. The README identifies S2 as pedestrian/coastal-suburban and S3 as vehicle/dense-urban. It describes S2_GT/S3_GT as time-tagged post-processed GNSS/INS position, velocity and attitude. Scenario S1 and S4 use fixed coordinate references in the README; S4 is simulated and uses a different IF acquisition configuration.

The README's raw IF files are linked through an external Google Drive folder. They are not present in the Git repository tree. The README estimates the recordings at multi-gigabyte scale (around 10–13 GB per file, with S4_Clean using a distinct configuration). Exact downloaded objects and SHA-256 checksums have not been frozen.

## Why admission is blocked

- GitHub API reports no repository licence and the root has no LICENSE file.
- The payload licence and Google Drive terms are not explicitly stated in the pinned repository.
- Research use, commercial use and redistribution permissions are therefore all unknown.
- Raw IF objects are external and not frozen by checksums.
- The README says ray-tracing labels are coming soon. They are not part of the pinned Git tree.
- The README retains an older statement saying ground truth will be released after publication, while S2_GT and S3_GT files are already present in the pinned tree and the ION GNSS+ 2026 paper is published. Treat this as a release-documentation inconsistency; obtain author confirmation.
- S2/S3 trajectory files need review for exact clock/time scale, antenna/IMU lever-arm, datum/frame, processing version and independence of truth from the spoofed receiver under evaluation.
- Geometric ray-tracing labels, when released, do not necessarily represent dynamic objects, foliage, irregular surfaces or all propagation conditions.

## Safe benchmark design after rights and oracle qualification

Preserve each complete spoofing test and each physical baseline recording as an indivisible group. Keep S1/S2/S3 real-source scenarios separate from S4 simulation. Report performance by environment, receiver motion, spoofing power advantage and target displacement, including 30 m small-displacement tests.

Use the data to compare receiver-level anti-spoofing and integrity behaviour only after:
1. the authors confirm payload rights and the exact current release;
2. raw IF files, metadata and trajectory references are downloaded and SHA-256 hashed;
3. receiver/antenna/front-end and timing provenance are documented;
4. the GNSS/INS truth dependency and processing lineage are reviewed;
5. all splits hold out complete spoofing experiments and physical sessions.

Do not treat 17 attacks as 17 independent physical environments. Multiple tests share the same scenario/environment/recording setup. Use experiment-level grouping and cluster-aware intervals.

## Source references

- Repository: https://github.com/Fangjingxiaotao/HK-UrbanSpoof
- Pinned README: https://github.com/Fangjingxiaotao/HK-UrbanSpoof/blob/95def2b6a42eee6299bbc1a5b880a9f3bfc070d6/README.md
- ION GNSS+ 2026 paper: https://doi.org/10.33012/2026.20861
- ION abstract: https://www.ion.org/publications/abstract.cfm?articleID=20861
