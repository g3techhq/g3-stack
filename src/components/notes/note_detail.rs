use crate::{
    app::Route,
    components::{
        BackButton,
        shared::{LoadFailed, confirm_destructive, error_message},
    },
    data_change::DataChange,
    db::{delete_note, get_note, relative_time},
    state::AppState,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{FileX, Pencil};
use g3_cache::use_cached;
use g3_route_transitions::animated_navigate;
use g3_ui::{
    Button, ButtonExpand, ButtonFill, Card, Color, Content,
    ContentWidth, EmptyState, Header, Spinner, Text, TextTone, use_alert, use_toast,
};

/// A pushed page. It renders inside `AppShell`, so the rail stays put, but
/// supplies its own header with a back button.
///
/// `layer = stack_page` on the route is what makes arriving here slide in and
/// leaving slide back. The component itself knows nothing about it.
///
/// `id` is a `ReadSignal`: `use_cached` reads it, so a navigation from one
/// note straight to another (same component, new `id`) fetches the new one.
#[component]
pub fn NoteDetail(id: ReadSignal<String>) -> Element {
    let mut app_state = use_context::<AppState>();
    let navigator = use_navigator();
    let toast = use_toast();
    let alerts = use_alert();

    // Cached on the device, keyed by the server function and its arguments:
    // reopening a note shows it at once and refetches behind it.
    let note = use_cached(get_note, (id(),));

    let delete = move |_| async move {
        if !confirm_destructive(alerts, "Delete this note?", "This cannot be undone.", "Delete")
            .await
        {
            return;
        }
        match delete_note(id()).await {
            Ok(()) => {
                app_state.changed(DataChange::Notes);
                toast.show("Note deleted.");
                // `replace`, not an animated pop: the note this page is about
                // no longer exists, so leaving a history entry pointing at it
                // would make browser Back land on a "not found" screen.
                navigator.replace(Route::Notes { filter: None });
            }
            Err(error) => {
                toast.error(format!("Could not delete the note: {}", error_message(&error)));
            }
        }
    };

    let header = rsx! {
        Header {
            title: "Note",
            start: rsx! { BackButton {} },
            end: rsx! {
                Button {
                    fill: ButtonFill::Clear,
                    aria_label: "Edit",
                    onclick: move |_| animated_navigate(Route::EditNote { id: id() }),
                    Pencil { size: 20 }
                }
            },
        }
    };

    let note_read = note.read();
    let note = match note_read.as_ref() {
        None => {
            return rsx! {
                {header}
                Content { Spinner { center: true } }
            };
        }
        Some(Err(_)) => {
            return rsx! {
                {header}
                Content { LoadFailed {} }
            };
        }
        // `get_note` answers `None` both for a note that never existed and
        // for one belonging to somebody else — see the `owner` clause in
        // `src/db/note.rs`. The screen cannot tell them apart, and should
        // not: saying "not yours" would confirm the id exists.
        Some(Ok(None)) => {
            return rsx! {
                {header}
                Content {
                    EmptyState {
                        title: "Note not found",
                        icon: rsx! { FileX { size: 40 } },
                        "It may have been deleted."
                    }
                }
            };
        }
        Some(Ok(Some(note))) => note,
    };

    rsx! {
        {header}
        Content { width: ContentWidth::Readable,
            Card {
                title: note.title.clone(),
                end: rsx! {
                    Text { tone: TextTone::Secondary, {relative_time(&note.updated_at)} }
                },
                if note.body.is_empty() {
                    Text { tone: TextTone::Tertiary, "This note has no body yet." }
                } else {
                    // `whitespace-pre-wrap` so the line breaks the author
                    // typed survive; without it several paragraphs render as
                    // one run-on block.
                    Text { class: "whitespace-pre-wrap", "{note.body}" }
                }
            }

            Button {
                fill: ButtonFill::Outline,
                color: Color::Danger,
                expand: ButtonExpand::Block,
                onclick: delete,
                "Delete note"
            }
        }
    }
}
