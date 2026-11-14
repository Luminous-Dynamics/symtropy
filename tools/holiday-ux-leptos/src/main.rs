// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

mod model;

use leptos::{mount::mount_to_body, prelude::*};
use model::{ACTIVITIES, DESTINATIONS, INTENTS, Pace, day_load_label, fit_reason, recovery_message, toggle_choice};

#[component]
fn App() -> impl IntoView {
    let (intent_ids, set_intent_ids) = signal(vec!["rest", "nature"]);
    let (pace, set_pace) = signal(Pace::Unhurried);
    let (destination_id, set_destination_id) = signal("moss");
    let (activity_ids, set_activity_ids) = signal(vec!["garden", "food"]);
    let (show_access_detail, set_show_access_detail) = signal(false);
    let (closure_active, set_closure_active) = signal(false);
    let (recovery, set_recovery) = signal(None::<&'static str>);

    let intent_chips = INTENTS.iter().copied().map(|intent| {
        view! {
            <button
                type="button"
                class="intent-chip"
                class:active=move || intent_ids.get().contains(&intent.id)
                aria-pressed=move || if intent_ids.get().contains(&intent.id) { "true" } else { "false" }
                on:click=move |_| set_intent_ids.update(|values| toggle_choice(values, intent.id))
            >
                <span class="intent-symbol" aria-hidden="true">{intent.symbol}</span>
                <span class="intent-copy"><strong>{intent.label}</strong><small>{intent.detail}</small></span>
                <span class="intent-check" aria-hidden="true">"✓"</span>
            </button>
        }
    }).collect_view();

    let pace_cards = Pace::ALL.iter().copied().map(|option| {
        view! {
            <button
                type="button"
                class="pace-option"
                class:active=move || pace.get() == option
                aria-pressed=move || if pace.get() == option { "true" } else { "false" }
                on:click=move |_| set_pace.set(option)
            >
                <strong>{option.label()}</strong>
                <span>{option.detail()}</span>
            </button>
        }
    }).collect_view();

    let destination_cards = DESTINATIONS.iter().copied().map(|destination| {
        view! {
            <article class="destination-card" class:chosen=move || destination_id.get() == destination.id>
                <div class=format!("destination-art art-{}", destination.art_class)>
                    <span class="art-symbol" aria-hidden="true">{destination.symbol}</span>
                    <span class="art-label">"CONCEPT DESTINATION"</span>
                </div>
                <div class="destination-content">
                    <div class="destination-heading">
                        <div><p class="eyebrow">{destination.region}</p><h3>{destination.name}</h3></div>
                        <span class="price">{destination.estimate}</span>
                    </div>
                    <p class="destination-headline">{destination.headline}</p>
                    <p class="destination-summary">{destination.summary}</p>
                    <div class="access-state"><span class="state-dot" aria-hidden="true"></span><span>{destination.access_label}</span></div>
                    <p class="mini-note">{destination.duration}</p>
                    <button
                        type="button"
                        class="select-destination"
                        aria-label=format!("Choose {}", destination.name)
                        aria-pressed=move || if destination_id.get() == destination.id { "true" } else { "false" }
                        on:click=move |_| set_destination_id.set(destination.id)
                    >
                        {move || if destination_id.get() == destination.id { "Selected for comparison" } else { "Explore this option" }}
                        <span aria-hidden="true">"→"</span>
                    </button>
                </div>
            </article>
        }
    }).collect_view();

    let activity_rows = ACTIVITIES.iter().copied().map(|activity| {
        view! {
            <div class="activity-row" class:included=move || activity_ids.get().contains(&activity.id)>
                <div class="activity-time">{activity.time}</div>
                <div class="activity-main">
                    <div class="activity-topline"><h3>{activity.title}</h3><span class="activity-kind">{activity.kind}</span></div>
                    <p>{activity.detail}</p>
                    <small>{activity.effort}</small>
                </div>
                <button
                    type="button"
                    class="activity-toggle"
                    aria-pressed=move || if activity_ids.get().contains(&activity.id) { "true" } else { "false" }
                    on:click=move |_| set_activity_ids.update(|values| toggle_choice(values, activity.id))
                >
                    {move || if activity_ids.get().contains(&activity.id) { "In plan ✓" } else { "Add to plan +"}}
                </button>
            </div>
        }
    }).collect_view();

    view! {
        <a class="skip-link" href="#trip-plan">"Skip to your draft plan"</a>
        <main class="app-shell">
            <header class="topbar">
                <a class="brand" href="#top" aria-label="Daywell holiday design prototype">
                    <span class="brand-mark" aria-hidden="true">"d."</span>
                    <span><strong>"daywell"</strong><small>"HOLIDAY DESIGN LAB"</small></span>
                </a>
                <div class="topbar-status"><span class="status-pip" aria-hidden="true"></span>"Synthetic prototype · no live bookings"</div>
            </header>

            <section class="hero" id="top">
                <div class="hero-copy">
                    <p class="eyebrow hero-eyebrow">"A MORE HUMAN KIND OF TRAVEL PLANNING"</p>
                    <h1>"Make room for a "<em>"good"</em>" holiday."</h1>
                    <p class="hero-lede">"Start with how you want your time away to feel. Build a plan that fits your energy, your people, and your budget — with space for the unexpected."</p>
                    <div class="hero-promises">
                        <span><b aria-hidden="true">"✓"</b>"Your pace, your call"</span>
                        <span><b aria-hidden="true">"✓"</b>"Unknowns stay visible"</span>
                        <span><b aria-hidden="true">"✓"</b>"Rest is a valid plan"</span>
                    </div>
                </div>
                <aside class="hero-note">
                    <span class="note-mark" aria-hidden="true">"✳"</span>
                    <p class="eyebrow">"A DIFFERENT SUCCESS METRIC"</p>
                    <p class="hero-note-quote">"The goal isn't to fit more in. It's to make more of the time you have."</p>
                    <small>"Design principle · not a measured outcome"</small>
                </aside>
            </section>

            <section class="section-block" aria-labelledby="intent-heading">
                <div class="section-title">
                    <div class="step-number">"01"</div>
                    <div><p class="eyebrow">"YOUR TIME AWAY"</p><h2 id="intent-heading">"What would make this worthwhile?"</h2><p class="section-subtitle">"Choose any that fit. Change them whenever you like."</p></div>
                </div>
                <div class="intent-grid">{intent_chips}</div>
                <div class="pace-panel">
                    <div class="pace-heading">
                        <div><h3>"Choose your rhythm"</h3><p>"A good plan respects the energy you want to spend."</p></div>
                        <span class="small-tag">"EDITABLE ANYTIME"</span>
                    </div>
                    <div class="pace-grid">{pace_cards}</div>
                </div>
            </section>

            <section class="section-block" aria-labelledby="destinations-heading">
                <div class="section-title">
                    <div class="step-number">"02"</div>
                    <div><p class="eyebrow">"POSSIBLE WORLDS"</p><h2 id="destinations-heading">"Three ways your time could feel"</h2><p class="section-subtitle">"Fictional concepts with illustrative prices, not real offers or ranked recommendations."</p></div>
                </div>
                <div class="destination-grid">{destination_cards}</div>
                <div class="evidence-banner">
                    <span class="evidence-icon" aria-hidden="true">"i"</span>
                    <p><strong>"Honesty over false certainty."</strong>" Prices, access notes, and resource considerations are synthetic examples. An unknown is not a promise that something will work for you."</p>
                </div>
            </section>

            <section class="section-block trip-section" id="trip-plan" aria-labelledby="plan-heading">
                <div class="section-title">
                    <div class="step-number">"03"</div>
                    <div><p class="eyebrow">"YOUR WORKING PLAN"</p><h2 id="plan-heading">"A rhythm, not a timetable prison."</h2><p class="section-subtitle">"Add, remove, or leave the day open. Nothing here is booked."</p></div>
                </div>
                <div class="plan-layout">
                    <div class="plan-main">
                        <div class="day-card">
                            <div class="day-heading">
                                <div><p class="eyebrow">"DAY ONE · A DRAFT"</p><h3>"A little structure, lots of breathing room"</h3></div>
                                <span class="load-pill">{move || day_load_label(pace.get(), activity_ids.get().len())}</span>
                            </div>
                            <div class="activity-list">{activity_rows}</div>
                            <div class="open-time">
                                <span class="open-time-icon" aria-hidden="true">"☼"</span>
                                <div><strong>"Protected open time"</strong><p>"Keep a gap for rest, a long lunch, a surprise, or nothing at all."</p></div>
                                <span class="open-time-state">"Always allowed"</span>
                            </div>
                        </div>

                        <div class="recovery-card">
                            <div class="recovery-header">
                                <div class="recovery-icon" aria-hidden="true">"↺"</div>
                                <div><p class="eyebrow">"REAL LIFE HAPPENS"</p><h3>"What if your afternoon closes?"</h3><p>"Try a disruption. The original plan stays intact until you choose."</p></div>
                            </div>
                            <Show when=move || closure_active.get() fallback=|| view! {
                                <button type="button" class="button button-dark" on:click=move |_| {
                                    set_closure_active.set(true);
                                    set_recovery.set(None);
                                }>
                                    "Simulate a closure" <span aria-hidden="true">"→"</span>
                                </button>
                            }>
                                <div class="recovery-options">
                                    <button type="button" class="recovery-option" on:click=move |_| set_recovery.set(Some("rest"))>
                                        <span class="recovery-choice-symbol" aria-hidden="true">"☾"</span><strong>"Protect the afternoon"</strong><small>"No replacement purchase"</small>
                                    </button>
                                    <button type="button" class="recovery-option" on:click=move |_| set_recovery.set(Some("makers"))>
                                        <span class="recovery-choice-symbol" aria-hidden="true">"✳"</span><strong>"Consider an indoor studio"</strong><small>"Availability unknown · +R 230 example"</small>
                                    </button>
                                    <button type="button" class="recovery-option" on:click=move |_| set_recovery.set(Some("defer"))>
                                        <span class="recovery-choice-symbol" aria-hidden="true">"↗"</span><strong>"Move it to tomorrow"</strong><small>"Check opening and weather first"</small>
                                    </button>
                                </div>
                                <p class="recovery-feedback" role="status" aria-live="polite">{move || recovery_message(recovery.get())}</p>
                                <button type="button" class="text-button" on:click=move |_| {
                                    set_closure_active.set(false);
                                    set_recovery.set(None);
                                }>"Reset scenario"</button>
                            </Show>
                        </div>
                    </div>

                    <aside class="summary-card" aria-labelledby="summary-heading">
                        <div class="summary-top">
                            <p class="eyebrow">"CURRENTLY EXPLORING"</p>
                            <h3 id="summary-heading">{move || DESTINATIONS.iter().find(|item| item.id == destination_id.get()).map(|item| item.name).unwrap_or("Choose a concept")}</h3>
                            <p>{move || DESTINATIONS.iter().find(|item| item.id == destination_id.get()).map(|item| item.headline).unwrap_or("Your draft plan")}</p>
                        </div>
                        <div class="summary-line"><span>"Synthetic estimate"</span><strong>{move || DESTINATIONS.iter().find(|item| item.id == destination_id.get()).map(|item| item.estimate).unwrap_or("Not selected")}</strong></div>
                        <p class="summary-fineprint">"3 nights · per person · illustrative only. Dates, party size, taxes, transport, and extras are not priced."</p>
                        <div class="summary-line"><span>"Your rhythm"</span><strong>{move || pace.get().label()}</strong></div>
                        <div class="summary-reason"><p class="eyebrow">"WHY IT MAY FIT"</p><p>{move || fit_reason(destination_id.get(), &intent_ids.get(), pace.get())}</p></div>
                        <button type="button" class="text-button access-toggle" aria-expanded=move || if show_access_detail.get() { "true" } else { "false" } on:click=move |_| set_show_access_detail.update(|show| *show = !*show)>
                            {move || if show_access_detail.get() { "Hide evidence notes" } else { "Review access and resource notes" }}<span aria-hidden="true">"⌄"</span>
                        </button>
                        <Show when=move || show_access_detail.get()>
                            <div class="access-detail" role="region" aria-label="Access and resource evidence notes">
                                <h4>"Access evidence"</h4>
                                <p>{move || DESTINATIONS.iter().find(|item| item.id == destination_id.get()).map(|item| item.access_note).unwrap_or("No destination selected.")}</p>
                                <h4>"Local and ecological boundaries"</h4>
                                <p>{move || DESTINATIONS.iter().find(|item| item.id == destination_id.get()).map(|item| item.resource_note).unwrap_or("No destination selected.")}</p>
                            </div>
                        </Show>
                        <div class="summary-footer"><span class="summary-footer-mark" aria-hidden="true">"✳"</span><p>"Best next step: verify real access and total cost before you commit."</p></div>
                    </aside>
                </div>
            </section>

            <footer class="footer">
                <div><strong>"daywell"</strong><p>"Less pressure. More possibility."</p></div>
                <p class="footer-disclaimer">"Prototype only · Fictional places and prices · No bookings · No live inventory · No visitor tracking"</p>
            </footer>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
