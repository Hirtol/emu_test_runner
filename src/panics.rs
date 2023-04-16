use std::collections::HashMap;
use std::sync::Mutex;
use std::thread::ThreadId;

use once_cell::sync::Lazy;

pub static PANIC_BUFFER: Lazy<Mutex<HashMap<ThreadId, Vec<PanicCorrelation>>>> = Lazy::new(Mutex::default);

#[derive(Debug)]
pub struct PanicCorrelation {
    panic_msg: String,
    rom_name: Option<String>,
}

pub fn correlate(rom_name: &str) -> String {
    let thread = std::thread::current().id();
    let mut buffer = PANIC_BUFFER.lock().unwrap();
    let new_buffer = buffer.get_mut(&thread).unwrap();
    let item = new_buffer.last_mut().unwrap();
    item.rom_name = Some(rom_name.to_string());
    item.panic_msg.clone()
}

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
                rom_name: None,
            };
            let nested = global_buffer.entry(thread.id()).or_default();
            nested.push(correlation);
        })
    });

    let out = function();

    std::panic::set_hook(hook);

    out
}
