use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::widget::image::Handle;

static HANDLES: OnceLock<Mutex<HashMap<String, Handle>>> = OnceLock::new();

/// The handle for an asset. Cheap to call: it clones an id, not the pixels.
pub fn asset_handle(file_path: &str) -> Handle {
    let cache = HANDLES.get_or_init(|| Mutex::new(HashMap::new()));

    cache
        .lock()
        .unwrap()
        .entry(file_path.to_owned())
        .or_insert_with(|| {
            let bytes = std::fs::read(file_path)
                .unwrap_or_else(|e| panic!("failed to read asset {file_path}: {e}"));
            Handle::from_bytes(bytes)
        })
        .clone()
}
