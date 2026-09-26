use crate::{
    app::Route,
    components::PageShell,
    db::{delete_note, get_note, relative_time},
    state::AppState,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::Pencil;
use g3_cache::use_cached;
use g3_route_transitions::animated_navigate;
use g3_ui::{Button, ButtonStyle, Card, ConfirmModal, RightSlot, Spinner, StatusColor};

/// A pushed page: it covers the tab bar, so it renders its own shell with a
/// back button rather than sitting inside `AppShell`.
///
/// `#[transition(pushed)]` on the route is what makes arriving here slide
/// left and leaving slide right. The component itself knows nothing about it.
#[component]
pub fn NoteDetail(id: String) -> Element {
    let mut app_state = use_context::<AppState>();
    let navigator = use_navigator();
    let mut confirm_delete = use_signal(|| false);

    // Cached on the device, keyed by the server function and its arguments:
    // reopening a note shows it at once and refetches behind it, and a
    // navigation from one note straight to another (same component, new `id`)
    // fetches the new one. `bump_data` after an edit marks it stale.
    let note = use_cached(get_note, (id.clone(),));

    let body = match &*note.read() {
        None => rsx! { Spinner { center: true } },
        // `get_note` answers `None` both for a note that never existed and
        // for one belonging to somebody else — see the `owner` clause in
        // `src/db/note.rs`. The screen cannot tell them apart, and should
        // not: saying "not yours" would confirm the id exists.
        Some(Ok(None)) => rsx! {
            Card { title: "Note not found",
                p { class: "g3-message-text g3-message-text-muted", "It may have been deleted." }
            }
        },
        Some(Err(error)) => rsx! {
            Card { title: "Could not load this note",
                p { class: "g3-message-text g3-message-text-muted", "{error}" }
            }
        },
        Some(Ok(Some(note))) => rsx! {
            Card {
                title: note.title.clone(),
                right_slot: RightSlot::Text(relative_time(&note.updated_at)),
                if note.body.is_empty() {
                    p { class: "g3-message-text g3-message-text-subtle", "This note has no body yet." }
                } else {
                    // `whitespace-pre-wrap` so the line breaks the author
                    // typed survive; without it several paragraphs render as
                    // one run-on block.
                    p { class: "whitespace-pre-wrap", "{note.body}" }
                }
            }

            Button {
                style: ButtonStyle::Danger,
                expand: true,
                onclick: move |_| confirm_delete.set(true),
                "Delete note"
            }
        },
    };

    let edit_id = id.clone();
    let delete_id = id.clone();

    rsx! {
        PageShell {
            title: "Note",
            end_button: rsx! {
                Button {
                    style: ButtonStyle::Clear,
                    aria_label: "Edit",
                    onclick: move |_| {
                        spawn(animated_navigate(Route::EditNote { id: edit_id.clone() }));
                    },
                    Pencil { size: 20 }
                }
            },
            {body}
        }

        ConfirmModal {
            open: confirm_delete,
            title: "Delete this note?",
            description: rsx! { "This cannot be undone." },
            confirm_text: "Delete",
            on_confirm: move |_| {
                let delete_id = delete_id.clone();
                spawn(async move {
                    match delete_note(delete_id).await {
                        Ok(()) => {
                            app_state.bump_data();
                            app_state.show_toast("Note deleted.", StatusColor::Neutral);
                            // `replace`, not an animated pop: the note this
                            // page is about no longer exists, so leaving a
                            // history entry pointing at it would make browser
                            // back land on a "not found" screen.
                            navigator.replace(Route::Notes { filter: None });
                        }
                        Err(error) => app_state.show_toast(
                            format!("Could not delete the note: {error}"),
                            StatusColor::Danger,
                        ),
                    }
                });
            },
        }
    }
}
