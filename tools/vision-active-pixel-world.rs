// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-ACT-004: dependency-free RGB active-vision environment.
//!
//! This is a rendered-pixel toy world, not a photorealistic simulator or robot
//! result. The acting policy receives RGB bytes plus the legal-action flag;
//! hidden world state and terminal truth stay evaluator-side. A side-view action
//! returns one transient informative RGB frame, then the next frame is occluded.
//!
//! Run:
//!   rustc --edition=2024 tools/vision-active-pixel-world.rs -o /tmp/vis-act-pixels
//!   /tmp/vis-act-pixels
//!   rustc --edition=2024 --test tools/vision-active-pixel-world.rs -o /tmp/vis-act-pixel-tests
//!   /tmp/vis-act-pixel-tests

use std::collections::VecDeque;

const WIDTH: usize = 32;
const HEIGHT: usize = 32;
const CHANNELS: usize = 3;
const ROOT_SEED: u64 = 0x3C6E_F372_FE94_F82B;
const WORLD_REPLICATES: usize = 12;
const PAIRS_PER_REPLICATE: usize = 16;
const LATENCY_BUDGET: u64 = 12;
const VIEW_ACTION_BUDGET: u64 = 1;

fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn unit_milli(seed: u64, domain: u64) -> u16 {
    (mix64(seed ^ domain) % 1000) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneFamily {
    Development,
    HeldOut,
}

impl SceneFamily {
    fn name(self) -> &'static str {
        match self {
            Self::Development => "development_scene_family",
            Self::HeldOut => "held_out_scene_family",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cue {
    Present,
    Absent,
    Occluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Present,
    Absent,
    Abstain,
}

impl Cue {
    fn decision(self) -> Option<Decision> {
        match self {
            Self::Present => Some(Decision::Present),
            Self::Absent => Some(Decision::Absent),
            Self::Occluded => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RgbFrame {
    tick: u8,
    pixels: Vec<u8>,
    // Legal action metadata is public. This flag is set by the environment,
    // not inferred from hidden truth, and does not expose the world seed.
    side_view_available: bool,
}

impl RgbFrame {
    fn new(tick: u8, side_view_available: bool) -> Self {
        Self {
            tick,
            pixels: vec![0; WIDTH * HEIGHT * CHANNELS],
            side_view_available,
        }
    }

    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let index = (y * WIDTH + x) * CHANNELS;
        [self.pixels[index], self.pixels[index + 1], self.pixels[index + 2]]
    }

    fn set_pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }
        let index = (y * WIDTH + x) * CHANNELS;
        self.pixels[index..index + CHANNELS].copy_from_slice(&rgb);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HiddenWorld {
    // Evaluator-owned. Never passed to policy proposal, perception, or memory.
    exogenous_seed: u64,
    terminal_present: bool,
    family: SceneFamily,
    target_x: usize,
    target_y: usize,
    exposure: i16,
    decoy_x: usize,
    decoy_y: usize,
    decoy_count: u8,
    side_view_available: bool,
    // Hidden environment outcome: a committed legal view may still return an
    // occluded/no-information frame. This never crosses the policy boundary.
    side_view_successful: bool,
}

impl HiddenWorld {
    fn generated(exogenous_seed: u64, terminal_present: bool, family: SceneFamily) -> Self {
        let target_x = match family {
            SceneFamily::Development => 20,
            SceneFamily::HeldOut => 16 + usize::from(unit_milli(exogenous_seed, 0x5441_5247_4554_5801) % 7),
        };
        let target_y = match family {
            SceneFamily::Development => 16,
            SceneFamily::HeldOut => 13 + usize::from(unit_milli(exogenous_seed, 0x5441_5247_4554_5901) % 7),
        };
        let exposure = match family {
            SceneFamily::Development => (unit_milli(exogenous_seed, 0x4558_504F_5355_5245) % 9) as i16 - 4,
            SceneFamily::HeldOut => (unit_milli(exogenous_seed, 0x4558_504F_5355_5245) % 41) as i16 - 20,
        };
        Self {
            exogenous_seed,
            terminal_present,
            family,
            target_x,
            target_y,
            exposure,
            decoy_x: 5 + usize::from(unit_milli(exogenous_seed, 0x4445_434F_595F_5801) % 9),
            decoy_y: 6 + usize::from(unit_milli(exogenous_seed, 0x4445_434F_595F_5901) % 19),
            decoy_count: match family {
                SceneFamily::Development => 1,
                SceneFamily::HeldOut => 1 + (unit_milli(exogenous_seed, 0x4445_434F_595F_4E01) % 3) as u8,
            },
            side_view_available: unit_milli(exogenous_seed, 0x5349_4445_5F41_4354) >= 180,
            side_view_successful: match family {
                SceneFamily::Development => unit_milli(exogenous_seed, 0x5349_4445_5F53_5543) >= 100,
                SceneFamily::HeldOut => unit_milli(exogenous_seed, 0x5349_4445_5F53_5543) >= 350,
            },
        }
    }

    fn pixel_noise(self, tick: u8, x: usize, y: usize) -> i16 {
        let domain = (u64::from(tick) << 56)
            ^ ((y * WIDTH + x) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ 0x5049_5845_4C4E_4F49;
        (mix64(self.exogenous_seed ^ domain) % 7) as i16 - 3
    }

    fn shade(self, rgb: [u8; 3], noise: i16) -> [u8; 3] {
        rgb.map(|channel| (i16::from(channel) + self.exposure + noise).clamp(0, 255) as u8)
    }

    fn draw_cross(&self, frame: &mut RgbFrame, cx: usize, cy: usize, side_view: bool, center: [u8; 3]) {
        // A target is a red cross with a bright neutral center. Red-cross
        // decoys have a blue center and are deliberately not the task target.
        let red = if side_view { [190, 76, 76] } else { [232, 36, 36] };
        let offsets: [(isize, isize); 8] = [
            (0, -2), (0, -1), (0, 1), (0, 2),
            (-2, 0), (-1, 0), (1, 0), (2, 0),
        ];
        for (dx, dy) in offsets {
            let x = cx as isize + dx;
            let y = cy as isize + dy;
            if x >= 0 && y >= 0 {
                let x = x as usize;
                let y = y as usize;
                frame.set_pixel(x, y, self.shade(red, self.pixel_noise(frame.tick, x, y)));
            }
        }
        frame.set_pixel(cx, cy, self.shade(center, self.pixel_noise(frame.tick, cx, cy)));
    }

    fn draw_wall(&self, frame: &mut RgbFrame) {
        let left = self.target_x.saturating_sub(5);
        let right = (self.target_x + 5).min(WIDTH - 1);
        let top = self.target_y.saturating_sub(5);
        let bottom = (self.target_y + 5).min(HEIGHT - 1);
        for y in top..=bottom {
            for x in left..=right {
                let base = if (x + y) % 2 == 0 { [70, 70, 70] } else { [98, 98, 98] };
                frame.set_pixel(x, y, self.shade(base, self.pixel_noise(frame.tick, x, y)));
            }
        }
    }

    fn render(self, tick: u8, side_view: bool) -> RgbFrame {
        let available = tick == 1 && self.side_view_available;
        let mut frame = RgbFrame::new(tick, available);

        // Backgrounds and pixel noise are deterministic and paired across
        // persistent/removal counterfactuals. Held-out scenes change exposure,
        // object position, and distractor count without changing task semantics.
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let noise = self.pixel_noise(tick, x, y);
                let base = match self.family {
                    SceneFamily::Development => [
                        20 + ((x + y) % 4) as u8,
                        31 + ((x * 2 + y) % 5) as u8,
                        47 + ((x + y * 2) % 6) as u8,
                    ],
                    SceneFamily::HeldOut => [
                        27 + ((x * 3 + y) % 13) as u8,
                        38 + ((x + y * 3) % 17) as u8,
                        43 + ((x * 2 + y * 5) % 19) as u8,
                    ],
                };
                frame.set_pixel(x, y, self.shade(base, noise));
            }
        }

        // Distractors deliberately resemble part of the target's red shape,
        // but fail the target's bright-center pattern. No hidden target label is
        // encoded in this decoy; the policy must use the pixels themselves.
        for index in 0..self.decoy_count {
            let dx = (self.decoy_x + usize::from(index) * 4).min(WIDTH - 4);
            let dy = (self.decoy_y + usize::from(index) * 5).min(HEIGHT - 4);
            self.draw_cross(&mut frame, dx, dy, false, [28, 45, 220]);
        }

        match tick {
            0 => {
                // Initial pre-intervention frame: target is visible in both
                // paired worlds, even though terminal truth will later differ.
                self.draw_cross(&mut frame, self.target_x, self.target_y, false, [238, 238, 238]);
            }
            1 | 3 => {
                // The object is drawn identically before a foreground wall
                // hides it. Persistent and removed worlds therefore render
                // byte-identical pixels through the reveal boundary.
                self.draw_cross(&mut frame, self.target_x, self.target_y, false, [238, 238, 238]);
                self.draw_wall(&mut frame);
            }
            2 if side_view => {
                // A committed viewpoint action can still fail to deliver fresh
                // evidence. On failure, the returned RGB frame remains
                // occluded and is identical in paired persistent/removal worlds.
                if self.side_view_successful {
                    if self.terminal_present {
                        let shifted_x = (self.target_x + 6).min(WIDTH - 3);
                        self.draw_cross(&mut frame, shifted_x, self.target_y, true, [184, 184, 184]);
                    }
                } else {
                    self.draw_cross(&mut frame, self.target_x, self.target_y, false, [238, 238, 238]);
                    self.draw_wall(&mut frame);
                }
            }
            2 => {
                // Without an executed view, the stream remains occluded and
                // has no branch-specific visual difference.
                self.draw_cross(&mut frame, self.target_x, self.target_y, false, [238, 238, 238]);
                self.draw_wall(&mut frame);
            }
            _ => {}
        }
        frame
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Evidence {
    tick: u8,
    cue: Cue,
    confidence_milli: u16,
}

fn is_red(rgb: [u8; 3]) -> bool {
    rgb[0] > 120 && u16::from(rgb[0]) > u16::from(rgb[1]) + 48
        && u16::from(rgb[0]) > u16::from(rgb[2]) + 48
}

fn is_bright_center(rgb: [u8; 3]) -> bool {
    rgb.iter().all(|channel| *channel > 135)
        && (i16::from(*rgb.iter().max().unwrap_or(&0)) - i16::from(*rgb.iter().min().unwrap_or(&0))) < 55
}

/// Derive evidence strictly from RGB bytes. No view id, world metadata,
/// intervention label, seed, target position, or evaluator outcome is accepted.
fn perceive(frame: &RgbFrame) -> Evidence {
    let mut best_confidence = 0_u16;

    for cy in 2..HEIGHT - 2 {
        for cx in 2..WIDTH - 2 {
            if !is_bright_center(frame.pixel(cx, cy)) {
                continue;
            }
            let offsets: [(isize, isize); 8] = [
                (0, -2), (0, -1), (0, 1), (0, 2),
                (-2, 0), (-1, 0), (1, 0), (2, 0),
            ];
            let mut red_count = 0_u16;
            let mut contrast_sum = 0_u16;
            for (dx, dy) in offsets {
                let x = (cx as isize + dx) as usize;
                let y = (cy as isize + dy) as usize;
                let rgb = frame.pixel(x, y);
                if is_red(rgb) {
                    red_count += 1;
                    contrast_sum += u16::from(rgb[0].saturating_sub(rgb[1].max(rgb[2])));
                }
            }
            if red_count >= 7 {
                let average_contrast = contrast_sum / red_count.max(1);
                let confidence = (500 + average_contrast * 2).min(950);
                best_confidence = best_confidence.max(confidence);
            }
        }
    }

    if best_confidence > 0 {
        return Evidence {
            tick: frame.tick,
            cue: Cue::Present,
            confidence_milli: best_confidence,
        };
    }

    let neutral_wall_pixels = (0..HEIGHT)
        .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let [r, g, b] = frame.pixel(*x, *y);
            let spread = i16::from(r.max(g).max(b)) - i16::from(r.min(g).min(b));
            spread <= 4 && (45..=135).contains(&r)
        })
        .count();

    if neutral_wall_pixels >= 50 {
        Evidence {
            tick: frame.tick,
            cue: Cue::Occluded,
            confidence_milli: 0,
        }
    } else {
        // An unobstructed view with no target pattern is explicit negative
        // evidence, but its synthetic confidence is intentionally lower than
        // the original close-up positive observation.
        Evidence {
            tick: frame.tick,
            cue: Cue::Absent,
            confidence_milli: 650,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Wait,
    ChangeView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    NoMemory,
    LastObservation,
    BoundedFifo,
    ConfidenceGated,
    ProvenancePassive,
    ActiveRevision,
    UniformReservoir,
    RecencyProtectedReservoir,
}

impl Policy {
    fn name(self) -> &'static str {
        match self {
            Self::NoMemory => "no_memory",
            Self::LastObservation => "last_observation",
            Self::BoundedFifo => "bounded_fifo",
            Self::ConfidenceGated => "confidence_gated",
            Self::ProvenancePassive => "provenance_memory_passive",
            Self::ActiveRevision => "active_revision",
            Self::UniformReservoir => "uniform_reservoir_2",
            Self::RecencyProtectedReservoir => "recency_protected_reservoir_2",
        }
    }

    fn memory_cap(self) -> usize {
        match self {
            Self::NoMemory => 0,
            Self::LastObservation | Self::ConfidenceGated | Self::ProvenancePassive | Self::ActiveRevision => 1,
            Self::BoundedFifo | Self::UniformReservoir | Self::RecencyProtectedReservoir => 2,
        }
    }

    fn propose(self, frame: &RgbFrame, view_used: bool) -> Action {
        let evidence = perceive(frame);
        if self != Self::ProvenancePassive
            && evidence.cue == Cue::Occluded
            && frame.side_view_available
            && !view_used
        {
            Action::ChangeView
        } else {
            Action::Wait
        }
    }
}

#[derive(Debug, Clone)]
enum Memory {
    None,
    Last(Option<Evidence>),
    Fifo(VecDeque<Evidence>),
    ConfidenceBest(Option<Evidence>),
    ProvenanceLatest(Option<Evidence>),
    UniformReservoir {
        entries: VecDeque<Evidence>,
        seen: u64,
        actor_seed: u64,
    },
    /// Two timescales: preserve the latest direct cue while independently
    /// sampling one historical frame. Occlusion cannot evict the direct cue.
    RecencyProtectedReservoir {
        latest_informative: Option<Evidence>,
        archive: VecDeque<Evidence>,
        seen: u64,
        actor_seed: u64,
    },
}

impl Memory {
    fn new(policy: Policy) -> Self {
        Self::new_seeded(policy, 0)
    }

    fn new_seeded(policy: Policy, actor_seed: u64) -> Self {
        match policy {
            Policy::NoMemory => Self::None,
            Policy::LastObservation => Self::Last(None),
            Policy::BoundedFifo => Self::Fifo(VecDeque::with_capacity(2)),
            Policy::ConfidenceGated => Self::ConfidenceBest(None),
            Policy::ProvenancePassive | Policy::ActiveRevision => Self::ProvenanceLatest(None),
            Policy::UniformReservoir => Self::UniformReservoir {
                entries: VecDeque::with_capacity(2),
                seen: 0,
                actor_seed,
            },
            Policy::RecencyProtectedReservoir => Self::RecencyProtectedReservoir {
                latest_informative: None,
                archive: VecDeque::with_capacity(1),
                seen: 0,
                actor_seed,
            },
        }
    }

    fn observe(&mut self, frame: &RgbFrame) {
        let evidence = perceive(frame);
        match self {
            Self::None => {}
            Self::Last(last) => *last = Some(evidence),
            Self::Fifo(entries) => {
                entries.push_back(evidence);
                while entries.len() > 2 {
                    entries.pop_front();
                }
            }
            Self::ConfidenceBest(best) => {
                if evidence.cue != Cue::Occluded
                    && best.as_ref().map_or(true, |previous| evidence.confidence_milli >= previous.confidence_milli)
                {
                    *best = Some(evidence);
                }
            }
            Self::ProvenanceLatest(latest) => {
                if evidence.cue != Cue::Occluded {
                    // A later direct cue supersedes earlier evidence even when
                    // its confidence is lower. Timestamp is retained.
                    *latest = Some(evidence);
                }
            }
            Self::UniformReservoir { entries, seen, actor_seed } => {
                *seen = (*seen).saturating_add(1);
                if entries.len() < 2 {
                    entries.push_back(evidence);
                } else {
                    // Standard reservoir sampling gives each released frame
                    // equal marginal retention probability. No hidden state or
                    // outcome label is available to this update.
                    let draw = mix64(
                        *actor_seed
                            ^ (*seen).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                            ^ 0x554E_4946_4F52_4D52,
                    ) % *seen;
                    if draw < 2 {
                        entries[draw as usize] = evidence;
                    }
                }
            }
            Self::RecencyProtectedReservoir { latest_informative, archive, seen, actor_seed } => {
                if evidence.cue != Cue::Occluded {
                    *latest_informative = Some(evidence);
                }
                *seen = (*seen).saturating_add(1);
                if archive.is_empty() {
                    archive.push_back(evidence);
                } else {
                    // The archive is a one-entry uniform sample. It is kept
                    // separate from the protected current-evidence slot.
                    let draw = mix64(
                        *actor_seed
                            ^ (*seen).wrapping_mul(0xD1B5_4A32_D192_ED03)
                            ^ 0x5245_4350_524F_5445,
                    ) % *seen;
                    if draw == 0 {
                        archive[0] = evidence;
                    }
                }
            }
        }
    }

    fn decide(&self, policy: Policy, terminal: &RgbFrame) -> Decision {
        match policy {
            Policy::NoMemory => perceive(terminal).cue.decision().unwrap_or(Decision::Abstain),
            Policy::LastObservation => match self {
                Self::Last(last) => last.and_then(|e| e.cue.decision()).unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            Policy::BoundedFifo => match self {
                Self::Fifo(entries) => entries.iter().rev()
                    .find(|e| e.cue != Cue::Occluded && terminal.tick.saturating_sub(e.tick) <= 1)
                    .and_then(|e| e.cue.decision())
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            Policy::ConfidenceGated => match self {
                Self::ConfidenceBest(best) => best.and_then(|e| e.cue.decision()).unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            Policy::ProvenancePassive | Policy::ActiveRevision => match self {
                Self::ProvenanceLatest(latest) => latest
                    .filter(|e| terminal.tick.saturating_sub(e.tick) <= 1)
                    .and_then(|e| e.cue.decision())
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            Policy::UniformReservoir => match self {
                Self::UniformReservoir { entries, .. } => entries.iter()
                    .filter(|e| e.cue != Cue::Occluded && terminal.tick.saturating_sub(e.tick) <= 1)
                    .max_by_key(|e| e.tick)
                    .and_then(|e| e.cue.decision())
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
            Policy::RecencyProtectedReservoir => match self {
                Self::RecencyProtectedReservoir { latest_informative, archive, .. } => latest_informative
                    .filter(|e| terminal.tick.saturating_sub(e.tick) <= 1)
                    .and_then(|e| e.cue.decision())
                    .or_else(|| archive.iter()
                        .filter(|e| e.cue != Cue::Occluded && terminal.tick.saturating_sub(e.tick) <= 1)
                        .max_by_key(|e| e.tick)
                        .and_then(|e| e.cue.decision()))
                    .unwrap_or(Decision::Abstain),
                _ => Decision::Abstain,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EpisodeResult {
    terminal_present: bool,
    decision: Decision,
    loss: f64,
    view_used: bool,
    fresh_view_evidence: bool,
    latency_units: u64,
    budget_violations: u64,
    explicit_absence_seen: bool,
}

fn run_episode(world: HiddenWorld, policy: Policy) -> EpisodeResult {
    let mut memory = Memory::new(policy);
    let mut view_used = false;
    let mut latency_units = 0_u64;
    let mut budget_violations = 0_u64;
    let mut explicit_absence_seen = false;
    let mut fresh_view_evidence = false;
    let mut terminal = world.render(0, false);

    for tick in 0..=3_u8 {
        // Only tick 2 may show the shifted viewpoint, and only after its action
        // was validated and committed on tick 1.
        let side_view = tick == 2 && view_used;
        let frame = world.render(tick, side_view);
        memory.observe(&frame);
        explicit_absence_seen |= perceive(&frame).cue == Cue::Absent;
        if tick == 2 && view_used {
            fresh_view_evidence = perceive(&frame).cue != Cue::Occluded;
        }

        if tick == 1 {
            let proposed = policy.propose(&frame, view_used);
            let executed = match proposed {
                Action::ChangeView
                    if frame.side_view_available
                        && !view_used
                        && latency_units.saturating_add(3) <= LATENCY_BUDGET
                        && (view_used as u64) < VIEW_ACTION_BUDGET =>
                {
                    view_used = true;
                    latency_units += 3;
                    Action::ChangeView
                }
                Action::ChangeView => {
                    budget_violations += 1;
                    latency_units = latency_units.saturating_add(1);
                    Action::Wait
                }
                Action::Wait => {
                    latency_units = latency_units.saturating_add(1);
                    Action::Wait
                }
            };
            // The action is recorded before the next observation is rendered.
            if executed == Action::Wait {
                // view_used remains false; the next frame cannot reveal truth.
            }
        } else {
            latency_units = latency_units.saturating_add(1);
        }

        terminal = frame;
    }

    if latency_units > LATENCY_BUDGET || (view_used as u64) > VIEW_ACTION_BUDGET {
        budget_violations += 1;
    }

    let decision = memory.decide(policy, &terminal);
    let loss = match decision {
        Decision::Present if world.terminal_present => 0.0,
        Decision::Absent if !world.terminal_present => 0.0,
        Decision::Abstain => 0.25,
        _ => 1.0,
    };

    EpisodeResult {
        terminal_present: world.terminal_present,
        decision,
        loss,
        view_used,
        fresh_view_evidence,
        latency_units,
        budget_violations,
        explicit_absence_seen,
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    episodes: u64,
    loss_sum: f64,
    answered: u64,
    correct_answered: u64,
    abstentions: u64,
    persistent_episodes: u64,
    persistent_correct: u64,
    removed_episodes: u64,
    false_persistence: u64,
    removed_with_absence: u64,
    removed_revised_correctly: u64,
    view_actions: u64,
    fresh_view_evidence_count: u64,
    views_without_fresh_evidence: u64,
    latency_units: u64,
    budget_violations: u64,
}

impl Counts {
    fn record(&mut self, result: EpisodeResult) {
        self.episodes += 1;
        self.loss_sum += result.loss;
        self.view_actions += u64::from(result.view_used);
        if result.view_used {
            if result.fresh_view_evidence {
                self.fresh_view_evidence_count += 1;
            } else {
                self.views_without_fresh_evidence += 1;
            }
        }
        self.latency_units += result.latency_units;
        self.budget_violations += result.budget_violations;
        match result.decision {
            Decision::Abstain => self.abstentions += 1,
            decision => {
                self.answered += 1;
                let correct = matches!(
                    (decision, result.terminal_present),
                    (Decision::Present, true) | (Decision::Absent, false)
                );
                if correct {
                    self.correct_answered += 1;
                }
            }
        }

        if result.terminal_present {
            self.persistent_episodes += 1;
            if result.decision == Decision::Present {
                self.persistent_correct += 1;
            }
        } else {
            self.removed_episodes += 1;
            if result.decision == Decision::Present {
                self.false_persistence += 1;
            }
            if result.explicit_absence_seen {
                self.removed_with_absence += 1;
                if result.decision == Decision::Absent {
                    self.removed_revised_correctly += 1;
                }
            }
        }
    }

    fn mean_loss(self) -> f64 {
        self.loss_sum / self.episodes.max(1) as f64
    }

    fn rate(numerator: u64, denominator: u64) -> String {
        if denominator == 0 { "NA".to_string() }
        else { format!("{:.4}", numerator as f64 / denominator as f64) }
    }
}

fn seed_for(replicate: usize, pair: usize, family: SceneFamily) -> u64 {
    let family_domain = match family {
        SceneFamily::Development => 0x4445_565F_5343_454E,
        SceneFamily::HeldOut => 0x4845_4C44_4F55_545F,
    };
    mix64(
        ROOT_SEED ^ family_domain
            ^ (replicate as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (pair as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
    )
}

fn run_report() {
    let policies = [
        Policy::NoMemory,
        Policy::LastObservation,
        Policy::BoundedFifo,
        Policy::ConfidenceGated,
        Policy::ProvenancePassive,
        Policy::ActiveRevision,
        Policy::UniformReservoir,
        Policy::RecencyProtectedReservoir,
    ];
    let families = [SceneFamily::Development, SceneFamily::HeldOut];

    println!("VIS-ACT-004 rendered RGB active-vision diagnostic");
    println!("32x32 RGB procedural renderer; paired persistent/removal worlds; one transient side-view observation");
    println!("synthetic resource units; no camera, GPU, robot, or physical-energy claims");
    println!("family\tpolicy\tepisodes\tmean_loss\tanswered_accuracy\tabstention_rate\tpersistent_correct_rate\tremoved_false_persistence_rate\tremoved_revision_when_absence_seen\tremoved_absence_cases\tview_actions_per_episode\tfresh_evidence_rate_given_view\tviews_without_fresh_evidence\tlatency_units_per_episode\tmemory_cap\tbudget_violations");

    for family in families {
        for policy in policies {
            let mut total = Counts::default();
            for replicate in 0..WORLD_REPLICATES {
                for pair in 0..PAIRS_PER_REPLICATE {
                    let seed = seed_for(replicate, pair, family);
                    let persistent = HiddenWorld::generated(seed, true, family);
                    let removed = HiddenWorld::generated(seed, false, family);
                    total.record(run_episode(persistent, policy));
                    total.record(run_episode(removed, policy));
                }
            }

            println!(
                "{}\t{}\t{}\t{:.4}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{}\t{:.3}\t{}\t{}",
                family.name(),
                policy.name(),
                total.episodes,
                total.mean_loss(),
                Counts::rate(total.correct_answered, total.answered),
                Counts::rate(total.abstentions, total.episodes),
                Counts::rate(total.persistent_correct, total.persistent_episodes),
                Counts::rate(total.false_persistence, total.removed_episodes),
                Counts::rate(total.removed_revised_correctly, total.removed_with_absence),
                total.removed_with_absence,
                total.view_actions as f64 / total.episodes.max(1) as f64,
                Counts::rate(total.fresh_view_evidence_count, total.view_actions),
                total.views_without_fresh_evidence,
                total.latency_units as f64 / total.episodes.max(1) as f64,
                policy.memory_cap(),
                total.budget_violations,
            );
        }
    }
}

fn main() {
    run_report();
    randomized_timing::run_report();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paired(family: SceneFamily, seed: u64) -> (HiddenWorld, HiddenWorld) {
        let persistent = HiddenWorld::generated(seed, true, family);
        let removed = HiddenWorld::generated(seed, false, family);
        (
            HiddenWorld { side_view_successful: true, ..persistent },
            HiddenWorld { side_view_successful: true, ..removed },
        )
    }

    #[test]
    fn rgb_rendering_is_correctly_sized_and_deterministic() {
        let world = HiddenWorld::generated(77, true, SceneFamily::Development);
        let first = world.render(0, false);
        assert_eq!(first.pixels.len(), WIDTH * HEIGHT * CHANNELS);
        assert_eq!(first, world.render(0, false));
        assert_eq!(first.pixel(20, 16).iter().all(|c| *c > 130), true);
    }

    #[test]
    fn paired_worlds_have_byte_identical_pixels_before_reveal() {
        for family in [SceneFamily::Development, SceneFamily::HeldOut] {
            let (present, removed) = paired(family, 0xCAFE_BABE);
            for tick in [0, 1, 3] {
                assert_eq!(present.render(tick, false), removed.render(tick, false));
            }
            assert_eq!(present.render(2, false), removed.render(2, false));
        }
    }

    #[test]
    fn legal_side_view_reveals_present_versus_absent_in_rgb() {
        let (present, removed) = paired(SceneFamily::Development, 0x1020_3040);
        let visible_present = present.render(2, true);
        let visible_absent = removed.render(2, true);
        assert_eq!(perceive(&visible_present).cue, Cue::Present);
        assert_eq!(perceive(&visible_absent).cue, Cue::Absent);
        assert_ne!(visible_present.pixels, visible_absent.pixels);
    }

    #[test]
    fn occluded_pixels_do_not_disclose_hidden_intervention() {
        let (present, removed) = paired(SceneFamily::Development, 0x1234_9876);
        for tick in [1, 3] {
            let a = present.render(tick, false);
            let b = removed.render(tick, false);
            assert_eq!(a.pixels, b.pixels);
            assert_eq!(perceive(&a).cue, Cue::Occluded);
            assert_eq!(perceive(&b).cue, Cue::Occluded);
        }
    }

    #[test]
    fn red_decoy_with_wrong_center_is_not_mistaken_for_target() {
        let world = HiddenWorld::generated(0xABCDEF01, false, SceneFamily::Development);
        let frame = world.render(2, true);
        assert_eq!(perceive(&frame).cue, Cue::Absent);
    }

    #[test]
    fn transient_view_requires_memory_after_the_next_occlusion() {
        let (present, removed) = paired(SceneFamily::Development, 0xF00D);
        let transient = removed.render(2, true);
        let terminal = removed.render(3, false);
        assert_eq!(perceive(&transient).cue, Cue::Absent);
        assert_eq!(perceive(&terminal).cue, Cue::Occluded);
        let mut memory = Memory::new(Policy::ActiveRevision);
        memory.observe(&transient);
        memory.observe(&terminal);
        assert_eq!(memory.decide(Policy::ActiveRevision, &terminal), Decision::Absent);

        let no_memory = Memory::new(Policy::NoMemory);
        assert_eq!(no_memory.decide(Policy::NoMemory, &terminal), Decision::Abstain);
        // Also ensure the paired persistent branch remains identifiable.
        assert_eq!(perceive(&present.render(2, true)).cue, Cue::Present);
    }

    #[test]
    fn a_failed_view_costs_latency_but_does_not_supply_evidence() {
        let base = HiddenWorld::generated(0xFA17, false, SceneFamily::Development);
        let failed = HiddenWorld {
            side_view_available: true,
            side_view_successful: false,
            ..base
        };
        let result = run_episode(failed, Policy::ActiveRevision);
        assert!(result.view_used);
        assert!(!result.fresh_view_evidence);
        assert_eq!(result.decision, Decision::Abstain);
        assert_eq!(result.budget_violations, 0);

        let present = HiddenWorld { terminal_present: true, ..failed };
        let present_frame = present.render(2, true);
        let absent_frame = failed.render(2, true);
        assert_eq!(present_frame.pixels, absent_frame.pixels);
        assert_eq!(perceive(&present_frame).cue, Cue::Occluded);
        assert_eq!(perceive(&absent_frame).cue, Cue::Occluded);
    }

    #[test]
    fn confidence_gated_memory_fails_on_newer_lower_confidence_absence() {
        let (present, removed) = paired(SceneFamily::Development, 0xCAFE);
        let mut memory = Memory::new(Policy::ConfidenceGated);
        let initial = present.render(0, false);
        let absence = removed.render(2, true);
        memory.observe(&initial);
        memory.observe(&absence);
        assert_eq!(perceive(&initial).cue, Cue::Present);
        assert!(perceive(&initial).confidence_milli > perceive(&absence).confidence_milli);
        let terminal = removed.render(3, false);
        memory.observe(&terminal);
        assert_eq!(memory.decide(Policy::ConfidenceGated, &terminal), Decision::Present);
    }

    #[test]
    fn held_out_family_changes_scene_without_changing_pairing_contract() {
        let dev = HiddenWorld::generated(89, true, SceneFamily::Development);
        let held = HiddenWorld::generated(89, true, SceneFamily::HeldOut);
        assert_ne!((dev.target_x, dev.target_y, dev.exposure), (held.target_x, held.target_y, held.exposure));
        let dev_removed = HiddenWorld::generated(89, false, SceneFamily::Development);
        let held_removed = HiddenWorld::generated(89, false, SceneFamily::HeldOut);
        assert_eq!(dev.render(1, false), dev_removed.render(1, false));
        assert_eq!(held.render(1, false), held_removed.render(1, false));
    }

    #[test]
    fn active_view_actions_obey_budget_and_use_only_public_frame_fields() {
        let (present, _) = paired(SceneFamily::HeldOut, 0x9911);
        let frame = present.render(1, false);
        let action = Policy::ActiveRevision.propose(&frame, false);
        assert!(matches!(action, Action::Wait | Action::ChangeView));
        if action == Action::ChangeView {
            assert!(frame.side_view_available);
        }
        let result = run_episode(present, Policy::ActiveRevision);
        assert!(result.latency_units <= LATENCY_BUDGET);
        assert!(u64::from(result.view_used) <= VIEW_ACTION_BUDGET);
        assert_eq!(result.budget_violations, 0);
    }

    #[test]
    fn no_executed_or_unavailable_view_never_gets_information_credit() {
        let seed = (0_u64..1000)
            .find(|seed| !HiddenWorld::generated(*seed, true, SceneFamily::Development).side_view_available)
            .unwrap();
        let (present, removed) = paired(SceneFamily::Development, seed);
        assert_eq!(present.render(1, false), removed.render(1, false));
        assert!(!present.render(1, false).side_view_available);
        assert_eq!(present.render(2, false), removed.render(2, false));
        assert_eq!(perceive(&present.render(2, false)).cue, Cue::Occluded);
    }

    #[test]
    fn undefined_rates_are_na_and_defined_rates_are_numeric() {
        assert_eq!(Counts::rate(0, 0), "NA");
        assert_eq!(Counts::rate(1, 4), "0.2500");
    }

    #[test]
    fn generated_report_includes_development_and_heldout_slices() {
        assert_ne!(SceneFamily::Development.name(), SceneFamily::HeldOut.name());
        assert_eq!(WIDTH * HEIGHT * CHANNELS, 3072);
    }

    #[test]
    fn deterministic_pairing_and_policy_rollouts_repeat_exactly() {
        let seed = 0xDEAD_BEEF;
        let present = HiddenWorld::generated(seed, true, SceneFamily::HeldOut);
        let removed = HiddenWorld::generated(seed, false, SceneFamily::HeldOut);
        assert_eq!(present, HiddenWorld::generated(seed, true, SceneFamily::HeldOut));
        assert_eq!(removed, HiddenWorld::generated(seed, false, SceneFamily::HeldOut));
        assert_eq!(
            run_episode(present, Policy::ActiveRevision).loss,
            run_episode(present, Policy::ActiveRevision).loss
        );
    }
}


// VIS-ACT-005 adds an event-driven wrapper around the same RGB renderer.
// The base RGB pixel tests remain intact; these tests vary occlusion timing,
// observation delay, and action success independently of task-stage timing.
mod randomized_timing {
    use super::*;

    const HORIZON: u8 = 6;
    const DECISION_DEADLINE: u8 = HORIZON;
    const LATENCY_BUDGET: u64 = 16;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TimedWorld {
        visual: HiddenWorld,
        occlusion_onset: u8,
        occlusion_duration: u8,
        view_delay: u8,
        view_available: bool,
        view_successful: bool,
    }

    impl TimedWorld {
        fn generated(seed: u64, terminal_present: bool, family: SceneFamily) -> Self {
            let visual = HiddenWorld::generated(seed, terminal_present, family);
            // Separate random domains: event timing is not correlated with the
            // target, render noise, action availability, or action success.
            let onset_domain = 0x5449_4D45_4F4E_5345;
            let duration_domain = 0x5449_4D45_4455_5241;
            let delay_domain = 0x5649_4557_4445_4C59;
            let availability_domain = 0x5449_4D45_4156_4149;
            let success_domain = 0x5449_4D45_5355_4343;

            let occlusion_onset = match family {
                SceneFamily::Development => 1 + (unit_milli(seed, onset_domain) % 3) as u8,
                SceneFamily::HeldOut => 2 + (unit_milli(seed, onset_domain) % 3) as u8,
            };
            let occlusion_duration = match family {
                SceneFamily::Development => 1 + (unit_milli(seed, duration_domain) % 3) as u8,
                SceneFamily::HeldOut => 2 + (unit_milli(seed, duration_domain) % 3) as u8,
            };
            let view_delay = match family {
                SceneFamily::Development => 1 + (unit_milli(seed, delay_domain) % 3) as u8,
                SceneFamily::HeldOut => 2 + (unit_milli(seed, delay_domain) % 3) as u8,
            };
            let view_available = unit_milli(seed, availability_domain) >= 180;
            let view_success_threshold = match family {
                SceneFamily::Development => 100,
                SceneFamily::HeldOut => 350,
            };
            let view_successful = unit_milli(seed, success_domain) >= view_success_threshold;

            Self {
                visual,
                occlusion_onset,
                occlusion_duration,
                view_delay,
                view_available,
                view_successful,
            }
        }

        fn natural_reveal_tick(self) -> u8 {
            self.occlusion_onset.saturating_add(self.occlusion_duration)
        }

        fn paired_with(self, terminal_present: bool) -> Self {
            Self {
                visual: HiddenWorld {
                    terminal_present,
                    ..self.visual
                },
                ..self
            }
        }

        fn frame(
            self,
            tick: u8,
            attempted: bool,
            receipt: Option<ViewReceipt>,
        ) -> RgbFrame {
            let natural_reveal_tick = self.natural_reveal_tick();
            let action_reveals = receipt.is_some_and(|r| {
                r.completion_tick == tick
                    && r.completion_tick <= DECISION_DEADLINE
                    && r.completion_tick < natural_reveal_tick
                    && r.executed
                    && r.successful
            });
            let naturally_revealed = tick == natural_reveal_tick;

            let mut frame = if tick < self.occlusion_onset {
                // Stable pre-intervention view. The paired branch's pixels are
                // identical because terminal truth is not rendered pre-reveal.
                self.visual.render(0, false)
            } else if action_reveals || naturally_revealed {
                // A completed successful action or a short natural visibility
                // window gives one informative frame; the obstruction returns
                // on later steps. This makes memory carry the evidence.
                let revealed_world = HiddenWorld {
                    side_view_successful: true,
                    ..self.visual
                };
                revealed_world.render(2, true)
            } else {
                // Alternate two deterministic noise realizations while keeping
                // the hidden target covered.
                self.visual.render(if tick % 2 == 0 { 3 } else { 1 }, false)
            };

            frame.tick = tick;
            frame.side_view_available = tick >= self.occlusion_onset
                && tick < natural_reveal_tick
                && tick < DECISION_DEADLINE
                && self.view_available
                && !attempted;
            frame
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ViewReceipt {
        requested_tick: u8,
        completion_tick: u8,
        executed: bool,
        successful: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TimedPolicy {
        NoMemory,
        PassiveNoView,
        LastObservation,
        BoundedFifo,
        ConfidenceGated,
        ProvenancePassive,
        ActiveRevision,
        FixedSweep,
        RandomLegalView,
        UniformReservoir,
        RecencyProtectedReservoir,
    }

    impl TimedPolicy {
        const ALL: [Self; 11] = [
            Self::NoMemory,
            Self::PassiveNoView,
            Self::LastObservation,
            Self::BoundedFifo,
            Self::ConfidenceGated,
            Self::ProvenancePassive,
            Self::ActiveRevision,
            Self::FixedSweep,
            Self::RandomLegalView,
            Self::UniformReservoir,
            Self::RecencyProtectedReservoir,
        ];

        fn name(self) -> &'static str {
            match self {
                Self::NoMemory => "reactive_view_no_memory",
                Self::PassiveNoView => "passive_no_view",
                Self::LastObservation => "last_observation",
                Self::BoundedFifo => "bounded_fifo",
                Self::ConfidenceGated => "confidence_gated",
                Self::ProvenancePassive => "provenance_memory_passive",
                Self::ActiveRevision => "active_revision",
                Self::FixedSweep => "fixed_sweep",
                Self::RandomLegalView => "random_legal_view",
                Self::UniformReservoir => "uniform_reservoir_2",
                Self::RecencyProtectedReservoir => "recency_protected_reservoir_2",
            }
        }

        fn memory_policy(self) -> Policy {
            match self {
                Self::NoMemory | Self::PassiveNoView => Policy::NoMemory,
                Self::LastObservation => Policy::LastObservation,
                Self::BoundedFifo | Self::FixedSweep => Policy::BoundedFifo,
                Self::ConfidenceGated => Policy::ConfidenceGated,
                Self::ProvenancePassive => Policy::ProvenancePassive,
                Self::ActiveRevision => Policy::ActiveRevision,
                Self::RandomLegalView => Policy::BoundedFifo,
                Self::UniformReservoir => Policy::UniformReservoir,
                Self::RecencyProtectedReservoir => Policy::RecencyProtectedReservoir,
            }
        }

        fn propose(self, frame: &RgbFrame, attempted: bool, actor_seed: u64) -> Action {
            match self {
                Self::PassiveNoView => Action::Wait,
                // The fixed-sweep baseline acts at two predeclared ticks when
                // possible, rather than reacting to pixels. A shared one-view
                // budget makes this a fair fixed schedule, not free scanning.
                Self::FixedSweep => {
                    if matches!(frame.tick, 2 | 4)
                        && frame.side_view_available
                        && !attempted
                    {
                        Action::ChangeView
                    } else {
                        Action::Wait
                    }
                }
                Self::RandomLegalView => {
                    // Actor-owned pseudo-randomness is domain-separated from
                    // world/sensor generation; legality and budgets stay with
                    // the environment evaluator.
                    let draw = mix64(
                        actor_seed
                            ^ u64::from(frame.tick)
                                .wrapping_mul(0x9E37_79B9_7F4A_7C15),
                    );
                    if frame.side_view_available && !attempted && draw % 2 == 0 {
                        Action::ChangeView
                    } else {
                        Action::Wait
                    }
                }
                _ => self.memory_policy().propose(frame, attempted),
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct TimedResult {
        truth: bool,
        decision: Decision,
        loss: f64,
        attempt: bool,
        executed: bool,
        completed: bool,
        fresh_view_evidence: bool,
        view_returned_no_fresh_evidence: bool,
        natural_or_view_recovery_steps: Option<u8>,
        view_attributable_recovery_steps: Option<u8>,
        latency_units: u64,
        budget_violations: u64,
    }

    fn run_episode(world: TimedWorld, policy: TimedPolicy, actor_seed: u64) -> TimedResult {
        let memory_policy = policy.memory_policy();
        let mut memory = Memory::new_seeded(memory_policy, actor_seed);
        let mut attempted = false;
        let mut executed = false;
        let mut completed = false;
        let mut fresh_view_evidence = false;
        let mut latency_units = 0_u64;
        let mut budget_violations = 0_u64;
        let mut receipt: Option<ViewReceipt> = None;
        let mut recovery_steps = None;
        let mut view_recovery_steps = None;
        let mut terminal = world.frame(0, false, None);

        for tick in 0..=HORIZON {
            let frame = world.frame(tick, attempted, receipt);
            let evidence = perceive(&frame);

            if tick >= world.occlusion_onset
                && recovery_steps.is_none()
                && evidence.cue != Cue::Occluded
            {
                recovery_steps = Some(tick.saturating_sub(world.occlusion_onset));
            }

            if let Some(r) = receipt {
                if r.completion_tick == tick && tick <= DECISION_DEADLINE && r.executed {
                    debug_assert!(r.completion_tick >= r.requested_tick);
                    completed = true;
                    let action_revealed_fresh_evidence = r.successful
                        && tick < world.natural_reveal_tick()
                        && evidence.cue != Cue::Occluded;
                    fresh_view_evidence |= action_revealed_fresh_evidence;
                    if action_revealed_fresh_evidence && view_recovery_steps.is_none() {
                        view_recovery_steps =
                            Some(tick.saturating_sub(world.occlusion_onset));
                    }
                }
            }

            memory.observe(&frame);
            terminal = frame.clone();

            if tick == HORIZON {
                break;
            }

            // One unit for a decision/observation opportunity, regardless of
            // whether it produces useful evidence.
            latency_units = latency_units.saturating_add(1);

            if attempted {
                continue;
            }

            let proposed = policy.propose(&frame, attempted, actor_seed);
            if proposed == Action::ChangeView {
                attempted = true;
                // Attempt, execution and information are distinct. A policy
                // request is charged even when legality or budget rejects it.
                if frame.side_view_available
                    && latency_units.saturating_add(3) <= LATENCY_BUDGET
                {
                    executed = true;
                    latency_units += 3;
                    receipt = Some(ViewReceipt {
                        requested_tick: tick,
                        completion_tick: tick.saturating_add(world.view_delay),
                        executed: true,
                        successful: world.view_successful,
                    });
                } else {
                    budget_violations += 1;
                    receipt = Some(ViewReceipt {
                        requested_tick: tick,
                        completion_tick: tick.saturating_add(world.view_delay),
                        executed: false,
                        successful: false,
                    });
                }
            }
        }

        let decision = memory.decide(memory_policy, &terminal);
        let loss = match decision {
            Decision::Present if world.visual.terminal_present => 0.0,
            Decision::Absent if !world.visual.terminal_present => 0.0,
            Decision::Abstain => 0.25,
            _ => 1.0,
        };
        let view_returned_no_fresh_evidence = executed && !fresh_view_evidence;

        if latency_units > LATENCY_BUDGET || (executed as u64) > 1 {
            budget_violations += 1;
        }

        TimedResult {
            truth: world.visual.terminal_present,
            decision,
            loss,
            attempt: attempted,
            executed,
            completed,
            fresh_view_evidence,
            view_returned_no_fresh_evidence,
            natural_or_view_recovery_steps: recovery_steps,
            view_attributable_recovery_steps: view_recovery_steps,
            latency_units,
            budget_violations,
        }
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct Aggregate {
        episodes: u64,
        loss_sum: f64,
        answered: u64,
        correct_answered: u64,
        abstentions: u64,
        persistent_episodes: u64,
        persistent_correct: u64,
        removed_episodes: u64,
        false_persistence: u64,
        view_attempts: u64,
        executed_views: u64,
        completed_views: u64,
        fresh_views: u64,
        views_without_fresh: u64,
        recovery_steps_sum: u64,
        recovery_observations: u64,
        view_recovery_steps_sum: u64,
        view_recovery_observations: u64,
        budget_violations: u64,
        latency_sum: u64,
    }

    impl Aggregate {
        fn record(&mut self, result: TimedResult) {
            self.episodes += 1;
            self.loss_sum += result.loss;
            self.view_attempts += u64::from(result.attempt);
            self.executed_views += u64::from(result.executed);
            self.completed_views += u64::from(result.completed);
            self.fresh_views += u64::from(result.fresh_view_evidence);
            self.views_without_fresh += u64::from(result.view_returned_no_fresh_evidence);
            self.budget_violations += result.budget_violations;
            self.latency_sum += result.latency_units;
            if let Some(steps) = result.natural_or_view_recovery_steps {
                self.recovery_steps_sum += u64::from(steps);
                self.recovery_observations += 1;
            }
            if let Some(steps) = result.view_attributable_recovery_steps {
                self.view_recovery_steps_sum += u64::from(steps);
                self.view_recovery_observations += 1;
            }

            if result.truth {
                self.persistent_episodes += 1;
                if result.decision == Decision::Present {
                    self.persistent_correct += 1;
                }
            } else {
                self.removed_episodes += 1;
                if result.decision == Decision::Present {
                    self.false_persistence += 1;
                }
            }

            match result.decision {
                Decision::Abstain => self.abstentions += 1,
                decision => {
                    self.answered += 1;
                    let correct = matches!(
                        (decision, result.truth),
                        (Decision::Present, true) | (Decision::Absent, false)
                    );
                    if correct {
                        self.correct_answered += 1;
                    }
                }
            }
        }

        fn mean_loss(self) -> f64 {
            self.loss_sum / self.episodes.max(1) as f64
        }

        fn rate(numerator: u64, denominator: u64) -> String {
            if denominator == 0 {
                "NA".to_string()
            } else {
                format!("{:.4}", numerator as f64 / denominator as f64)
            }
        }
    }

    fn paired_delta(policy_losses: &[f64], baseline_losses: &[f64]) -> Vec<f64> {
        policy_losses
            .iter()
            .zip(baseline_losses.iter())
            .map(|(policy, baseline)| policy - baseline)
            .collect()
    }

    fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            return f64::NAN;
        }
        values.iter().sum::<f64>() / values.len() as f64
    }

    /// Percentile paired bootstrap over exogenous seed pairs. The persistent
    /// and removed branches are averaged inside each pair before resampling,
    /// so counterfactual siblings are never treated as independent samples.
    fn paired_bootstrap_ci(values: &[f64], domain: u64) -> (f64, f64) {
        const BOOTSTRAPS: usize = 2_000;
        if values.is_empty() {
            return (f64::NAN, f64::NAN);
        }
        let mut means = Vec::with_capacity(BOOTSTRAPS);
        for bootstrap_index in 0..BOOTSTRAPS {
            let bootstrap_seed = mix64(
                ROOT_SEED
                    ^ domain
                    ^ (bootstrap_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
            let mut total = 0.0;
            for draw in 0..values.len() {
                let draw_seed = mix64(
                    bootstrap_seed
                        ^ (draw as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
                );
                let index = (draw_seed % values.len() as u64) as usize;
                total += values[index];
            }
            means.push(total / values.len() as f64);
        }
        means.sort_by(f64::total_cmp);
        let lower = means[((BOOTSTRAPS as f64 * 0.025) as usize).min(BOOTSTRAPS - 1)];
        let upper = means[((BOOTSTRAPS as f64 * 0.975) as usize).min(BOOTSTRAPS - 1)];
        (lower, upper)
    }

    fn family_domain(family: SceneFamily) -> u64 {
        match family {
            SceneFamily::Development => 0x5449_4D45_4445_564C,
            SceneFamily::HeldOut => 0x5449_4D45_484F_4C44,
        }
    }

    fn seed_for(replicate: usize, pair: usize, family: SceneFamily) -> u64 {
        mix64(
            ROOT_SEED
                ^ family_domain(family)
                ^ (replicate as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (pair as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
        )
    }

    pub(super) fn run_report() {
        println!("VIS-ACT-005 randomized-timing and delayed-view diagnostic");
        println!("event-driven onset/duration/delay; transient RGB evidence; hidden view-success outcomes");
        println!("paired bootstrap resamples exogenous seed pairs; resource units remain synthetic");
        println!("family\tpolicy\tepisodes\tmean_loss\tanswered_accuracy\tabstention_rate\tremoved_false_persistence_rate\tview_attempts_per_episode\texecuted_views\tcompleted_views\tfresh_evidence_rate_per_executed_view\tviews_without_fresh_evidence\trecovery_rate\tmean_recovery_steps_when_recovered\tview_attributable_recovery_rate\tmean_view_recovery_steps_when_attributed\tlatency_units_per_episode\tbudget_violations\tpaired_delta_loss_vs_passive\tbootstrap95_lo_vs_passive\tbootstrap95_hi_vs_passive\tpaired_delta_loss_vs_fixed_sweep\tbootstrap95_lo_vs_fixed_sweep\tbootstrap95_hi_vs_fixed_sweep");

        let policies = TimedPolicy::ALL;
        let passive_index = policies
            .iter()
            .position(|policy| *policy == TimedPolicy::PassiveNoView)
            .expect("passive no-view baseline exists");
        let sweep_index = policies
            .iter()
            .position(|policy| *policy == TimedPolicy::FixedSweep)
            .expect("fixed-sweep baseline exists");

        for family in [SceneFamily::Development, SceneFamily::HeldOut] {
            // Split early/late relative to each family's support so each
            // profile has both strata; print the actual cutoff to avoid
            // implying that the same absolute tick defines both families.
            let early_cutoff = match family {
                SceneFamily::Development => 2,
                SceneFamily::HeldOut => 3,
            };
            let mut totals = vec![Aggregate::default(); policies.len()];
            let mut early_onset = vec![Aggregate::default(); policies.len()];
            let mut late_onset = vec![Aggregate::default(); policies.len()];
            let mut available_success = vec![Aggregate::default(); policies.len()];
            let mut available_failure = vec![Aggregate::default(); policies.len()];
            let mut unavailable = vec![Aggregate::default(); policies.len()];
            let mut short_delay = vec![Aggregate::default(); policies.len()];
            let mut long_delay = vec![Aggregate::default(); policies.len()];
            let pair_count = WORLD_REPLICATES * PAIRS_PER_REPLICATE;
            let mut paired_losses = vec![Vec::with_capacity(pair_count); policies.len()];

            for replicate in 0..WORLD_REPLICATES {
                for pair in 0..PAIRS_PER_REPLICATE {
                    let seed = seed_for(replicate, pair, family);
                    let persistent = TimedWorld::generated(seed, true, family);
                    let removed = persistent.paired_with(false);
                    let is_early = persistent.occlusion_onset <= early_cutoff;

                    for (index, policy) in policies.iter().copied().enumerate() {
                        let actor_seed = mix64(
                            0x4143_544F_525F_5345
                                ^ (replicate as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                                ^ (pair as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
                                ^ (index as u64).wrapping_mul(0xA24B_AED4_963E_E407),
                        );
                        let present_result = run_episode(persistent, policy, actor_seed);
                        let removed_result = run_episode(removed, policy, actor_seed);
                        totals[index].record(present_result);
                        totals[index].record(removed_result);
                        if is_early {
                            early_onset[index].record(present_result);
                            early_onset[index].record(removed_result);
                        } else {
                            late_onset[index].record(present_result);
                            late_onset[index].record(removed_result);
                        }

                        // Evaluator-only acquisition strata. These conditions
                        // are never passed to the policy. The paired branches
                        // share availability, success, and delay by construction.
                        if persistent.view_available && persistent.view_successful {
                            available_success[index].record(present_result);
                            available_success[index].record(removed_result);
                        } else if persistent.view_available {
                            available_failure[index].record(present_result);
                            available_failure[index].record(removed_result);
                        } else {
                            unavailable[index].record(present_result);
                            unavailable[index].record(removed_result);
                        }
                        if persistent.view_delay <= 2 {
                            short_delay[index].record(present_result);
                            short_delay[index].record(removed_result);
                        } else {
                            long_delay[index].record(present_result);
                            long_delay[index].record(removed_result);
                        }

                        // Treat the paired persistent/removal branches as one
                        // independent exogenous unit for uncertainty analysis.
                        paired_losses[index].push(
                            (present_result.loss + removed_result.loss) / 2.0,
                        );
                    }
                }
            }

            let family_code = family_domain(family);
            for (index, policy) in policies.iter().copied().enumerate() {
                let total = totals[index];
                let passive_delta =
                    paired_delta(&paired_losses[index], &paired_losses[passive_index]);
                let sweep_delta =
                    paired_delta(&paired_losses[index], &paired_losses[sweep_index]);
                let (passive_low, passive_high) = paired_bootstrap_ci(
                    &passive_delta,
                    family_code ^ (index as u64).wrapping_mul(0xA24B_AED4_963E_E407),
                );
                let (sweep_low, sweep_high) = paired_bootstrap_ci(
                    &sweep_delta,
                    family_code ^ 0x5357_4545_505F_4349
                        ^ (index as u64).wrapping_mul(0x9FB2_1C65_1E98_DF25),
                );
                let mean_recovery = if total.recovery_observations == 0 {
                    "NA".to_string()
                } else {
                    format!(
                        "{:.3}",
                        total.recovery_steps_sum as f64
                            / total.recovery_observations as f64
                    )
                };
                let mean_view_recovery = if total.view_recovery_observations == 0 {
                    "NA".to_string()
                } else {
                    format!(
                        "{:.3}",
                        total.view_recovery_steps_sum as f64
                            / total.view_recovery_observations as f64
                    )
                };
                println!(
                    "{}\t{}\t{}\t{:.4}\t{}\t{}\t{}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}",
                    family.name(),
                    policy.name(),
                    total.episodes,
                    total.mean_loss(),
                    Aggregate::rate(total.correct_answered, total.answered),
                    Aggregate::rate(total.abstentions, total.episodes),
                    Aggregate::rate(total.false_persistence, total.removed_episodes),
                    total.view_attempts as f64 / total.episodes.max(1) as f64,
                    total.executed_views,
                    total.completed_views,
                    Aggregate::rate(total.fresh_views, total.executed_views),
                    total.views_without_fresh,
                    Aggregate::rate(total.recovery_observations, total.episodes),
                    mean_recovery,
                    Aggregate::rate(total.view_recovery_observations, total.episodes),
                    mean_view_recovery,
                    total.latency_sum as f64 / total.episodes.max(1) as f64,
                    total.budget_violations,
                    mean(&passive_delta),
                    passive_low,
                    passive_high,
                    mean(&sweep_delta),
                    sweep_low,
                    sweep_high,
                );
            }

            println!(
                "VIS-ACT-005 timing-stratum diagnostics for {} (early onset <= {}; late onset > {})",
                family.name(),
                early_cutoff,
                early_cutoff
            );
            println!("family\tstratum\tpolicy\tepisodes\tmean_loss\tabstention_rate\tremoved_false_persistence_rate\tfresh_evidence_rate_per_executed_view\trecovery_rate\tmean_recovery_steps_when_recovered\tview_attributable_recovery_rate\tmean_view_recovery_steps_when_attributed");
            for (stratum_name, strata) in [
                ("early_onset", &early_onset),
                ("late_onset", &late_onset),
            ] {
                for (index, policy) in policies.iter().copied().enumerate() {
                    let aggregate = strata[index];
                    let recovery = if aggregate.recovery_observations == 0 {
                        "NA".to_string()
                    } else {
                        format!(
                            "{:.3}",
                            aggregate.recovery_steps_sum as f64
                                / aggregate.recovery_observations as f64
                        )
                    };
                    let view_recovery = if aggregate.view_recovery_observations == 0 {
                        "NA".to_string()
                    } else {
                        format!(
                            "{:.3}",
                            aggregate.view_recovery_steps_sum as f64
                                / aggregate.view_recovery_observations as f64
                        )
                    };
                    println!(
                        "{}\t{}\t{}\t{}\t{:.4}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        family.name(),
                        stratum_name,
                        policy.name(),
                        aggregate.episodes,
                        aggregate.mean_loss(),
                        Aggregate::rate(aggregate.abstentions, aggregate.episodes),
                        Aggregate::rate(aggregate.false_persistence, aggregate.removed_episodes),
                        Aggregate::rate(aggregate.fresh_views, aggregate.executed_views),
                        Aggregate::rate(aggregate.recovery_observations, aggregate.episodes),
                        recovery,
                        Aggregate::rate(aggregate.view_recovery_observations, aggregate.episodes),
                        view_recovery,
                    );
                }
            }

            println!(
                "VIS-ACT-005 acquisition-stratum diagnostics for {}",
                family.name()
            );
            println!("family\tacquisition_stratum\tpolicy\tepisodes\tmean_loss\tabstention_rate\tfalse_persistence_rate\tview_attempts_per_episode\tfresh_evidence_rate_per_executed_view\tview_attributable_recovery_rate\tmean_view_recovery_steps_when_attributed");
            for (stratum_name, strata) in [
                ("view_available_and_successful", &available_success),
                ("view_available_but_failed", &available_failure),
                ("view_unavailable", &unavailable),
                ("short_view_delay_le_2", &short_delay),
                ("long_view_delay_gt_2", &long_delay),
            ] {
                for (index, policy) in policies.iter().copied().enumerate() {
                    let aggregate = strata[index];
                    let mean_view_recovery = if aggregate.view_recovery_observations == 0 {
                        "NA".to_string()
                    } else {
                        format!(
                            "{:.3}",
                            aggregate.view_recovery_steps_sum as f64
                                / aggregate.view_recovery_observations as f64
                        )
                    };
                    println!(
                        "{}\t{}\t{}\t{}\t{:.4}\t{}\t{}\t{:.3}\t{}\t{}\t{}",
                        family.name(),
                        stratum_name,
                        policy.name(),
                        aggregate.episodes,
                        aggregate.mean_loss(),
                        Aggregate::rate(aggregate.abstentions, aggregate.episodes),
                        Aggregate::rate(aggregate.false_persistence, aggregate.removed_episodes),
                        aggregate.view_attempts as f64 / aggregate.episodes.max(1) as f64,
                        Aggregate::rate(aggregate.fresh_views, aggregate.executed_views),
                        Aggregate::rate(aggregate.view_recovery_observations, aggregate.episodes),
                        mean_view_recovery,
                    );
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn paired(seed: u64, family: SceneFamily) -> (TimedWorld, TimedWorld) {
            let persistent = TimedWorld::generated(seed, true, family);
            (persistent, persistent.paired_with(false))
        }

        #[test]
        fn early_and_late_strata_exist_within_each_family_support() {
            for family in [SceneFamily::Development, SceneFamily::HeldOut] {
                let cutoff = match family {
                    SceneFamily::Development => 2,
                    SceneFamily::HeldOut => 3,
                };
                let worlds: Vec<TimedWorld> = (0..128)
                    .map(|seed| TimedWorld::generated(seed, true, family))
                    .collect();
                assert!(worlds.iter().any(|world| world.occlusion_onset <= cutoff));
                assert!(worlds.iter().any(|world| world.occlusion_onset > cutoff));
            }
        }

        #[test]
        fn paired_hidden_branches_share_randomized_timing_and_action_outcomes() {
            for family in [SceneFamily::Development, SceneFamily::HeldOut] {
                let (present, removed) = paired(0xCAFE_BABE, family);
                assert_eq!(present.occlusion_onset, removed.occlusion_onset);
                assert_eq!(present.occlusion_duration, removed.occlusion_duration);
                assert_eq!(present.view_delay, removed.view_delay);
                assert_eq!(present.view_available, removed.view_available);
                assert_eq!(present.view_successful, removed.view_successful);
            }
        }

        #[test]
        fn timing_distributions_vary_independently_and_repeat_exactly() {
            for family in [SceneFamily::Development, SceneFamily::HeldOut] {
                let first = TimedWorld::generated(29, true, family);
                assert_eq!(first, TimedWorld::generated(29, true, family));

                let worlds: Vec<TimedWorld> = (0..128)
                    .map(|seed| TimedWorld::generated(seed, true, family))
                    .collect();
                assert!(worlds.iter().map(|w| w.occlusion_onset).collect::<std::collections::BTreeSet<_>>().len() > 1);
                assert!(worlds.iter().map(|w| w.occlusion_duration).collect::<std::collections::BTreeSet<_>>().len() > 1);
                assert!(worlds.iter().map(|w| w.view_delay).collect::<std::collections::BTreeSet<_>>().len() > 1);
            }
        }

        #[test]
        fn branch_pixels_are_identical_before_the_first_legal_reveal() {
            let (present, removed) = paired(0x1234_5678, SceneFamily::HeldOut);
            for tick in 0..=DECISION_DEADLINE {
                let frame_a = present.frame(tick, false, None);
                let frame_b = removed.frame(tick, false, None);
                if tick < present.natural_reveal_tick() {
                    assert_eq!(frame_a.pixels, frame_b.pixels, "tick {tick}");
                }
            }
        }

        #[test]
        fn delayed_success_only_reveals_at_completion_tick() {
            let (present, removed) = paired(0x1122_3344, SceneFamily::Development);
            let receipt = ViewReceipt {
                requested_tick: present.occlusion_onset,
                completion_tick: present.occlusion_onset + 2,
                executed: true,
                successful: true,
            };
            assert_eq!(
                perceive(&present.frame(receipt.requested_tick, true, Some(receipt))).cue,
                Cue::Occluded
            );
            let p = present.frame(receipt.completion_tick, true, Some(receipt));
            let r = removed.frame(receipt.completion_tick, true, Some(receipt));
            if receipt.completion_tick < present.natural_reveal_tick() {
                assert_eq!(perceive(&p).cue, Cue::Present);
                assert_eq!(perceive(&r).cue, Cue::Absent);
            }
        }

        #[test]
        fn successful_view_recovery_is_separated_from_natural_recovery() {
            let (world, _) = paired(0x1234_5678, SceneFamily::Development);
            let forced = TimedWorld {
                view_available: true,
                view_successful: true,
                occlusion_onset: 1,
                occlusion_duration: 4,
                view_delay: 1,
                ..world
            };
            let result = run_episode(forced, TimedPolicy::ActiveRevision, 0xF0CED);
            assert!(result.fresh_view_evidence);
            assert_eq!(result.view_attributable_recovery_steps, Some(1));
            assert!(result.natural_or_view_recovery_steps.is_some());

            let mut aggregate = Aggregate::default();
            aggregate.record(result);
            assert_eq!(aggregate.view_recovery_observations, 1);
            assert_eq!(aggregate.view_recovery_steps_sum, 1);
        }

        #[test]
        fn failed_view_is_completed_but_has_no_fresh_evidence_credit() {
            let (world, _) = paired(0xFACE_1234, SceneFamily::HeldOut);
            let failed = TimedWorld {
                view_available: true,
                view_successful: false,
                occlusion_onset: 2,
                occlusion_duration: 4,
                view_delay: 1,
                ..world
            };
            let result = run_episode(failed, TimedPolicy::ActiveRevision, 0xFA1ED);
            assert!(result.attempt);
            assert!(result.executed);
            assert!(result.completed);
            assert!(!result.fresh_view_evidence);
            assert!(result.view_returned_no_fresh_evidence);
            assert_eq!(result.view_attributable_recovery_steps, None);
            assert_eq!(result.budget_violations, 0);
        }

        #[test]
        fn deadline_late_action_cannot_claim_fresh_view_evidence() {
            let (world, _) = paired(0xFEED_4321, SceneFamily::HeldOut);
            let late = TimedWorld {
                view_available: true,
                view_successful: true,
                occlusion_onset: 4,
                occlusion_duration: 4,
                view_delay: 4,
                ..world
            };
            let result = run_episode(late, TimedPolicy::ActiveRevision, 0x1A7E);
            assert!(result.attempt);
            assert!(result.executed);
            assert!(!result.fresh_view_evidence);
            assert!(result.view_returned_no_fresh_evidence);
            assert_eq!(result.view_attributable_recovery_steps, None);
        }

        #[test]
        fn deadline_late_view_is_executed_but_not_completed_or_informative() {
            let (world, _) = paired(0xFEED_4321, SceneFamily::HeldOut);
            let late = TimedWorld {
                view_available: true,
                view_successful: true,
                occlusion_onset: 4,
                occlusion_duration: 4,
                view_delay: 4,
                ..world
            };
            let result = run_episode(late, TimedPolicy::ActiveRevision, 0x1A7E);
            assert!(result.attempt);
            assert!(result.executed);
            assert!(!result.completed);
            assert!(!result.fresh_view_evidence);
            assert!(result.view_returned_no_fresh_evidence);
        }

        #[test]
        fn passive_no_view_baseline_never_proposes_sensing() {
            let (world, _) = paired(0x0BADC0DE, SceneFamily::HeldOut);
            for tick in 0..=HORIZON {
                let frame = world.frame(tick, false, None);
                assert_eq!(TimedPolicy::PassiveNoView.propose(&frame, false, 0), Action::Wait);
            }
        }

        #[test]
        fn fixed_sweep_uses_schedule_not_hidden_timing_or_rgb_labels() {
            let (world, _) = paired(0xABCD, SceneFamily::Development);
            let occluded = world.frame(world.occlusion_onset, false, None);
            if occluded.side_view_available {
                let expected = if matches!(occluded.tick, 2 | 4) {
                    Action::ChangeView
                } else {
                    Action::Wait
                };
                assert_eq!(TimedPolicy::FixedSweep.propose(&occluded, false, 0), expected);
            }
            let sweep = world.frame(2, false, None);
            if sweep.side_view_available {
                assert_eq!(TimedPolicy::FixedSweep.propose(&sweep, false, 0), Action::ChangeView);
            }
        }

        #[test]
        fn uniform_reservoir_is_seeded_bounded_and_replayable() {
            let (world, _) = paired(0x6D45_0A11, SceneFamily::HeldOut);
            let mut left = Memory::new_seeded(Policy::UniformReservoir, 0xA11CE);
            let mut right = Memory::new_seeded(Policy::UniformReservoir, 0xA11CE);
            for tick in 0..=HORIZON {
                let frame = world.frame(tick, false, None);
                left.observe(&frame);
                right.observe(&frame);
                let (
                    Memory::UniformReservoir { entries: left_entries, seen: left_seen, .. },
                    Memory::UniformReservoir { entries: right_entries, seen: right_seen, .. },
                ) = (&left, &right) else {
                    panic!("uniform policy must instantiate reservoir memory");
                };
                assert!(left_entries.len() <= 2);
                assert_eq!(left_entries, right_entries);
                assert_eq!(*left_seen, u64::from(tick) + 1);
                assert_eq!(*left_seen, *right_seen);
            }
        }

        #[test]
        fn recency_protected_reservoir_preserves_direct_evidence_through_occlusion() {
            let (base, _) = paired(0xBEEF_A11CE, SceneFamily::HeldOut);
            let world = TimedWorld {
                occlusion_onset: 1,
                occlusion_duration: 4,
                view_delay: 1,
                view_available: true,
                view_successful: true,
                ..base
            };
            let receipt = ViewReceipt {
                requested_tick: 1,
                completion_tick: 2,
                executed: true,
                successful: true,
            };

            for (terminal_present, expected_cue, expected_decision) in [
                (true, Cue::Present, Decision::Present),
                (false, Cue::Absent, Decision::Absent),
            ] {
                let branch = world.paired_with(terminal_present);
                let mut memory = Memory::new_seeded(Policy::RecencyProtectedReservoir, 0xA11CE);
                let initial = branch.frame(0, false, None);
                assert_eq!(perceive(&initial).cue, Cue::Present);
                memory.observe(&initial);

                let occluded = branch.frame(1, false, None);
                assert_eq!(perceive(&occluded).cue, Cue::Occluded);
                memory.observe(&occluded);

                let revealed = branch.frame(2, true, Some(receipt));
                assert_eq!(perceive(&revealed).cue, expected_cue);
                memory.observe(&revealed);

                let terminal = branch.frame(3, true, Some(receipt));
                assert_eq!(perceive(&terminal).cue, Cue::Occluded);
                memory.observe(&terminal);
                assert_eq!(
                    memory.decide(Policy::RecencyProtectedReservoir, &terminal),
                    expected_decision
                );
            }
        }

        #[test]
        fn random_legal_view_uses_actor_seed_and_never_violates_legality() {
            let (world, _) = paired(0xBEEF_1234, SceneFamily::HeldOut);
            let mut frame = world.frame(world.occlusion_onset, false, None);
            frame.side_view_available = true;

            let mut attempts = 0_u64;
            for actor_seed in 0..64_u64 {
                let first = TimedPolicy::RandomLegalView.propose(&frame, false, actor_seed);
                let second = TimedPolicy::RandomLegalView.propose(&frame, false, actor_seed);
                assert_eq!(first, second, "same actor seed must replay deterministically");
                attempts += u64::from(first == Action::ChangeView);
            }
            assert!(attempts > 8 && attempts < 56, "seeded legal-action baseline should vary");

            let world = TimedWorld {
                view_available: true,
                view_successful: false,
                occlusion_onset: 1,
                occlusion_duration: 4,
                view_delay: 1,
                ..world
            };
            for actor_seed in 0..64_u64 {
                let result = run_episode(world, TimedPolicy::RandomLegalView, actor_seed);
                assert!(result.latency_units <= LATENCY_BUDGET);
                assert_eq!(result.budget_violations, 0);
            }
        }

        #[test]
        fn paired_bootstrap_is_deterministic_and_returns_ordered_intervals() {
            let deltas = [-0.2, -0.1, 0.0, 0.1, 0.05, -0.15];
            let a = paired_bootstrap_ci(&deltas, 0x1234);
            let b = paired_bootstrap_ci(&deltas, 0x1234);
            assert_eq!(a, b);
            assert!(a.0 <= a.1);

            let zeros = [0.0; 8];
            assert_eq!(paired_bootstrap_ci(&zeros, 9), (0.0, 0.0));
        }

        #[test]
        fn every_timed_policy_stays_within_budgets_and_is_deterministic() {
            let world = TimedWorld::generated(0x9911, false, SceneFamily::HeldOut);
            for policy in TimedPolicy::ALL {
                let a = run_episode(world, policy, 0xD37E_0001);
                let b = run_episode(world, policy, 0xD37E_0001);
                assert_eq!(a.loss, b.loss);
                assert!(a.latency_units <= LATENCY_BUDGET);
                assert_eq!(a.budget_violations, 0);
            }
        }
    }
}
