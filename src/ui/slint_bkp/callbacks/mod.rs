mod builder_page;
mod character_page;
mod chat_page;
mod main_page;
pub mod register;

use uuid::Uuid;

pub(crate) fn parse_id(id: &str) -> Option<Uuid> {
    Uuid::parse_str(id)
        .map_err(|_| log::error!("invalid id: {id}"))
        .ok()
}
