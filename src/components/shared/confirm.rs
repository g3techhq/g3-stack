use g3_ui::{AlertButton, AlertOptions, Alerts};

/// Asks before something that cannot be undone, with the confirm button in
/// the danger color. Resolves to `true` when the user confirms.
///
/// `Alerts::confirm` draws an ordinary confirm button; a delete or a sign-out
/// that loses data should look like one.
pub async fn confirm_destructive(
    alerts: Alerts,
    title: &str,
    message: &str,
    confirm_label: &str,
) -> bool {
    let buttons = vec![
        AlertButton::cancel("Cancel"),
        AlertButton::destructive(confirm_label),
    ];
    alerts
        .show(AlertOptions {
            title: title.to_string(),
            message: Some(message.to_string()),
            buttons: buttons.clone(),
            input: None,
        })
        .await
        .confirmed(&buttons)
}
