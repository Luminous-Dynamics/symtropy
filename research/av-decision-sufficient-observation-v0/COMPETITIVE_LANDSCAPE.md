# Competitive Landscape: Evidence-Grounded Streaming and Active Observation

**Snapshot:** 2026-11-19.  
**Programme:** [AV-DECISION-SUFFICIENT-OBSERVATION-001 / #6222](https://github.com/Luminous-Dynamics/symtropy/issues/6222).  
**Purpose:** select fair external comparators and define what a credible improvement would mean.

This is a research comparison, not a claim that Symthaea currently matches or exceeds these systems. Published figures below belong to the cited paper/benchmark protocol; they are not directly interchangeable across datasets, hardware or metrics.

## The field is separating into several problems

The strongest recent systems do not simply process more frames. They specialize in retaining the right information, escalating retrieval only when needed, and controlling when an assistant emits a response.

| Work | Main contribution | Published reference point | What CEGI must test |
|---|---|---|---|
| HERMES (ACL 2026) | Training-free hierarchical KV-cache memory for streaming video | Authors report 10× faster time-to-first-token, up to 68% fewer video tokens than uniform sampling, and up to 11.4% gain on streaming datasets | Match quality with lower cache/token cost; compare exact same backbone and input schedule |
| StreamArena / StreamMind (2026) | Hour-scale open-ended interaction, historical recall, proactive monitoring and tools; two-tier frontend/backend workers | 243 videos averaging 88.8 min; 3,646 tasks. Public v2: StreamMind-Qwen RTP/HR/Tool/Pro 44.9/40.8/57.1/19.4; mixed multi-backbone 55.9/49.1/70.2/23.3. Paper reports 66.2% lower pooled query-to-answer latency than offline Qwen3.5-397B-A17B on that same backbone | Flagship long-horizon comparator; report model composition explicitly, preserve original AV evidence in memory, isolate deadlines from background load, score cancellation and as-known retrieval |
| StreamScout (arXiv 2609.00291) | Progressive retrieval depth: recent glance → uniform look-back → query-salient retrieval, with stop/escalate | StreamScout-S reports +14.65 OVO-Bench points over Qwen3-VL-8B with 59% fewer tokens and 1.04 s average answer time | Beat or match its quality-cost frontier while adding evidence lineage, decision-value accounting and historical source verification |
| OASIS (CVPR 2026) | Query-conditioned hierarchical event memory / temporal routing | On the paper's StreamingBench experiment, Qwen3-VL-8B Real-Time All: 72.83 → 78.22; Misleading Context Understanding: 35.74 → 49.60 | Reproduce matched backbone uplift, then test if bitemporal/proof-aware evidence packets improve late contradiction and source-grounding tasks |
| ProactiveBench / LiveProBench (arXiv 2609.12658) | One-second streaming evaluation without explicit response cue; response window, early/late, silence and duplicate behavior | Paper reports premature responses outnumber missed responses for four of six evaluated systems | Measure silence, false intervention, missed window and duplicate-trigger rates independently |
| SPOT-Bench (NeurIPS 2026 E&D) | Fully proactive multi-turn streaming benchmark; every prediction is scored with Timeliness-F1 | Public leaderboard reports Ctrl-SPOT baseline T-F1@5 = 20.9; JoyAI-VL-Interaction 16.3; MiniCPM-o 4.5 14.9 | Reproduce the official baseline, then improve timeliness without increasing unsolicited alerts or destroying recall |
| OmniInteract (EMNLP 2026) | Native online audio-video interaction with 250 videos and 1,430 temporally grounded response slots | Best reported overall IA-QTF1 = 0.368; best 1QnA = 0.052 | Raise continuous task/chain quality and response timing under native streaming, not offline clip QA |
| IPIBench (2026) | Interactive proactive intelligence under continuous streams: proactive monitoring, task management, interleaved reactive/proactive requests | 1,831 videos and 3,738 QA instances across three task families | Test request modification/cancellation, multi-task interference and user intent change |
| P3IBench (arXiv 2610.21568) | Daily perception, risk prediction and intervention in real-life video | 12 representative open/closed models; authors report poor calibration of response decisions | Evaluate the entire perception → prediction → intervention chain and calibrated restraint |
| DCASE 2026 Task 3 | Dense semantic acoustic maps from spatial audio and audiovisual inputs | Official best audio-only result: Macro mAP 0.1080, Pearson r 0.3962; AV track best Pearson r 0.4002 | Compare dense maps, spatial event coverage and uncertainty. Preserve official evaluator and splits |
| LOCATA | Moving sources and arrays with spatial tracking references | Dataset-specific localization/tracking metrics | Evaluate source-bearing error, track continuity and uncertainty under moving-array geometry |
| African ASR 2026 (PMLR) | Four ASR families across 13 African languages / 1–400 supervised hours | Architecture rankings depend on data regime | Build language, variety, code-switch and domain cohorts; do not claim multilingual strength from one pooled WER |

## Sources

- HERMES, ACL Anthology: https://aclanthology.org/2026.acl-long.381/
- StreamArena / StreamMind paper: https://arxiv.org/abs/2608.05703 ; public benchmark and v2 leaderboard: https://github.com/JIA-Lab-research/StreamArena
- StreamScout, arXiv: https://arxiv.org/abs/2609.00291
- OASIS, CVPR Open Access: https://openaccess.thecvf.com/content/CVPR2026/html/Liang_OASIS_On-Demand_Hierarchical_Event_Memory_for_Streaming_Video_Reasoning_CVPR_2026_paper.html
- LiveProBench / ProactiveBench, arXiv: https://arxiv.org/abs/2609.12658
- SPOT-Bench, paper: https://arxiv.org/abs/2604.24317 ; project/leaderboard: https://dibschat.github.io/SPOT-Bench/
- OmniInteract, ACL Anthology: https://aclanthology.org/2026.emnlp-main.490/
- IPIBench: https://lijinzhao30.github.io/IPIBench/
- P3IBench, arXiv: https://arxiv.org/abs/2610.21568
- DCASE 2026 Task 3 and results: https://dcase.community/challenge2026/task-semantic-acoustic-imaging-for-sound-event-localization-and-detection-from-spatial-audio-and-audiovisual-scenes
- DCASE 2026 challenge results index: https://dcase.community/challenge2026/index
- LOCATA: https://www.locata.lms.tf.fau.de/datasets/
- African ASR study, PMLR: https://proceedings.mlr.press/v302/nahabwe26a.html


## Additional interpretation notes

- **SPOT-Bench** is especially valuable because it scores every proactive output across the stream rather than only answering selected queries. Its published leaderboard reports Ctrl-SPOT at 20.9 T-F1@5, JoyAI-VL-Interaction at 16.3 and MiniCPM-o 4.5 at 14.9. The authors also identify “dead-time”: intervals in which no response is expected. Use this to evaluate restraint and resource scheduling, not just event recall.
- **LiveProBench / ProactiveBench** evaluates one-second stream intervals without an explicit response cue and separates premature, in-window and missed responses. Its finding that premature responses exceed missed responses for four of six evaluated systems makes false intervention a first-class metric.
- **IPIBench** extends proactive interaction beyond a fixed alert: user requests may be added, modified or cancelled while proactive monitoring and task management continue. This is relevant to long-lived assistants whose task intent can change during a stream.
- **DCASE Task 3** uses dense spatial acoustic maps (13 classes; 360×180 equirectangular output; 20° angular matching threshold for the matching metrics). The challenge reports 29 submissions from eight teams. Audio-only and audiovisual track values differ by metric; do not compare the best mAP from one track to the best Pearson correlation from another or assume that adding video always improves every score.


## StreamArena protocol details that matter for fair comparison

The 3,646 validated tasks are divided into four capability groups: Real-Time Perception (RTP), Historical Retrospection (HR), External Tool Use (Tool) and Proactive Interaction (Pro). Historical retrospection has evidence-to-query horizons of <=5, 5–15, 15–30 and >30 minutes; proactive interaction has monitoring horizons of <=30 seconds, 30 seconds–4 minutes and >4 minutes.

Do not cite the v2 mixed score as if one model achieved it: the public toolkit documents Gemini 3.5 Flash for Front/Router/Search/Recall/Reviewer, Kimi-K2.6 for Memory Writer and Qwen3.5 for Monitor. The v2 Qwen configuration is a separate row. Reproduce the exact mixture or use the Qwen row; never compare it as an apples-to-apples single-backbone result against native Symthaea without disclosing the difference.

The paper's 66.2% pooled query-to-answer latency reduction is a same-Qwen3.5-397B-A17B comparison between StreamMind and offline processing, but it excludes continuous stream processing completed before the query. Report both continuous monitoring/maintenance cost and query-triggered latency; query latency alone does not capture total resource use.

SPOT-Bench's public status also matters: detection tasks are released in full, while interaction/intervention include representative validation subsets and their held-out test challenge is scheduled for a later release. We can reproduce the live detection leaderboard now, but should not imply its full interaction/intervention held-out benchmark is publicly scoreable today.

## The paradigm shifts these results imply

### 0. From one streaming loop to deadline-isolated foreground and background cognition

StreamArena/StreamMind makes the strongest architecture comparison more concrete: a latency-critical frontend can handle immediate interaction/proactive monitoring while asynchronous backend workers build persistent memory, search and recall. Its benchmark explicitly tests hour-scale video and long-horizon retrospection, rather than relying on short clips.

For Symthaea, compose this with #5169 (floor/stop controller) and #3934/#10644/#9075/#9177 (visual and temporal memory) rather than making a second scheduler or memory engine. The research question becomes: can background memory consolidation and retrieval proceed without blocking a response deadline, while still returning the exact source interval and valid-time evidence behind a recalled claim?

Benchmark a foreground-only baseline, a shared single-loop baseline, and the composed foreground/background architecture at matched compute, memory and stream budgets. Measure deadline misses and interference, not just mean answer latency.

### 1. From retaining more context to retaining decision-sufficient context

HERMES, StreamScout and OASIS show that memory organization and retrieval policy can matter as much as backbone scale. CEGI's differentiator must be the *warrant* for using retrieved context: exact source intervals, transaction/valid-time distinction, dependence, source revision, and explicit support for the resulting claim.

### 2. From reactive correctness to continuous output discipline

SPOT-Bench and LiveProBench expose two distinct failure modes: missing an event window and speaking when no response is required. Half of SPOT-Bench video time is described as dead-time, where no response is expected. The goal cannot be “maximize event recall” alone. A system must balance coverage, timeliness, silence, duplicate avoidance and user-request state changes.

### 3. From entropy reduction to decision improvement

Information gain is not the objective when the task is a decision. EIG, EVSI and probability-free worst-case discrimination must remain separate. A model can lower uncertainty without changing the optimal decision, or obtain modest information that changes an important decision. Issue #6222 makes this a paired known-answer experiment.

### 4. From modality fusion to dependence-aware evidence association

The DCASE audiovisual track is not automatically superior to the audio-only track across all metrics. A modality should earn its place through held-out contribution, calibrated uncertainty and independence evidence. Two derived views from one capture path are not two independent sensors.

### 5. From one answer score to quality-cost-risk frontiers

Publish task quality, response timing, evidence grounding, abstention risk, bytes/tokens, latency, peak memory and measured energy separately. A composite can be shown as a secondary visualization only after hard integrity and permission gates pass.

## Proposed competitive targets

These are programme targets, **not achieved results**:

- HERMES: reproduce its published memory/token comparison on a pinned backbone and input schedule; target non-inferior task quality with at least 10% less retained context or cache cost than the reproduced implementation. Treat the paper's 68% token reduction as a reference to reproduce, not a directly portable guarantee.
- StreamScout: reproduce the reported OVO-Bench gain and cost reduction first. Then target a further 2-point improvement at the same or lower token budget, or a 10% cost reduction at non-inferior quality, on a held-out task subset. Declare the exact metric and split.
- OASIS: reproduce the paper's matched StreamingBench result. Then target at least +2 absolute points over the reproduced temporal-routing baseline on a pre-registered long-horizon / misleading-context subset, with paired confidence intervals.
- SPOT-Bench: reproduce Ctrl-SPOT T-F1@5 = 20.9 on the official protocol, then target 23 and later 25, while separately bounding false alarms, omissions and duplicate responses.
- StreamArena: reproduce the published baseline on the same pinned backbone and task subsets. Then target (a) at least offline-parity on historical retrospection and proactive interaction, (b) lower pooled query-to-answer latency without losing source-frame/audio provenance, and (c) robust request modification/cancellation on source-family-disjoint episodes. Compare StreamMind v1/v2 only under their documented model composition and evaluation conditions.
- OmniInteract: reproduce IA-QTF1 = 0.368 overall and 0.052 for 1QnA under the official protocol; initial targets 0.40 overall and 0.10 1QnA. Report component scores, not only aggregate F1.
- DCASE Task 3: reproduce the official baseline and best challenge references under the official evaluator; target top-three performance in one declared track. Do not mix audio-only and audiovisual leaderboard claims.
- South African ASR: establish per-language and per-domain reference results before setting improvement gates. Report code-switch boundaries and non-lexical acoustic perception separately from WER/CER.

The immediate target is not to claim every benchmark win. It is to reproduce exact baselines, construct a valid local paired challenge, and earn one independently reproducible advantage on a clearly named task.
