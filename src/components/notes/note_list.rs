use crate::{
    app::{NotesFilter, Route},
    components::shared::{LoadFailed, error_message, peek_row},
    data_change::DataChange,
    db::{Note, list_notes, record_key, relative_time, set_note_pinned},
    state::AppState,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{NotebookPen, Pin, PinOff};
use g3_cache::{update_cached, use_cached};
use g3_route_transitions::animated_navigate;
use g3_ui::{
    Button, ButtonFill, Color, EmptyState, Item, ItemDetail, List, ListLines, ListVariant,
    Refresher, SegmentButton, SegmentGroup, Spinner, SwipeAction, SwipeItem, use_toast,
};

/// The All/Pinned segmented control in the shell's header.
///
/// Lives here rather than in `AppShell` because it belongs to this screen —
/// the shell just renders whatever toolbar the current route asks for.
///
/// The filter is the route's, so the group only reports a pick
/// (`defer_selection`) and follows the route back, rather than keeping a copy
/// of its own that could disagree with the URL. The effect is what moves it
/// on browser Back: `filter` is a `ReadSignal`, so reading it inside the
/// effect subscribes, and the effect reruns when the parent passes a new
/// value.
#[component]
pub fn NotesToolbar(filter: ReadSignal<NotesFilter>, on_change: Callback<NotesFilter>) -> Element {
    let mut selected = use_signal(|| *filter.peek());
    use_effect(move || selected.set(filter()));

    rsx! {
        SegmentGroup {
            value: selected,
            aria_label: "Filter notes",
            defer_selection: true,
            onchange: move |filter| on_change.call(filter),
            SegmentButton { value: NotesFilter::All, "All" }
            SegmentButton { value: NotesFilter::Pinned, "Pinned" }
        }
    }
}

/// The list screen.
///
/// `filter` is a `ReadSignal` because the memo below reads it, so the list
/// reshapes when the route's filter changes without the screen remounting.
#[component]
pub fn Notes(filter: ReadSignal<Option<NotesFilter>>) -> Element {
    let mut app_state = use_context::<AppState>();
    let toast = use_toast();

    // Cached on the device: reopening the app shows the last list at once and
    // refetches behind it. `app_state.changed(DataChange::Notes)` after any
    // edit marks it stale (see `src/data_change.rs`).
    let notes = use_cached(list_notes, ());

    // Which rows the filter keeps, as indices into the cached list. Rerun
    // when the list or the filter changes, not on every render, and it copies
    // no note: the rows below borrow them from the cache.
    let shown = use_memo(move || {
        let pinned_only = filter().unwrap_or_default() == NotesFilter::Pinned;
        match &*notes.read() {
            Some(Ok(all)) => all
                .iter()
                .enumerate()
                .filter(|(_, note)| !pinned_only || note.pinned)
                .map(|(index, _)| index)
                .collect(),
            _ => Vec::new(),
        }
    });

    // Rows capture only their index (a `usize`, which is `Copy`) and look up
    // the note when tapped, instead of cloning an id into two closures per
    // row on every render.
    let open = move |index: usize| {
        if let Some(id) = peek_row(&notes, index, |note: &Note| record_key(&note.id)) {
            spawn(animated_navigate(Route::NoteDetail { id }));
        }
    };

    let toggle_pin = move |index: usize| {
        let Some((id, pinned)) = peek_row(&notes, index, |note: &Note| {
            (record_key(&note.id), !note.pinned)
        }) else {
            return;
        };
        // Instant feedback: edit the cached list, then ask the server, then
        // reconcile. The refetch `changed` starts replaces the guess with what
        // the server holds, which also undoes it if the save failed.
        update_cached(list_notes, (), |notes: &mut Vec<Note>| {
            if let Some(note) = notes.get_mut(index) {
                note.pinned = pinned;
            }
        });
        spawn(async move {
            // "Set to", not "toggle": a device showing a stale list cannot
            // undo a change another device made.
            if let Err(error) = set_note_pinned(id, pinned).await {
                toast.error(format!(
                    "Could not update the note: {}",
                    error_message(&error)
                ));
            }
            app_state.changed(DataChange::Notes);
        });
    };

    // Hooks are all above this line. Everything below branches, and a hook
    // called after an early return runs on some renders and not others.
    let notes_read = notes.read();
    let all = match notes_read.as_ref() {
        None => return rsx! { Spinner { center: true } },
        Some(Err(_)) => return rsx! { LoadFailed {} },
        Some(Ok(all)) => all,
    };

    if shown.read().is_empty() {
        let pinned_only = filter().unwrap_or_default() == NotesFilter::Pinned;
        let title = if pinned_only {
            "No pinned notes"
        } else {
            "No notes yet"
        };
        return rsx! {
            EmptyState {
                title,
                icon: rsx! { NotebookPen { size: 40 } },
                action: rsx! {
                    Button {
                        fill: ButtonFill::Outline,
                        onclick: move |_| animated_navigate(Route::NewNote {}),
                        "Write a note"
                    }
                },
                if pinned_only {
                    "Pin a note from its page, or swipe it to the right."
                } else {
                    "Notes you write show up here."
                }
            }
        };
    }

    rsx! {
        Refresher {
            refreshing: notes.pending(),
            on_refresh: move |_| notes.refresh(),
            List { variant: ListVariant::Raised, lines: ListLines::Inset,
                for &index in shown.read().iter() {
                    {
                        let note = &all[index];
                        rsx! {
                            SwipeItem {
                                key: "{record_key(&note.id)}",
                                start_actions: rsx! {
                                    SwipeAction {
                                        color: Color::Accent,
                                        onclick: move |_| toggle_pin(index),
                                        if note.pinned { "Unpin" } else { "Pin" }
                                    }
                                },
                                Item {
                                    start: rsx! {
                                        if note.pinned {
                                            Pin { size: 18, color: "var(--g3-color-accent)" }
                                        } else {
                                            PinOff { size: 18, color: "var(--g3-color-text-secondary)" }
                                        }
                                    },
                                    label: note.title.clone(),
                                    description: relative_time(&note.updated_at),
                                    detail: ItemDetail::Show,
                                    onclick: move |_| open(index),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
