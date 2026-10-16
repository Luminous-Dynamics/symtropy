// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

mod model;

use leptos::{mount::mount_to_body, prelude::*};
use model::{
    DESIGN_TENSIONS, NEEDS, ProjectStage, RECOMMENDATIONS, brief_text, project_stage_guidance,
    selected_recommendations, selected_tensions, toggle_value,
};

#[component]
fn App() -> impl IntoView {
    let (selected_needs, set_selected_needs) = signal(Vec::<&'static str>::new());
    let (project_stage, set_project_stage) = signal(ProjectStage::NewBuild);
    let (included_ids, set_included_ids) = signal(vec![
        "approach-entry", "circulation", "bathroom-night", "life-safety",
    ]);
    let (show_brief, set_show_brief) = signal(false);
    let (show_all_tensions, set_show_all_tensions) = signal(false);

    let need_cards = NEEDS.iter().copied().map(|need| {
        view! {
            <button
                type="button"
                class="need-card"
                class:active=move || selected_needs.get().contains(&need.id)
                aria-pressed=move || if selected_needs.get().contains(&need.id) { "true" } else { "false" }
                on:click=move |_| set_selected_needs.update(|ids| toggle_value(ids, need.id))
            >
                <span class="need-symbol" aria-hidden="true">{need.symbol}</span>
                <span class="need-copy"><strong>{need.label}</strong><small>{need.description}</small></span>
                <span class="need-check" aria-hidden="true">"✓"</span>
            </button>
        }
    }).collect_view();

    let stage_cards = ProjectStage::ALL.iter().copied().map(|stage| {
        view! {
            <button
                type="button"
                class="stage-option"
                class:active=move || project_stage.get() == stage
                aria-pressed=move || if project_stage.get() == stage { "true" } else { "false" }
                on:click=move |_| set_project_stage.set(stage)
            >{stage.label()}</button>
        }
    }).collect_view();

    let recommendation_cards = move || {
        let needs = selected_needs.get();
        selected_recommendations(&needs)
            .into_iter()
            .map(|recommendation| {
                let item = *recommendation;
                view! {
                    <article class="recommendation-card" class:in-brief=move || included_ids.get().contains(&item.id)>
                        <div class="recommendation-top">
                            <span class="room-chip">{item.room}</span>
                            <span class="unverified-chip"><span aria-hidden="true">"○"</span>" Planning prompt · not verified"</span>
                        </div>
                        <h3>{item.title}</h3>
                        <p class="task-line"><strong>"Daily task: "</strong>{item.task}</p>
                        <p class="why-line">{item.rationale}</p>
                        <div class="evidence-row">
                            <div class="evidence-icon" aria-hidden="true">"?"</div>
                            <div><strong>"What still needs checking"</strong><p>{item.evidence_question}</p></div>
                        </div>
                        <div class="review-row">
                            <span><strong>"Review with: "</strong>{item.reviewer}</span>
                            <span class="stage-hint">{item.planning_stage}</span>
                        </div>
                        <button
                            type="button"
                            class="include-button"
                            aria-pressed=move || if included_ids.get().contains(&item.id) { "true" } else { "false" }
                            on:click=move |_| set_included_ids.update(|ids| toggle_value(ids, item.id))
                        >{move || if included_ids.get().contains(&item.id) { "Included in my brief ✓" } else { "Add to my design brief +"}}</button>
                    </article>
                }
            })
            .collect_view()
    };

    let tension_cards = move || {
        let needs = selected_needs.get();
        selected_tensions(&needs)
            .into_iter()
            .map(|tension| {
                let item = *tension;
                view! {
                    <article class="tension-card">
                        <span class="tension-symbol" aria-hidden="true">"↔"</span>
                        <div><h3>{item.title}</h3><p>{item.prompt}</p></div>
                    </article>
                }
            })
            .collect_view()
    };

    let included_rows = move || {
        let included = included_ids.get();
        RECOMMENDATIONS
            .iter()
            .filter(|item| included.contains(&item.id))
            .map(|item| {
                view! {
                    <li class="brief-row">
                        <span class="brief-check" aria-hidden="true">"✓"</span>
                        <span><strong>{item.title}</strong><small>{item.room} · {item.id}</small></span>
                        <button
                            type="button"
                            class="remove-button"
                            aria-label=format!("Remove {} from brief", item.title)
                            on:click=move |_| set_included_ids.update(|ids| toggle_value(ids, item.id))
                        >"Remove"</button>
                    </li>
                }
            })
            .collect_view()
    };

    view! {
        <a class="skip-link" href="#design-brief">"Skip to your design brief"</a>
        <main class="app-shell">
            <header class="topbar">
                <a class="brand" href="#top" aria-label="Common Ground accessible home design lab">
                    <span class="brand-mark" aria-hidden="true">"cg"</span>
                    <span><strong>"common ground"</strong><small>"ACCESSIBLE HOME DESIGN LAB"</small></span>
                </a>
                <div class="prototype-status"><span class="status-dot" aria-hidden="true"></span>"Research prototype · no compliance verdict"</div>
            </header>

            <section class="hero" id="top">
                <div class="hero-copy">
                    <p class="eyebrow">"HOMES THAT ADAPT TO PEOPLE"</p>
                    <h1>"A home should fit "<em>"your life."</em></h1>
                    <p class="hero-lede">"Design around the things you want to do, the ways you move and communicate, and what makes a space feel right. Combine needs, keep your choices, and make every unknown visible."</p>
                    <div class="hero-points">
                        <span><b aria-hidden="true">"✓"</b>"More independence"</span>
                        <span><b aria-hidden="true">"✓"</b>"Resident-led decisions"</span>
                        <span><b aria-hidden="true">"✓"</b>"Safety reviewed by the right people"</span>
                    </div>
                </div>
                <aside class="hero-note">
                    <span class="note-icon" aria-hidden="true">"✳"</span>
                    <p class="eyebrow">"THE DESIGN PRINCIPLE"</p>
                    <p class="quote">"Don't ask people to fit the house. Make the house work for the people who live in it."</p>
                    <small>"A design goal — not a measured outcome"</small>
                </aside>
            </section>

            <section class="step-section" aria-labelledby="stage-heading">
                <div class="section-head">
                    <span class="step-number">"01"</span>
                    <div><p class="eyebrow">"WHERE ARE YOU STARTING?"</p><h2 id="stage-heading">"Choose the project context."</h2><p class="section-lede">"This changes the next-step guidance, not the resident's right to define the brief."</p></div>
                </div>
                <div class="stage-selector" role="group" aria-label="Home project context">{stage_cards}</div>
                <div class="stage-guidance"><span class="guidance-icon" aria-hidden="true">"↗"</span><p>{move || project_stage_guidance(project_stage.get())}</p></div>
            </section>

            <section class="step-section" aria-labelledby="needs-heading">
                <div class="section-head">
                    <span class="step-number">"02"</span>
                    <div><p class="eyebrow">"YOUR LIFE, YOUR CHOICES"</p><h2 id="needs-heading">"Which daily needs should we design around?"</h2><p class="section-lede">"Choose as many or as few as you like. These are optional design dimensions, not diagnoses. The suggestions combine when needs overlap."</p></div>
                </div>
                <div class="needs-grid">{need_cards}</div>
                <div class="selection-status" role="status" aria-live="polite">
                    <span class="selection-count">{move || selected_needs.get().len()}</span>
                    <span>{move || if selected_needs.get().is_empty() { "No individual dimensions selected. The universal planning baseline remains available." } else { "dimensions selected. Recommendations are prompts; you decide what belongs in your brief." }}</span>
                    <button type="button" class="clear-selection" on:click=move |_| set_selected_needs.set(Vec::new())>"Clear choices"</button>
                </div>
            </section>

            <section class="step-section" aria-labelledby="recommendations-heading">
                <div class="section-head">
                    <span class="step-number">"03"</span>
                    <div><p class="eyebrow">"ROOM BY ROOM · TASK BY TASK"</p><h2 id="recommendations-heading">"Build your design questions."</h2><p class="section-lede">"The universal baseline always stays. Choose needs above to see additional ideas. Nothing is considered verified just because it appears here."</p></div>
                </div>
                <div class="notice-line"><span class="notice-icon" aria-hidden="true">"i"</span><p><strong>"No generic 'accessible' badge."</strong>" Each card states a real task, why it matters, what evidence is missing, and who should review it."</p></div>
                <div class="recommendations-grid">{recommendation_cards}</div>
            </section>

            <section class="step-section tensions-section" aria-labelledby="tensions-heading">
                <div class="section-head">
                    <span class="step-number">"04"</span>
                    <div><p class="eyebrow">"INTERSECTIONS MATTER"</p><h2 id="tensions-heading">"Catch conflicts before they become barriers."</h2><p class="section-lede">"Some useful adaptations interact. These are design questions to work through with the resident and qualified professionals, not automatic answers."</p></div>
                </div>
                <div class="tensions-list">{tension_cards}</div>
                <button type="button" class="text-button" aria-expanded=move || if show_all_tensions.get() { "true" } else { "false" } on:click=move |_| set_show_all_tensions.update(|value| *value = !*value)>
                    {move || if show_all_tensions.get() { "Hide all additional design tensions" } else { "Review all design tensions" }}<span aria-hidden="true">"⌄"</span>
                </button>
                <Show when=move || show_all_tensions.get()>
                    <div class="all-tensions">
                        {DESIGN_TENSIONS.iter().map(|tension| {
                            let item = *tension;
                            view! { <div class="all-tension-row"><strong>{item.title}</strong><p>{item.prompt}</p></div> }
                        }).collect_view()}
                    </div>
                </Show>
            </section>

            <section class="step-section brief-section" id="design-brief" aria-labelledby="brief-heading">
                <div class="section-head">
                    <span class="step-number">"05"</span>
                    <div><p class="eyebrow">"YOUR BRIEF, YOUR AUTHORITY"</p><h2 id="brief-heading">"Keep the requirements you choose."</h2><p class="section-lede">"Included items can be removed at any time. Your brief keeps the evidence questions and who should review each item."</p></div>
                </div>
                <div class="brief-layout">
                    <div class="brief-panel">
                        <div class="brief-panel-head"><div><p class="eyebrow">"DESIGN BRIEF"</p><h3>"Your included prompts"</h3></div><span class="count-tag">{move || format!("{} items", included_ids.get().len())}</span></div>
                        <ul class="brief-list">{included_rows}</ul>
                        <button type="button" class="button-primary" on:click=move |_| set_show_brief.update(|visible| *visible = !*visible)>
                            {move || if show_brief.get() { "Hide copy-ready brief" } else { "Generate copy-ready brief" }}<span aria-hidden="true">"→"</span>
                        </button>
                        <Show when=move || show_brief.get()>
                            <label class="brief-label" for="brief-export">"Select and copy this draft"</label>
                            <textarea id="brief-export" readonly prop:value=move || brief_text(&selected_needs.get(), &included_ids.get(), project_stage.get())></textarea>
                            <p class="small-note">"This contains functional design dimensions you selected, not a medical diagnosis. It is a planning brief only."</p>
                        </Show>
                    </div>
                    <aside class="boundary-panel">
                        <div class="boundary-symbol" aria-hidden="true">"!"</div>
                        <h3>"What this brief does not prove"</h3>
                        <p>"It is not a construction drawing, building-code decision, accessibility certification, medical recommendation, structural assessment or life-safety approval."</p>
                        <ul><li>"Measure the actual site and clear space."</li><li>"Try the real task with the resident."</li><li>"Review applicable local requirements."</li><li>"Use qualified professionals for structure, electrical, fire and other regulated work."</li></ul>
                        <div class="authority-note"><strong>"Resident first."</strong><p>"The resident chooses what independence, support, privacy and comfort mean for them. An app, family member, model or designer cannot infer consent."</p></div>
                    </aside>
                </div>
            </section>

            <footer class="footer">
                <div><strong>"common ground"</strong><p>"Better homes begin with listening."</p></div>
                <p class="footer-disclaimer">"Synthetic design prompts · No sensitive personal data saved · No dimensions certified · No code or safety verdict"</p>
            </footer>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
