# External scheduling benchmark audit data

Source dataset: Uddin, M. A. et al. (2026), *An expert-informed multi-scale synthetic benchmark dataset for calendar-aware resource-constrained project scheduling (CA-RCPSP)*, Data in Brief 69, 113256, DOI: https://doi.org/10.1016/j.dib.2026.113256.

Data DOI: https://doi.org/10.5281/zenodo.20553817

Upstream source: https://github.com/aftab-fr/carm-plus-dataset

The source dataset repository declares its dataset under Creative Commons Attribution 4.0 International (CC BY 4.0). This directory contains selected original JSON/CSV input bytes for a reproducible lower-bound audit, not a claim that the published solver results are valid. Original instance identities are preserved; any new computed findings are separate audit outputs.

Attribution: Md Aftab Uddin, Jaafar Gaber, Pierre Petitjean, and François Cortinovis (2026), as cited above. License terms: https://creativecommons.org/licenses/by/4.0/

Audit scope is deliberately narrow: compare each successful result's declared project makespan against the lower bound given by the longest required single task's duration, assuming both use the dataset's declared work-hour units. If the metric units or semantics differ, that discrepancy itself must be explained before the value is used for solver ranking.
