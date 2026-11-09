use crate::ui::{NoticeGlobals, RootWindow};
use slint::ComponentHandle;

#[derive(Clone, Copy)]
enum NotificationType {
    Error,
    Warning,
    Info,
}

pub fn error(ui_weak: &slint::Weak<RootWindow>, message: impl Into<String>) {
    report(ui_weak, message.into(), NotificationType::Error);
}

pub fn warn(ui_weak: &slint::Weak<RootWindow>, message: impl Into<String>) {
    report(ui_weak, message.into(), NotificationType::Warning);
}

pub fn info(ui_weak: &slint::Weak<RootWindow>, message: impl Into<String>) {
    report(ui_weak, message.into(), NotificationType::Info);
}

fn report(ui_weak: &slint::Weak<RootWindow>, message: String, notification_type: NotificationType) {
    match notification_type {
        NotificationType::Error => {
            log::error!("{message}");
        }
        NotificationType::Warning => {
            log::warn!("{message}");
        }
        NotificationType::Info => {
            log::info!("{message}");
        }
    }

    // Most failures are raised from a callback's background task, and a property written
    // from there never reaches a repaint: the notice has to be posted to the UI loop.
    let ui_weak = ui_weak.clone();
    let posted = slint::invoke_from_event_loop(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        let notice = ui.global::<NoticeGlobals>();

        // Both flags on every report: a warning must not inherit the error of the message before it.
        let (is_error, is_warn) = match notification_type {
            NotificationType::Error => (true, false),
            NotificationType::Warning => (false, true),
            NotificationType::Info => (false, false),
        };

        notice.set_is_error(is_error);
        notice.set_is_warn(is_warn);
        notice.set_text(message.into());
    });

    if let Err(err) = posted {
        log::error!("could not post a notice to the UI: {err}");
    }
}
