use ::aw::cookie::Key;

#[cfg(debug_assertions)]
pub fn key() -> Key {
    Key::from(&[8; 64])
}

#[cfg(not(debug_assertions))]
pub fn key() -> Key {
    use ::base64::{engine::general_purpose::STANDARD, Engine as _};

    let key = std::env::vars()
        .find(|(k, _)| k == "SESSION_KEY")
        .map(|(_, v)| v)
        .expect("SESSION_KEY must be set");

    let key = STANDARD
        .decode(key)
        .expect("SESSION_KEY must be in base64 format");
    assert_eq!(key.len(), 64, "SESSION_KEY must have 64 bytes");
    Key::from(&key)
}
