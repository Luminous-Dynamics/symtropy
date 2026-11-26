# DNA Molecular Memory Channel — v0.1

**Programme:** MOLECULAR-MEMORY-CHANNEL-001  
**Tracking:** [Symtropy #7567](https://github.com/Luminous-Dynamics/symtropy/issues/7567)  
**Compiler:** [Symtropy #7482](https://github.com/Luminous-Dynamics/symtropy/issues/7482)  
**Physical systems:** [Symtropy #7447](https://github.com/Luminous-Dynamics/symtropy/issues/7447)

## Evidence basis

Öztürk et al., “DNA Data Storage: A Critical Review of Bio-Algorithmic Integration, Performance-Cost Tradeoffs, and Future Directions for Scalable Architectures,” published 3 June 2026, reviews synthesis, PCR/CRISPR access, sequence coding, biochemical noise and the integration of biological constraints with algorithmic optimization:
https://link.springer.com/article/10.1007/s11831-026-10651-2

The review identifies synthesis cost, sequencing errors, write/read efficiency and retrieval latency as scaling barriers. It describes substitution, insertion, deletion, oligo dropout, sequence constraints and amplification bias. These are platform-/protocol-dependent effects. The review's example error figures must not be treated as universal DNA-channel constants.

## Why DNA requires a distinct adapter

The physical channel is not simply a byte array with a bit-error rate. It is a sequence of biochemical and computational stages:

digital data → constrained nucleotide encoding → synthesis → storage/encapsulation → selective retrieval → amplification → sequencing → basecalling → alignment/clustering → synchronization-aware decoding → exact payload verification.

The adapter preserves raw reads and quality observations separately from aligned sequences and expected labels. Ground truth is held in a separately referenced corpus so the decoder/optimizer cannot use it at inference time.

## Error and uncertainty vocabulary

Keep distinct:
- **Substitution:** one nucleotide called as another.
- **Insertion/deletion:** sequence length changes and synchronization shifts.
- **Dropout:** an encoded oligo is not recovered in a read batch.
- **Abundance bias:** unequal copy/read abundance across oligos, including PCR amplification bias.
- **Duplicate reads:** repeated observations that are not independent molecules.
- **Contamination/cross-talk:** reads assigned to the wrong library or index.
- **Sequence constraint violation:** GC, homopolymer or motif rules not met.

A decoder must not count PCR duplicates as independent evidence without a validated model. A missing oligo is not the same mechanism as an indel within a recovered sequence.

## Baseline tournament

- Constrained sequence encoders that enforce declared GC/homopolymer/motif rules.
- Marker and synchronization codes for insertion/deletion recovery.
- VT/marker/indel-aware codes where their assumptions fit the measured channel.
- Reed–Solomon/BCH for substitutions after sequence alignment.
- Fountain/Luby Transform or other erasure coding for oligo dropout.
- LDPC/soft-decision alternatives where per-base quality scores have been calibrated.
- Primer/index and random-access methods evaluated for selection accuracy, cross-talk and retrieval overhead.

Compare the end-to-end chain, not isolated code rate. Measure net recoverable bits per nucleotide after sequence constraints, index, parity, dropout redundancy and manifest overhead.

## Required measurements

- Synthesis platform, chemistry, batch, oligo length distribution and accepted-oligo yield.
- Sequencing platform, chemistry, basecaller/version, raw reads and quality-score calibration.
- Substitution, insertion, deletion, dropout and abundance distributions by position and sequence context.
- PCR cycle conditions and amplification bias; duplicate-read clustering.
- Random-access primer selectivity, cross-talk, retrieval latency and coverage.
- Storage medium/encapsulation, environment, duration and recovery after ageing.
- Synthesis cost, sequencing cost, energy and accepted exact-payload throughput as separate terms.
- Exact payload hash match, unrecoverable fraction and held-out library performance.

## Data split and evidence gates

Partition by synthesis batch, oligo library, storage condition and sequencing run. Do not randomly split reads from the same oligo between training and test; this leaks molecule- and batch-specific signals.

Synthetic fixtures test parsing and decoder interfaces only. Published reproductions keep their original platform and conditions. Physical data requires raw bundles, specimen/library IDs, process lineage, instrument logs, calibration, sample sizes and uncertainty.

## Evidence boundary

The accompanying v0.1 schema and fixture are data contracts, not a DNA storage implementation. The synthetic reads are illustrative sequence edits, not measured error rates. No commercial DNA archive, net density, cost advantage, lifetime, read/write speed or physical reliability is claimed. The molecular medium remains research-only and must not be ranked as production-qualified by the compiler.
