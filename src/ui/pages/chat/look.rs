use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const AVATAR: f32 = 38.0;

pub fn initials_of(name: &str) -> String {
    name.chars().take(2).collect::<String>().to_uppercase()
}

pub fn name_hue(name: &str) -> f32 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    (hasher.finish() % 360) as f32
}
