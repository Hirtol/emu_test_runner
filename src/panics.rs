use std::collections::HashMap;
use std::sync::Mutex;
use std::thread::ThreadId;

use once_cell::sync::Lazy;

pub static PANIC_BUFFER: Lazy<Mutex<HashMap<ThreadId, Vec<PanicCorrelation>>>> = Lazy::new(Mutex::default);

#[derive(Debug)]
pub struct PanicCorrelation {
    panic_msg: String,
}

/// Returns the message of the most recent panic on the caller's thread.
///
/// # Returns
///
/// The message of the latest panic
pub fn latest_panic() -> Option<String> {
    let thread = std::thread::current().id();
    let mut buffer = PANIC_BUFFER.lock().ok()?;
    let new_buffer = buffer.get_mut(&thread)?;

    let item = new_buffer.last_mut()?;
    Some(item.panic_msg.clone())
}

/// Run the given closure in a custom panic handler which saves the panic message for later correlation
/// to the particular emulator run that caused it.
///
/// Note that [std::panic::catch_unwind] is still required to be able to correlate the panic, as otherwise the thread
/// will have died and no correlation would be possible any more.
pub fn run_in_custom_handler<R>(function: impl FnOnce() -> R) -> R {
    let hook = std::panic::take_hook();

    std::panic::set_hook({
        Box::new(move |info| {
            let mut global_buffer = PANIC_BUFFER.lock().unwrap();
            let msg = match info.payload().downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => match info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<dyn Any>",
                },
            };

            let thread = std::thread::current();
            let correlation = PanicCorrelation {
                panic_msg: msg.to_string(),
            };
            let nested = global_buffer.entry(thread.id()).or_default();
            nested.push(correlation);
        })
    });

    let out = function();

    std::panic::set_hook(hook);

    out
}
