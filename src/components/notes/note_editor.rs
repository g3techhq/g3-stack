use crate::{
    app::Route,
    components::{
        BackButton,
        shared::{LoadFailed, error_message},
    },
    data_change::DataChange,
    db::{BODY_MAX, TITLE_MAX, create_note, get_note, update_note, validate_note},
    state::AppState,
};
use dioxus::prelude::*;
use dioxus_icons::lucide::FileX;
use g3_cache::use_cached;
use g3_route_transitions::animated_back_or_navigate;
use g3_ui::{
    Button, Content, ContentWidth, EmptyState, Header, Input, Spinner, Stack, TextArea, use_toast,
};

/// New-note sheet. `layer = sheet` on the route makes it rise from the
/// bottom.
#[component]
pub fn NewNote() -> Element {
    rsx! {
        NoteForm { id: None, title: String::new(), body: String::new() }
    }
}

/// Edit sheet for an existing note: loads the note, then hands it to the
/// form.
///
/// Split this way so the draft lives in a component of its own. Typing
/// re-renders the form, not this loader, and the form's signals start from
/// the note it was given, so there is no "fill the form once the note
/// arrives" effect to guard against a refetch overwriting what was typed.
#[component]
pub fn EditNote(id: ReadSignal<String>) -> Element {
    let note = use_cached(get_note, (id(),));

    let header = rsx! {
        Header { title: "Edit note", start: rsx! { BackButton {} } }
    };

    match &*note.read() {
        None => rsx! {
            {header}
            Content { Spinner { center: true } }
        },
        Some(Err(_)) => rsx! {
            {header}
            Content { LoadFailed {} }
        },
        Some(Ok(None)) => rsx! {
            {header}
            Content {
                EmptyState {
                    title: "Note not found",
                    icon: rsx! { FileX { size: 40 } },
                    "It may have been deleted."
                }
            }
        },
        // Keyed by the note, so opening another note's editor starts a fresh
        // draft rather than keeping this one's.
        Some(Ok(Some(existing))) => rsx! {
            NoteForm {
                key: "{id}",
                id: Some(id()),
                title: existing.title.clone(),
                body: existing.body.clone(),
            }
        },
    }
}

/// One form for both. `id: None` creates, `id: Some(..)` updates.
///
/// Worth keeping as one component rather than two: the validation, the
/// disabled state, the error handling, and the save-then-leave flow are
/// identical, and the only real difference is which server function runs.
///
/// `title` and `body` seed the draft once; `use_signal`'s initializer does
/// not rerun, which is what a draft wants.
#[component]
fn NoteForm(id: Option<String>, title: String, body: String) -> Element {
    let mut app_state = use_context::<AppState>();
    let toast = use_toast();
    let title = use_signal(|| title);
    let body = use_signal(|| body);
    let mut saving = use_signal(|| false);
    let is_edit = id.is_some();

    let save = move |_| {
        // The same `validate_note` the server runs. Checking here as well is
        // a courtesy that saves a round trip — it is not the control. The
        // server never trusts this.
        let (clean_title, clean_body) = match validate_note(&title.read(), &body.read()) {
            Ok(values) => values,
            Err(message) => {
                toast.error(message);
                return;
            }
        };

        let id = id.clone();
        saving.set(true);
        spawn(async move {
            let (result, leave_to) = match id {
                Some(id) => (
                    update_note(id.clone(), clean_title, clean_body).await.map(|_| ()),
                    Route::NoteDetail { id },
                ),
                None => (
                    create_note(clean_title, clean_body).await.map(|_| ()),
                    Route::Notes { filter: None },
                ),
            };

            match result {
                Ok(()) => {
                    app_state.changed(DataChange::Notes);
                    toast.success("Note saved.");
                    // Dismisses downward, because the route is a sheet — the
                    // animation is the route's, not this component's.
                    animated_back_or_navigate(leave_to).await;
                }
                Err(error) => {
                    saving.set(false);
                    toast.error(format!("Could not save the note: {}", error_message(&error)));
                }
            }
        });
    };

    rsx! {
        Header {
            title: if is_edit { "Edit note" } else { "New note" },
            start: rsx! { BackButton {} },
            end: rsx! {
                Button { loading: saving(), onclick: save, "Save" }
            },
        }
        Content { width: ContentWidth::Readable,
            Stack {
                Input {
                    label: "Title",
                    value: title,
                    placeholder: "What is this about?",
                    maxlength: TITLE_MAX,
                    disabled: saving(),
                }
                TextArea {
                    label: "Body",
                    value: body,
                    rows: 10,
                    placeholder: "Write it down.",
                    maxlength: BODY_MAX,
                    disabled: saving(),
                }
            }
        }
    }
}
