use crate::{
    app::{NotesFilter, Route},
    db::{list_notes, record_key, relative_time, set_note_pinned},
    state::AppState,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Pin, PinOff};
use g3_route_transitions::animated_navigate;
use g3_ui::{
    Button, ButtonStyle, Card, Item, ItemDetail, List, ListLines, Refresher, SegmentButton,
    SegmentGroup, Spinner, StatusColor, SwipeAction, SwipeItem, SwipeSide,
};

/// The All/Pinned segmented control in the shell's header.
///
/// Lives here rather than in `AppShell` because it belongs to this screen —
/// the shell just renders whatever toolbar the current route asks for.
#[component]
pub fn NotesToolbar(filter: NotesFilter, on_change: Callback<NotesFilter>) -> Element {
    let mut active = use_signal(|| filter.segment_index());

    // Syncs the indicator when `filter` changes for a reason other than this
    // toolbar's own `on_change` — browser back/forward, or a navigation from
    // elsewhere. A `key` on the component would also work but forces a
    // remount, and a remounted SegmentGroup starts already in its new
    // position instead of sliding to it.
    use_effect(use_reactive!(|filter| {
        active.set(filter.segment_index());
    }));

    rsx! {
        SegmentGroup {
            active,
            on_change: move |index| on_change.call(NotesFilter::from_segment_index(index)),
            SegmentButton { index: 0, "All" }
            SegmentButton { index: 1, "Pinned" }
        }
    }
}

/// The list screen.
#[component]
pub fn Notes(filter: Option<NotesFilter>) -> Element {
    let filter = filter.unwrap_or_default();
    let mut app_state = use_context::<AppState>();

    // `data_version` is read inside the closure purely to register the
    // dependency, so this refetches after a mutation made from any other
    // screen. Without it, saving a note in the editor and coming back here
    // would show the list as it was before the save.
    let mut notes = use_resource(move || {
        let _ = (app_state.data_version)();
        async move { list_notes().await }
    });

    // Hooks are all above this line. Everything below branches, and a hook
    // called inside a branch panics the first time the branch changes.
    let notes_read = notes.read();
    let all = match notes_read.as_ref() {
        None => return rsx! { Spinner { center: true } },
        Some(Err(error)) => {
            return rsx! {
                Card { title: "Could not load your notes",
                    p { class: "g3-message-text g3-message-text-muted", "{error}" }
                    Button { style: ButtonStyle::Outline, onclick: move |_| notes.restart(), "Try again" }
                }
            };
        }
        Some(Ok(all)) => all,
    };

    // Filtering client-side is fine at this size and keeps one query serving
    // both segments. Push it into the SurrealQL `WHERE` clause when the list
    // outgrows a single fetch.
    let rows: Vec<_> = all
        .iter()
        .filter(|note| filter != NotesFilter::Pinned || note.pinned)
        .cloned()
        .collect();
    drop(notes_read);

    if rows.is_empty() {
        let (title, body) = match filter {
            NotesFilter::Pinned => ("No pinned notes", "Swipe a note to the right to pin it."),
            NotesFilter::All => ("No notes yet", "Notes you write show up here."),
        };
        return rsx! {
            Card { title,
                p { class: "g3-message-text g3-message-text-muted", "{body}" }
                Button {
                    style: ButtonStyle::Outline,
                    onclick: move |_| {
                        spawn(animated_navigate(Route::NewNote {}));
                    },
                    "Write a note"
                }
            }
        };
    }

    rsx! {
        Refresher {
            refreshing: notes.pending(),
            on_refresh: move |_| notes.restart(),
            List { inset: true, lines: ListLines::Inset,
                for note in rows {
                    {
                        let id = record_key(&note.id);
                        let pinned = note.pinned;
                        let open_id = id.clone();
                        let toggle_id = id.clone();
                        rsx! {
                            SwipeItem {
                                key: "{id}",
                                start_actions: rsx! {
                                    SwipeAction {
                                        side: SwipeSide::Start,
                                        accent: true,
                                        onclick: move |_| {
                                            let toggle_id = toggle_id.clone();
                                            spawn(async move {
                                                match set_note_pinned(toggle_id, !pinned).await {
                                                    Ok(_) => app_state.bump_data(),
                                                    Err(error) => app_state.show_toast(
                                                        format!("Could not update the note: {error}"),
                                                        StatusColor::Danger,
                                                    ),
                                                }
                                            });
                                        },
                                        if pinned { "Unpin" } else { "Pin" }
                                    }
                                },
                                Item {
                                    start: rsx! {
                                        if pinned {
                                            Pin { size: 18, color: "var(--color-focused)" }
                                        } else {
                                            PinOff { size: 18, color: "var(--color-label-secondary)" }
                                        }
                                    },
                                    label: note.title.clone(),
                                    description: relative_time(&note.updated_at),
                                    detail: ItemDetail::Show,
                                    onclick: move |_| {
                                        spawn(animated_navigate(Route::NoteDetail { id: open_id.clone() }));
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
