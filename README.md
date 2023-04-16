# EmuTest
A test running framework for permissive snapshot testing between different frames produced
by an emulator. This can be useful for automatically running test-roms without register indicators for
failed tests, for example.

# Example

```rust
use emu_test_runner::formatters::simple::SimpleConsoleFormatter;
use emu_test_runner::options::EmuRunnerOptions;
use emu_test_runner::outputs::FrameOutput;
use emu_test_runner::EmuTestRunner;

let formatter = Box::new(SimpleConsoleFormatter::new().with_progress(tests.len() as u64));
let options = EmuRunnerOptions::default();
let runner = EmuTestRunner::new(formatter, options)?;

let tests = emu_test_runner::inputs::TestCandidate::find_all_in_directory("./test_roms", ".gba")?

// `run_rom` would be your emulator of choice for running the provided rom_data
runner.run_tests(tests.into_iter(), |test, rom_data| run_rom(test, rom_data));

```