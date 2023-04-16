use crate::outputs::{TestOutput, TestOutputType, TestReport};
use crate::OutputDestinations;
use owo_colors::{CssColors, OwoColorize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub trait EmuTestResultFormatter {
    fn handle_start(&self, test_count: usize) -> anyhow::Result<()>;

    fn handle_test_progress(&self, test_complete: &TestOutput) -> anyhow::Result<()>;

    fn handle_complete(&self, report: &TestReport, time_taken: Duration) -> anyhow::Result<()>;
}

pub struct SimpleConsoleFormatter {
    output_path: PathBuf,
    progress: Option<indicatif::ProgressBar>,
}

impl SimpleConsoleFormatter {
    pub fn new(output_path: impl Into<PathBuf>) -> Self {
        Self {
            output_path: output_path.into(),
            progress: None,
        }
    }

    pub fn with_progress(mut self, bar: indicatif::ProgressBar) -> Self {
        self.progress = Some(bar);
        self
    }
}

impl EmuTestResultFormatter for SimpleConsoleFormatter {
    fn handle_start(&self, test_count: usize) -> anyhow::Result<()> {
        println!("=== Running {} Snapshot Tests ===\n", test_count.green());
        Ok(())
    }

    fn handle_test_progress(&self, _test_complete: &TestOutput) -> anyhow::Result<()> {
        if let Some(progress) = self.progress.as_ref() {
            progress.inc(1)
        }

        Ok(())
    }

    fn handle_complete(&self, report: &TestReport, time_taken: Duration) -> anyhow::Result<()> {
        if !report.errors.is_empty() {
            println!("{}", "== Found errors ==".on_red());

            for error in &report.errors {
                println!("= {}({:?}) =", error.rom_id.red(), error.rom_path);
                println!("Error: {:#?}", error.context);
                println!()
            }

            println!()
        }

        if !report.fails.is_empty() {
            println!("{}\n", "== Found failures ==".on_color(CssColors::DarkCyan));

            for fail in &report.fails {
                println!("= {}({:?}) =", fail.rom_id.color(CssColors::DarkCyan), fail.rom_path);
                println!("Failed snapshot test",);
                println!("Was: {:?}", fail.context.output.failure_path);
                println!("Expected: {:?}", fail.context.output.snapshot_path);
                println!()
            }

            println!()
        }

        if !report.changed.is_empty() {
            println!("{}\n", "== Found Changes ==".on_color(CssColors::RebeccaPurple));

            for change in &report.changed {
                println!(
                    "= {}({:?}) =",
                    change.rom_id.color(CssColors::RebeccaPurple),
                    change.rom_path
                );
                println!("Changed: {:?}", change.context.output.changed_path);
            }

            println!()
        }

        let changed_len = report.changed.len();
        let failed_len = report.fails.len();
        let errors_len = report.errors.len();

        // Final Report
        println!(
            "=== Report - Ran {} Tests in {:.2?} ===",
            report.test_outputs.len().green(),
            time_taken.purple()
        );

        let no_longer_failing = report
            .passed
            .iter()
            .flat_map(|p| OutputDestinations::Old.compare(&p.rom_id, &self.output_path, OutputDestinations::New))
            .filter(|equal| !*equal)
            .count();

        if no_longer_failing > 0 {
            println!(
                "{: <16} {} ({} no longer failing)",
                "✔ Passed:",
                report.passed.len().green(),
                no_longer_failing.bright_green()
            );
        } else {
            println!("{: <16} {}", "✔ Passed:", report.passed.len().green());
        }

        println!("{: <15} {}", "😴 Same:", report.unchanged.len().green());
        println!(
            "{: <15} {}",
            "🔀 Changed:",
            if report.changed.is_empty() {
                0.color(CssColors::Gray)
            } else {
                changed_len.color(CssColors::RebeccaPurple)
            }
        );

        println!(
            "{: <15} {}",
            "❌ Failed:",
            if report.fails.is_empty() { 0.color(CssColors::Gray) } else { failed_len.color(CssColors::Red) }
        );
        println!(
            "{: <15} {}",
            "💀 Died:",
            if report.errors.is_empty() { 0.color(CssColors::Gray) } else { errors_len.color(CssColors::Red) }
        );

        Ok(())
    }
}

impl SimpleConsoleFormatter {
    pub fn handle_start(&self, test_roms: &[impl AsRef<Path>]) {
        println!("=== Running {} Snapshot Tests ===\n", test_roms.len().green())
    }

    pub fn handle_complete_tests(&self, report: &TestReport, time_taken: Duration) {
        if !report.errors.is_empty() {
            println!("{}", "== Found errors ==".on_red());

            for error in &report.errors {
                println!("= {}({:?}) =", error.rom_id.red(), error.rom_path);
                println!("Error: {:#?}", error.context);
                println!()
            }

            println!()
        }

        if !report.fails.is_empty() {
            println!("{}\n", "== Found failures ==".on_color(CssColors::DarkCyan));

            for fail in &report.fails {
                println!("= {}({:?}) =", fail.rom_id.color(CssColors::DarkCyan), fail.rom_path);
                println!("Failed snapshot test",);
                println!("Was: {:?}", fail.context.output.failure_path);
                println!("Expected: {:?}", fail.context.output.snapshot_path);
                println!()
            }

            println!()
        }

        if !report.changed.is_empty() {
            println!("{}\n", "== Found Changes ==".on_color(CssColors::RebeccaPurple));

            for change in &report.changed {
                println!(
                    "= {}({:?}) =",
                    change.rom_id.color(CssColors::RebeccaPurple),
                    change.rom_path
                );
                println!("Changed: {:?}", change.context.output.changed_path);
            }

            println!()
        }

        let changed_len = report.changed.len();
        let failed_len = report.fails.len();
        let errors_len = report.errors.len();

        // Final Report
        println!(
            "=== Report - Ran {} Tests in {:.2?} ===",
            report.test_outputs.len().green(),
            time_taken.purple()
        );

        let no_longer_failing = report
            .passed
            .iter()
            .flat_map(|p| OutputDestinations::Old.compare(&p.rom_id, &self.output_path, OutputDestinations::New))
            .filter(|equal| !*equal)
            .count();

        if no_longer_failing > 0 {
            println!(
                "{: <16} {} ({} no longer failing)",
                "✔ Passed:",
                report.passed.len().green(),
                no_longer_failing.bright_green()
            );
        } else {
            println!("{: <16} {}", "✔ Passed:", report.passed.len().green());
        }

        println!("{: <15} {}", "😴 Same:", report.unchanged.len().green());
        println!(
            "{: <15} {}",
            "🔀 Changed:",
            if report.changed.is_empty() {
                0.color(CssColors::Gray)
            } else {
                changed_len.color(CssColors::RebeccaPurple)
            }
        );

        println!(
            "{: <15} {}",
            "❌ Failed:",
            if report.fails.is_empty() { 0.color(CssColors::Gray) } else { failed_len.color(CssColors::Red) }
        );
        println!(
            "{: <15} {}",
            "💀 Died:",
            if report.errors.is_empty() { 0.color(CssColors::Gray) } else { errors_len.color(CssColors::Red) }
        );
    }
}
