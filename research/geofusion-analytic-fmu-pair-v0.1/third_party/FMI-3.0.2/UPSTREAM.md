# FMI 3.0.2 C-API header provenance

The C-API headers in this directory are copied verbatim from the official Modelica Association FMI Standard repository.

- Repository: https://github.com/modelica/fmi-standard
- Release tag: `v3.0.2`
- FMI C-API header version returned by `fmi3GetVersion()`: `3.0` (the maintenance release suffix is not part of the C-header API version)
- Release commit: `b8778deaf5b746ba4a2f6c155d53f8a33eebbd33`
- Release note: maintenance release with clarifications and bug fixes; no new features compared with FMI 3.0.1.
- Header source blob SHAs:
  - `headers/fmi3Functions.h`: `062bb993257862eef5434098a91fa041e62aafd7`
  - `headers/fmi3FunctionTypes.h`: `8507f8060a2b384d96225535ab4ccf907157ec11`
  - `headers/fmi3PlatformTypes.h`: `d002977602f17216d11a5ec4f3b4e2cc42a89af1`
- Upstream `LICENSE.txt` is included unchanged. Header files carry the upstream 2-Clause BSD notices.

Do not replace these files with mutable `main` branch copies. Upgrade the release pin and all blob hashes together, with compatibility checks.
