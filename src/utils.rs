use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_current_time() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        .to_string()
}
