use crate::{
    app::Route,
    components::PageShell,
    db::{BODY_MAX, TITLE_MAX, create_note, get_note, update_note, validate_note},
    state::AppState,
};
use dioxus::prelude::*;
use g3_route_transitions::animated_go_back;
use g3_ui::{Button, Field, Spinner, StatusColor};

/// New-note sheet. `#[transition(cover)]` makes it rise from the bottom.
#[component]
pub fn NewNote() -> Element {
    rsx! {
        NoteEditor { id: None }
    }
}

/// Edit sheet for an existing note.
#[component]
pub fn EditNote(id: String) -> Element {
    rsx! {
        NoteEditor { id: Some(id) }
    }
}

/// One form for both. `id: None` creates, `id: Some(..)` updates.
///
/// Worth keeping as one component rather than two: the validation, the
/// disabled state, the error handling, and the save-then-leave flow are
/// identical, and the only real difference is which server function runs.
#[component]
fn NoteEditor(id: Option<String>) -> Element {
    let mut app_state = use_context::<AppState>();
    let mut title = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut loaded = use_signal(|| false);

    let existing = use_resource(use_reactive!(|id| async move {
        match id {
            Some(id) => get_note(id).await,
            None => Ok(None),
        }
    }));

    // Fills the form once, the first time the note arrives. Guarded by
    // `loaded` rather than running on every read: without it, a refetch
    // triggered by anything else would overwrite whatever the user has typed
    // since with the stored copy.
    use_effect(move || {
        if loaded() {
            return;
        }
        if let Some(Ok(Some(note))) = &*existing.read() {
            title.set(note.title.clone());
            body.set(note.body.clone());
            loaded.set(true);
        }
    });

    let is_edit = id.is_some();
    let waiting_for_existing = is_edit && !loaded();

    let save = move |_| {
        // The same `validate_note` the server runs. Checking here as well is
        // a courtesy that saves a round trip — it is not the control. The
        // server never trusts this.
        let (clean_title, clean_body) = match validate_note(&title(), &body()) {
            Ok(values) => values,
            Err(message) => {
                app_state.show_toast(message, StatusColor::Danger);
                return;
            }
        };

        let id = id.clone();
        saving.set(true);
        spawn(async move {
            let result = match id {
                Some(id) => update_note(id, clean_title, clean_body).await.map(|_| ()),
                None => create_note(clean_title, clean_body).await.map(|_| ()),
            };

            match result {
                Ok(()) => {
                    app_state.bump_data();
                    app_state.show_toast("Note saved.", StatusColor::Success);
                    // Dismisses downward, because the route is a `cover` —
                    // the animation is the route's, not this component's.
                    animated_go_back(Route::Notes { filter: None }).await;
                }
                Err(error) => {
                    saving.set(false);
                    app_state.show_toast(
                        format!("Could not save the note: {error}"),
                        StatusColor::Danger,
                    );
                }
            }
        });
    };

    rsx! {
        PageShell {
            title: if is_edit { "Edit note" } else { "New note" },
            end_button: rsx! {
                Button { disabled: saving() || waiting_for_existing, onclick: save, "Save" }
            },
            if waiting_for_existing {
                Spinner { center: true }
            } else {
                Field {
                    label: "Title",
                    value: title,
                    placeholder: "What is this about?",
                    maxlength: TITLE_MAX,
                    disabled: saving(),
                }
                Field {
                    label: "Body",
                    value: body,
                    multiline: true,
                    rows: 10,
                    placeholder: "Write it down.",
                    maxlength: BODY_MAX,
                    disabled: saving(),
                }
            }
        }
    }
}
