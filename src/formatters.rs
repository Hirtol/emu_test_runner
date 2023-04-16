use std::time::Duration;

pub use indicatif;
use owo_colors::{CssColors, OwoColorize};

use crate::outputs::{RunnerError, RunnerOutput, TestReport};

pub trait EmuTestResultFormatter {
    fn handle_start(&self, test_count: usize) -> anyhow::Result<()>;

    fn handle_test_progress(&self, test_complete: &Result<RunnerOutput, RunnerError>) -> anyhow::Result<()>;

    fn handle_complete(&self, report: &TestReport, time_taken: Duration) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct SimpleConsoleFormatter {
    progress: Option<indicatif::ProgressBar>,
}

impl SimpleConsoleFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_custom_progress(mut self, bar: indicatif::ProgressBar) -> Self {
        self.progress = Some(bar);
        self
    }

    pub fn with_progress(mut self, total_tests: u64) -> Self {
        self.progress = Some(indicatif::ProgressBar::new(total_tests));
        self
    }
}

impl EmuTestResultFormatter for SimpleConsoleFormatter {
    fn handle_start(&self, test_count: usize) -> anyhow::Result<()> {
        println!("=== Running {} Snapshot Tests ===\n", test_count.green());
        Ok(())
    }

    fn handle_test_progress(&self, _test_complete: &Result<RunnerOutput, RunnerError>) -> anyhow::Result<()> {
        if let Some(progress) = self.progress.as_ref() {
            progress.inc(1)
        }

        Ok(())
    }

    fn handle_complete(&self, report: &TestReport, time_taken: Duration) -> anyhow::Result<()> {
        if let Some(progress) = self.progress.as_ref() {
            progress.finish_and_clear()
        }

        if !report.errors.is_empty() {
            println!("{}", "== Found errors ==".on_red());

            for error in &report.errors {
                println!("= {}({:?}) =", error.rom_id.red(), error.rom_path);
                println!("Error: {:#?}", error.context);
                println!()
            }
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
                println!()
            }
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

        let newly_passing = report.passed.iter().filter(|p| p.context.output.is_new).count();
        if newly_passing > 0 {
            println!(
                "{: <16} {} ({} newly passing)",
                "✔ Passed:",
                report.passed.len().green(),
                newly_passing.bright_green()
            );
        } else {
            println!("{: <16} {}", "✔ Passed:", report.passed.len().green());
        }

        let new_tests = report.unchanged.iter().filter(|p| p.context.output.newly_added).count();
        if new_tests > 0 {
            println!(
                "{: <15} {} ({} new tests)",
                "😴 Same:",
                report.unchanged.len().green(),
                new_tests.green()
            );
        } else {
            println!("{: <15} {}", "😴 Same:", report.unchanged.len().green());
        }

        println!(
            "{: <15} {}",
            "🔀 Changed:",
            if report.changed.is_empty() {
                0.color(CssColors::Gray)
            } else {
                changed_len.color(CssColors::RebeccaPurple)
            }
        );

        let new_fails = report.fails.iter().filter(|p| p.context.output.is_new).count();
        if new_fails > 0 {
            println!(
                "{: <15} {} ({} new fails)",
                "❌ Failed:",
                if report.fails.is_empty() { 0.color(CssColors::Gray) } else { failed_len.color(CssColors::Red) },
                new_fails.red()
            );
        } else {
            println!(
                "{: <15} {}",
                "❌ Failed:",
                if report.fails.is_empty() { 0.color(CssColors::Gray) } else { failed_len.color(CssColors::Red) }
            );
        }

        println!(
            "{: <15} {}",
            "💀 Died:",
            if report.errors.is_empty() { 0.color(CssColors::Gray) } else { errors_len.color(CssColors::Red) }
        );

        Ok(())
    }
}
