# Third-party attribution for MINE-ACP-001

This benchmark re-expresses the ACI and COP update steps in Rust using the published method descriptions and the following reference implementation:

- Repository: https://github.com/creator-xi/Conformal-Optimistic-Prediction
- Pinned upstream commit: https://github.com/creator-xi/Conformal-Optimistic-Prediction/commit/a41c0dcadc10979d8521ab9b5b8f3d460807d883
- Relevant source path: `core/methods.py`
- Paper: Hu, Wu, Xia & Zou, “Distribution-informed Online Conformal Prediction,” ICLR 2026: https://proceedings.iclr.cc/paper_files/paper/2026/hash/5f8241131e6fd428aa49914da76b8ad0-Abstract-Conference.html
- Upstream license: MIT, copyright (c) 2025 Junxi Wu.

The Rust implementation does not copy the upstream Python file. Its ACI/COP adapters intentionally use this benchmark's fixed synthetic calibration seed, a separately generated monitor channel, and maturity-time processing of delayed labels. Therefore these adapters are reference-derived variants, not claims of bit-for-bit reproduction or of the paper's theoretical guarantees. Any future substantial copied code from the upstream repository must retain the notice and full license below.

## Upstream MIT license

MIT License

Copyright (c) 2025 Junxi Wu

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
