use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;


pub fn hash_str(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}
