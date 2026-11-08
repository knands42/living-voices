use crate::ui::{NoticeGlobals, RootWindow};
use slint::ComponentHandle;

pub fn error(ui_weak: &slint::Weak<RootWindow>, message: impl Into<String>) {
    report(ui_weak, message.into(), true);
}

pub fn info(ui_weak: &slint::Weak<RootWindow>, message: impl Into<String>) {
    report(ui_weak, message.into(), false);
}

fn report(ui_weak: &slint::Weak<RootWindow>, message: String, is_error: bool) {
    if is_error {
        log::error!("{message}");
    } else {
        log::info!("{message}");
    }

    let Some(ui) = ui_weak.upgrade() else {
        return;
    };

    let notice = ui.global::<NoticeGlobals>();
    notice.set_is_error(is_error);
    notice.set_text(message.into());
}
