// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic, resident-led design prompts. These are not specifications or compliance verdicts.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Need {
    pub id: &'static str,
    pub symbol: &'static str,
    pub label: &'static str,
    pub description: &'static str,
}

pub const NEEDS: [Need; 9] = [
    Need { id: "vision", symbol: "◉", label: "Blind / low vision", description: "Orientation, contrast, tactile or accessible controls" },
    Need { id: "hearing", symbol: "≋", label: "Deaf / hard of hearing", description: "Visual, vibration or chosen communication options" },
    Need { id: "mobility", symbol: "↔", label: "Mobility / wheelchair / walker", description: "Step-free routes, maneuvering, reach and transfers" },
    Need { id: "dexterity", symbol: "✋", label: "Dexterity / reach / strength", description: "Low-force controls and one-hand use" },
    Need { id: "fatigue", symbol: "☾", label: "Pain / fatigue / variable energy", description: "Rest, seated tasks and lower-effort routines" },
    Need { id: "sensory", symbol: "◌", label: "Sensory environment", description: "Manageable sound, light, texture and stimulation" },
    Need { id: "cognitive", symbol: "▤", label: "Cognitive / memory / processing", description: "Predictability, legibility and manageable choices" },
    Need { id: "deafblind", symbol: "✳", label: "Deafblind / multiple sensory needs", description: "Resident-chosen tactile and redundant orientation" },
    Need { id: "support", symbol: "♡", label: "Support person / service animal", description: "Shared routes, privacy and usable support space" },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectStage {
    NewBuild,
    Renovation,
    Rental,
}

impl ProjectStage {
    pub const ALL: [ProjectStage; 3] = [ProjectStage::NewBuild, ProjectStage::Renovation, ProjectStage::Rental];

    pub const fn label(self) -> &'static str {
        match self {
            ProjectStage::NewBuild => "New build",
            ProjectStage::Renovation => "Renovation",
            ProjectStage::Rental => "Existing / rental",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recommendation {
    pub id: &'static str,
    pub room: &'static str,
    pub title: &'static str,
    pub task: &'static str,
    pub rationale: &'static str,
    pub reviewer: &'static str,
    pub applies_to: &'static [&'static str],
    pub planning_stage: &'static str,
    pub evidence_question: &'static str,
}

pub const RECOMMENDATIONS: [Recommendation; 19] = [
    Recommendation {
        id: "approach-entry", room: "Site + entrance", title: "Plan the approach and entrance as one continuous journey",
        task: "Arrive, unlock the door, enter and bring in groceries or mobility equipment.",
        rationale: "Route surface, threshold, landing, drainage, door operation and maneuvering all interact. A separate side entrance can segregate residents.",
        reviewer: "Resident + architect / access specialist",
        applies_to: &[], planning_stage: "Resolve in site and layout planning",
        evidence_question: "Site measurements, actual mobility aids, door hardware, weather exposure and applicable rules are not yet assessed.",
    },
    Recommendation {
        id: "circulation", room: "Whole home", title: "Test clear paths with real furniture and equipment",
        task: "Move between rooms, turn, carry items, use a cane or mobility device, and open doors.",
        rationale: "Nominal wall-to-wall dimensions can hide blocked paths created by door swings, furniture, storage or equipment.",
        reviewer: "Resident + architect / access specialist",
        applies_to: &[], planning_stage: "Check every room-to-room route",
        evidence_question: "Record clear dimensions and maneuvering/transfer envelopes for the actual resident and equipment; don't infer from a generic profile.",
    },
    Recommendation {
        id: "bathroom-night", room: "Bathroom + bedroom route", title: "Design the bathroom and night-time route around the person's task",
        task: "Get to the toilet, wash or shower, transfer if needed, and return to bed safely.",
        rationale: "Route length, lighting, floor surface, privacy, controls, door swing, transfers and support needs should be reviewed together.",
        reviewer: "Resident + access specialist; qualified building professionals as needed",
        applies_to: &[], planning_stage: "Prioritize high-frequency and night-time tasks",
        evidence_question: "Actual fixture layout, clearances, wet-area design, support needs and local requirements need review.",
    },
    Recommendation {
        id: "reachable-controls", room: "Whole home", title: "Make essential controls easy to find and operate",
        task: "Operate lights, locks, windows, taps, appliances and utilities with the resident's preferred method.",
        rationale: "Reach, grip, force, contrast, tactile feedback, labels and consistent placement affect independence; a smartphone-only control can exclude people.",
        reviewer: "Resident + designer / relevant specialist",
        applies_to: &[], planning_stage: "Inventory controls before selecting products",
        evidence_question: "Trial the actual control in the intended position and verify a usable manual alternative where appropriate.",
    },
    Recommendation {
        id: "life-safety", room: "Whole home + exits", title: "Create a resident-specific life-safety and emergency review",
        task: "Recognize a life-safety alert, understand what it means, reach an exit and follow the resident's emergency plan.",
        rationale: "Alarm modality, placement, audibility/visibility/vibration, sleep, backup power and exit access interact. A design prompt cannot establish fire or evacuation safety.",
        reviewer: "Qualified fire / electrical / building professionals + resident",
        applies_to: &[], planning_stage: "Qualified safety review is a hard gate",
        evidence_question: "Approved alarm/device specification, coverage, installation, backup, testing and applicable authority review are unverified.",
    },
    Recommendation {
        id: "vision-wayfinding", room: "Routes + doorways + storage", title: "Agree a consistent wayfinding language",
        task: "Orient through the home and find important rooms or stored items.",
        rationale: "The resident may prefer tactile landmarks, consistent object locations, accessible text, high-contrast edges or audio description. No one cue works for everyone.",
        reviewer: "Blind / low-vision resident + orientation-and-mobility or access specialist as chosen",
        applies_to: &["vision"], planning_stage: "Co-design before fixing storage and finishes",
        evidence_question: "Trial cues with the resident; check glare, protrusions, trip hazards and whether the route stays consistent.",
    },
    Recommendation {
        id: "vision-appliances", room: "Kitchen + laundry + controls", title: "Choose accessible controls and labels for appliances",
        task: "Identify a setting, measure or dispense ingredients, use laundry controls and detect control state.",
        rationale: "Tactile markings, accessible menus, physical detents, talking output or contrast may help in different combinations. Touchscreens alone can remove useful tactile landmarks.",
        reviewer: "Resident + occupational / access specialist; appliance supplier",
        applies_to: &["vision", "deafblind"], planning_stage: "Check before appliances and cabinetry are ordered",
        evidence_question: "Exact model controls, feedback, language, independent usability and maintenance need to be tested.",
    },
    Recommendation {
        id: "hearing-alerts", room: "Entrance + bedroom + shared rooms", title: "Choose alert modalities room by room",
        task: "Notice a visitor, phone call, smoke/CO alarm or other important alert while awake or asleep.",
        rationale: "Depending on the resident and alert type, visual, vibration, tactile or suitable audible signals may be needed; one phone notification is not an assumed substitute for life-safety equipment.",
        reviewer: "Deaf / hard-of-hearing resident + qualified alarm/electrical professional",
        applies_to: &["hearing", "deafblind"], planning_stage: "Coordinate system, coverage, installation and maintenance",
        evidence_question: "Check approved devices, alert meaning, coverage across rooms, sleep alerts, power backup and regular testing; the selected system is not yet verified.",
    },
    Recommendation {
        id: "communication", room: "Entry + living / dining areas", title: "Make face-to-face communication work for the resident",
        task: "Talk with family, visitors, interpreters or support people in the resident's preferred language or modality.",
        rationale: "Sightlines, seating arrangement, lighting on faces, visual distractions, acoustics and privacy matter; not every Deaf person uses the same communication method.",
        reviewer: "Resident + Deaf / communication access specialist as chosen",
        applies_to: &["hearing", "deafblind"], planning_stage: "Test using the real conversation setup",
        evidence_question: "Resident's preferred modality, lighting and seating preferences are not inferred from the label.",
    },
    Recommendation {
        id: "mobility-maneuver", room: "Entrance + routes + key rooms", title: "Measure maneuvering, turning and transfer envelopes",
        task: "Turn, line up with a door, move beside a bed or fixture, and transfer between devices and surfaces if needed.",
        rationale: "Required space depends on the actual person, mobility aid, transfer method, helper involvement and configuration; broad averages can be inadequate.",
        reviewer: "Resident + access specialist / architect",
        applies_to: &["mobility"], planning_stage: "Measure before layout is frozen",
        evidence_question: "Drawings must be checked against measured clearances with furniture and door swings, not just room labels.",
    },
    Recommendation {
        id: "mobility-bath", room: "Bathroom + shower", title: "Co-design shower, toilet and transfer support",
        task: "Wash, toilet, transfer, reach toiletries and preserve privacy.",
        rationale: "Approach space, transfer side, controls, slip resistance, drainage and possible grab-bar reinforcement must fit the resident's chosen method.",
        reviewer: "Resident + access / occupational specialist + qualified installer",
        applies_to: &["mobility", "dexterity", "fatigue"], planning_stage: "Plan reinforcement and waterproofing before finishes",
        evidence_question: "Actual transfer method, structure, waterproofing, fixtures and jurisdictional provisions need professional assessment.",
    },
    Recommendation {
        id: "kitchen-work", room: "Kitchen", title: "Place food-preparation tasks within usable reach",
        task: "Prepare food, handle hot items, reach storage and work seated or standing if preferred.",
        rationale: "Work surfaces, knee space, appliance doors, storage, turning space and hot/wet tasks interact. One fixed counter height will not fit every household.",
        reviewer: "Resident + kitchen designer / occupational or access specialist",
        applies_to: &["mobility", "dexterity", "fatigue"], planning_stage: "Plan work zones and service locations early",
        evidence_question: "Mock up reach, approach and controls with the actual resident before selecting cabinets and appliances.",
    },
    Recommendation {
        id: "low-force", room: "Doors + windows + taps + storage", title: "Reduce avoidable force, pinching and complex manipulation",
        task: "Open doors, operate taps, use handles and retrieve frequently used items.",
        rationale: "Lever hardware, power assistance or alternative mechanisms may help, but choices depend on grip, reach, balance, privacy and fire-door requirements.",
        reviewer: "Resident + hardware / access professional",
        applies_to: &["dexterity", "fatigue", "mobility"], planning_stage: "Trial representative hardware and preserve required door functions",
        evidence_question: "Operating force, user trial, door closer/fire requirement and maintenance need confirmation.",
    },
    Recommendation {
        id: "energy-rest", room: "Entry + kitchen + bedroom + outdoor route", title: "Make rest and task staging part of the layout",
        task: "Pause during a routine, sit while preparing food or dressing, and place items down safely.",
        rationale: "Rest points, reachable staging surfaces, shorter routes and seated options can reduce unnecessary effort without prescribing a pace.",
        reviewer: "Resident + occupational / access specialist as chosen",
        applies_to: &["fatigue", "mobility"], planning_stage: "Map effort across a complete daily routine",
        evidence_question: "Resident-specific effort varies; validate by walking or simulating the real task sequence.",
    },
    Recommendation {
        id: "sensory-controls", room: "Lighting + living / sleep spaces", title: "Give the resident control over sensory conditions",
        task: "Read, rest, communicate, sleep and recover in preferred light and sound conditions.",
        rationale: "Adjustable light, glare control, acoustic zoning and a quiet refuge may help, but overly dim spaces, constant noise, reflective surfaces or alerts can also create barriers.",
        reviewer: "Resident + lighting / acoustics specialist where needed",
        applies_to: &["sensory", "vision", "hearing", "cognitive"], planning_stage: "Trial adjustable settings and day/night scenarios",
        evidence_question: "Test different times of day, glare, external noise, communication needs and resident preferences; no universal sensory setting is assumed.",
    },
    Recommendation {
        id: "cognitive-structure", room: "Whole home + storage", title: "Make the home's organization legible and predictable",
        task: "Find items, understand controls, complete routines and recover after an interruption.",
        rationale: "Consistent zones, plain labels, stable storage, good cues and less unnecessary visual clutter can be helpful; resident preference takes precedence.",
        reviewer: "Resident + chosen support / occupational specialist",
        applies_to: &["cognitive"], planning_stage: "Co-design routines with the resident",
        evidence_question: "Test real routines; do not assume labels, colors, automation or reminders work for every person.",
    },
    Recommendation {
        id: "deafblind-tactile", room: "Routes + essential controls + bedroom", title: "Co-design tactile orientation and alerts across the whole home",
        task: "Navigate, identify controls and notice important events with the resident's chosen sensory methods.",
        rationale: "Combined vision and hearing needs vary widely; a system may need tactile cues, tactile alert devices, consistent placement and human communication preferences.",
        reviewer: "Deafblind resident + specialist selected by the resident",
        applies_to: &["deafblind"], planning_stage: "Prototype and test the full route and alert chain with the resident",
        evidence_question: "No cue or device is presumed suitable; validate touch safety, meaning, placement, coverage and power failure.",
    },
    Recommendation {
        id: "support-space", room: "Entry + bathroom + sleeping + shared spaces", title: "Plan support without taking away resident autonomy",
        task: "Share a space with a support person or service animal while preserving privacy, access and independent choice.",
        rationale: "Support can affect maneuvering space, sleeping arrangements, animal rest/toileting routes, storage, communication and personal boundaries.",
        reviewer: "Resident + their chosen support people / animal professionals as relevant",
        applies_to: &["support", "mobility"], planning_stage: "Agree roles, routes, storage and privacy with the resident",
        evidence_question: "The resident chooses what support is wanted; no carer or family arrangement is assumed.",
    },
    Recommendation {
        id: "manual-fallback", room: "Locks + lighting + heating + alerts + appliances", title: "Avoid making essential independence depend on one connected system",
        task: "Operate essential functions during a power, network, phone or account failure.",
        rationale: "Automation can help, but a single app, cloud service, inaccessible touchscreen or depleted battery can become a new barrier.",
        reviewer: "Resident + electrical / assistive-technology professional",
        applies_to: &["hearing", "deafblind", "dexterity", "cognitive", "support"], planning_stage: "Include failure-mode tests before installation",
        evidence_question: "Document manual alternatives, backup power, privacy, interoperability, repair and what still works offline.",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DesignTension {
    pub id: &'static str,
    pub title: &'static str,
    pub prompt: &'static str,
    pub pair: &'static [&'static str],
}

pub const DESIGN_TENSIONS: [DesignTension; 7] = [
    DesignTension {
        id: "life-safety", title: "Life-safety systems need a qualified review",
        prompt: "Alarm coverage, notification methods, fire doors, exits, backup power and evacuation need applicable code and competent professional review. This planner cannot approve them.",
        pair: &[],
    },
    DesignTension {
        id: "contrast-glare", title: "Contrast and tactile cues versus glare and sensory load",
        prompt: "Choose color/brightness contrast and tactile landmarks with the resident; ensure cues do not protrude into circulation or overwhelm the person.",
        pair: &["vision", "sensory"],
    },
    DesignTension {
        id: "route-cues", title: "Wayfinding cues versus mobility hazards",
        prompt: "Tactile or floor-level cues must not become trip hazards, catch wheels/canes, or reduce required route clearance. Prefer resident-tested cues that preserve movement.",
        pair: &["vision", "mobility"],
    },
    DesignTension {
        id: "multichannel-alerts", title: "Alert redundancy must match the resident and the alert",
        prompt: "Visual, auditory and tactile signals are not interchangeable. Select approved life-safety equipment and user-tested everyday alerts; do not rely on a phone alone.",
        pair: &["hearing", "deafblind"],
    },
    DesignTension {
        id: "simple-controls", title: "Automation versus independent manual operation",
        prompt: "An automated lock, blind, tap or light still needs a usable fallback if power, connectivity or an account fails; test privacy and repairability.",
        pair: &["dexterity", "cognitive"],
    },
    DesignTension {
        id: "quiet-alerts", title: "Acoustic comfort versus audibility and communication",
        prompt: "Sound insulation and quiet zones must coexist with the resident's chosen alert and communication methods. Plan room-specific alerts and visible/tactile alternatives as appropriate.",
        pair: &["sensory", "hearing"],
    },
    DesignTension {
        id: "support-clearance", title: "Support space versus resident privacy and autonomy",
        prompt: "Allow space for transfers, assistance or an animal only when requested. Confirm permission, role, storage and boundaries with the resident, not just a household proxy.",
        pair: &["mobility", "support"],
    },
];

pub fn toggle_value(values: &mut Vec<&'static str>, value: &'static str) {
    if let Some(index) = values.iter().position(|existing| *existing == value) {
        values.remove(index);
    } else {
        values.push(value);
    }
}

pub fn selected_recommendations(selected_ids: &[&'static str]) -> Vec<&'static Recommendation> {
    RECOMMENDATIONS
        .iter()
        .filter(|recommendation| {
            recommendation.applies_to.is_empty()
                || recommendation.applies_to.iter().any(|need| selected_ids.contains(need))
        })
        .collect()
}

pub fn selected_tensions(selected_ids: &[&'static str]) -> Vec<&'static DesignTension> {
    DESIGN_TENSIONS
        .iter()
        .filter(|tension| {
            tension.pair.is_empty() || tension.pair.iter().all(|need| selected_ids.contains(need))
        })
        .collect()
}

pub fn project_stage_guidance(stage: ProjectStage) -> &'static str {
    match stage {
        ProjectStage::NewBuild => "Resolve high-impact choices before the layout, structure, services and finishes are fixed.",
        ProjectStage::Renovation => "Measure the existing home first; compare reversible changes with structural work and keep residents involved during disruption.",
        ProjectStage::Rental => "Start with resident-controlled, reversible options and obtain required permission before modifying the building or fixed systems.",
    }
}

pub fn brief_text(
    selected_ids: &[&'static str],
    included_ids: &[&'static str],
    stage: ProjectStage,
) -> String {
    let selected_needs = NEEDS
        .iter()
        .filter(|need| selected_ids.contains(&need.id))
        .map(|need| need.label)
        .collect::<Vec<_>>();
    let included = RECOMMENDATIONS
        .iter()
        .filter(|item| included_ids.contains(&item.id))
        .collect::<Vec<_>>();

    let mut output = String::from("ACCESSIBLE HOME DESIGN BRIEF — DRAFT v0.1\n");
    output.push_str("Status: resident-led planning prompts; nothing is verified by this brief.\n");
    output.push_str(&format!("Project context: {}\n", stage.label()));
    output.push_str(&format!(
        "Functional design dimensions selected: {}\n\n",
        if selected_needs.is_empty() { "None selected yet".to_owned() } else { selected_needs.join(", ") }
    ));
    output.push_str("PROJECT GUIDANCE\n");
    output.push_str(project_stage_guidance(stage));
    output.push_str("\n\nINCLUDED ROOM / TASK PROMPTS\n");

    if included.is_empty() {
        output.push_str("No prompts selected yet. Review the universal baseline and any suggestions below.\n");
    }

    for item in included {
        output.push_str(&format!(
            "- [{}] {} — {}\n  Task: {}\n  Why: {}\n  Review with: {}\n  Evidence needed: {}\n",
            item.room, item.title, item.id, item.task, item.rationale, item.reviewer, item.evidence_question
        ));
    }

    output.push_str("\nREVIEW BOUNDARY\n");
    output.push_str("This brief is not a construction drawing, building-code determination, accessibility certification, medical recommendation, structural assessment or life-safety approval. Obtain site-specific measurements, resident trials and qualified local professional review. Keep unknowns unknown.\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universal_baseline_is_available_without_profile_selection() {
        let items = selected_recommendations(&[]);
        assert!(items.iter().any(|item| item.id == "approach-entry"));
        assert!(items.iter().any(|item| item.id == "life-safety"));
        assert!(items.iter().any(|item| item.id == "bathroom-night"));
        assert_eq!(items.len(), 5);
    }

    #[test]
    fn selected_recommendations_are_additive_across_intersecting_needs() {
        let items = selected_recommendations(&["vision", "hearing", "mobility"]);
        assert!(items.iter().any(|item| item.id == "vision-wayfinding"));
        assert!(items.iter().any(|item| item.id == "hearing-alerts"));
        assert!(items.iter().any(|item| item.id == "mobility-maneuver"));
        assert!(items.iter().any(|item| item.id == "life-safety"));
    }

    #[test]
    fn no_recommendation_is_marked_verified() {
        assert!(RECOMMENDATIONS.iter().all(|item| {
            !item.evidence_question.is_empty() && !item.reviewer.is_empty()
        }));
    }

    #[test]
    fn selected_need_profiles_are_not_a_compliance_verdict() {
        let brief = brief_text(&["vision", "mobility"], &["approach-entry"], ProjectStage::NewBuild);
        assert!(brief.contains("nothing is verified"));
        assert!(brief.contains("not a construction drawing"));
        assert!(brief.contains("qualified local professional review"));
    }

    #[test]
    fn adding_need_profiles_never_removes_universal_gates() {
        for selection in [
            &[][..],
            &["vision"][..],
            &["hearing", "deafblind"][..],
            &["mobility", "dexterity", "fatigue"][..],
            &["vision", "hearing", "mobility", "deafblind"][..],
        ] {
            let items = selected_recommendations(selection);
            assert!(items.iter().any(|item| item.id == "life-safety"));
            assert!(items.iter().any(|item| item.id == "approach-entry"));
        }
    }

    #[test]
    fn hearing_and_deafblind_combination_surfaces_multichannel_alert_review() {
        let tensions = selected_tensions(&["hearing", "deafblind"]);
        assert!(tensions.iter().any(|item| item.id == "multichannel-alerts"));
        assert!(tensions.iter().any(|item| item.id == "life-safety"));
    }

    #[test]
    fn mobility_and_vision_surfaces_non_hazardous_wayfinding_question() {
        let tensions = selected_tensions(&["mobility", "vision"]);
        assert!(tensions.iter().any(|item| item.id == "route-cues"));
    }

    #[test]
    fn brief_includes_only_resident_included_items() {
        let brief = brief_text(&["hearing"], &["hearing-alerts"], ProjectStage::Renovation);
        assert!(brief.contains("hearing-alerts"));
        assert!(!brief.contains("vision-wayfinding"));
    }

    #[test]
    fn every_design_tension_is_tied_to_a_question() {
        assert!(DESIGN_TENSIONS.iter().all(|item| !item.prompt.is_empty() && !item.title.is_empty()));
    }

    #[test]
    fn toggling_design_prompt_never_duplicates_ids() {
        let mut included = vec!["life-safety"];
        toggle_value(&mut included, "hearing-alerts");
        toggle_value(&mut included, "hearing-alerts");
        assert_eq!(included, vec!["life-safety"]);
    }

    #[test]
    fn every_recommendation_has_evidence_and_review_context() {
        assert_eq!(RECOMMENDATIONS.len(), 19);
        assert!(RECOMMENDATIONS.iter().all(|item| {
            !item.id.is_empty() && !item.room.is_empty() && !item.task.is_empty()
                && !item.evidence_question.is_empty() && !item.reviewer.is_empty()
        }));
    }

    #[test]
    fn stage_guidance_differs_for_new_build_renovation_and_rental() {
        assert_ne!(project_stage_guidance(ProjectStage::NewBuild), project_stage_guidance(ProjectStage::Rental));
        assert_ne!(project_stage_guidance(ProjectStage::Renovation), project_stage_guidance(ProjectStage::Rental));
    }
}
