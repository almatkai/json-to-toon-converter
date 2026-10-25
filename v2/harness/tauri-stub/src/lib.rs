//! Headless type boundary only. No window, OS clipboard or desktop app exists.
#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit<T>(&self, channel: &str, payload: T) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit<T>(&self, _: &str, _: T) -> Result<(), String> {
        panic!("headless tests must never emit through a Tauri AppHandle")
    }
}
