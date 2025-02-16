use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use console::{Term, style};

/// User interface style configuration
#[derive(Debug, Clone)]
pub struct UiStyle {
    /// Color scheme for different elements
    pub colors: ColorScheme,
    /// Progress bar style
    pub progress_style: ProgressStyle,
    /// Whether to use unicode characters
    pub use_unicode: bool,
    /// Whether to use colors
    pub use_colors: bool,
}

/// Color scheme for UI elements
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// Primary color for main elements
    pub primary: Color,
    /// Secondary color for less important elements
    pub secondary: Color,
    /// Success color for completed operations
    pub success: Color,
    /// Error color for failures
    pub error: Color,
    /// Warning color for potential issues
    pub warning: Color,
    /// Info color for general information
    pub info: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            info: Color::White,
        }
    }
}

/// Progress indicator types
#[derive(Debug, Clone)]
pub enum ProgressType {
    /// Simple spinner
    Spinner,
    /// Progress bar with percentage
    Bar,
    /// Download progress with speed
    Download,
    /// Installation progress with steps
    Installation,
}

/// User interface manager
#[derive(Debug)]
pub struct UiManager {
    style: UiStyle,
    term: Term,
    multi_progress: MultiProgress,
    stdout: StandardStream,
}

impl UiManager {
    /// Create a new UI manager
    pub fn new() -> Self {
        let style = UiStyle {
            colors: ColorScheme::default(),
            progress_style: ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("=>-"),
            use_unicode: true,
            use_colors: true,
        };

        Self {
            style,
            term: Term::stdout(),
            multi_progress: MultiProgress::new(),
            stdout: StandardStream::stdout(ColorChoice::Auto),
        }
    }

    /// Create a new progress indicator
    pub fn create_progress(&self, total: u64, progress_type: ProgressType) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(total));
        
        let style = match progress_type {
            ProgressType::Spinner => ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
            ProgressType::Bar => self.style.progress_style.clone(),
            ProgressType::Download => ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}) {msg}")
                .unwrap(),
            ProgressType::Installation => ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {prefix:.bold.dim} {msg}")
                .unwrap(),
        };

        pb.set_style(style);
        pb
    }

    /// Print a colored message
    pub fn print_colored(&mut self, message: &str, color: Color) -> std::io::Result<()> {
        if self.style.use_colors {
            self.stdout.set_color(ColorSpec::new().set_fg(Some(color)))?;
            write!(self.stdout, "{}", message)?;
            self.stdout.reset()?;
        } else {
            write!(self.stdout, "{}", message)?;
        }
        Ok(())
    }

    /// Print a success message
    pub fn success(&mut self, message: &str) -> std::io::Result<()> {
        self.print_colored("✓ ", self.style.colors.success)?;
        writeln!(self.stdout, "{}", message)
    }

    /// Print an error message
    pub fn error(&mut self, message: &str) -> std::io::Result<()> {
        self.print_colored("✗ ", self.style.colors.error)?;
        writeln!(self.stdout, "{}", message)
    }

    /// Print a warning message
    pub fn warning(&mut self, message: &str) -> std::io::Result<()> {
        self.print_colored("! ", self.style.colors.warning)?;
        writeln!(self.stdout, "{}", message)
    }

    /// Print an info message
    pub fn info(&mut self, message: &str) -> std::io::Result<()> {
        self.print_colored("i ", self.style.colors.info)?;
        writeln!(self.stdout, "{}", message)
    }

    /// Create an interactive error navigator
    pub fn error_navigator(&mut self, errors: Vec<crate::diagnostics::Diagnostic>) -> std::io::Result<()> {
        if errors.is_empty() {
            return Ok(());
        }

        let mut current = 0;
        loop {
            self.term.clear_screen()?;
            
            // Display current error
            let error = &errors[current];
            self.print_colored("Error Navigator ", self.style.colors.primary)?;
            writeln!(self.stdout, "({}/{})", current + 1, errors.len())?;
            writeln!(self.stdout)?;

            // Print error details
            self.print_colored("Error: ", self.style.colors.error)?;
            writeln!(self.stdout, "{}", error.message)?;
            
            if let Some(details) = &error.details {
                writeln!(self.stdout)?;
                self.print_colored("Details: ", self.style.colors.secondary)?;
                writeln!(self.stdout, "{}", details)?;
            }

            // Print suggestions
            if !error.suggestions.is_empty() {
                writeln!(self.stdout)?;
                self.print_colored("Suggestions:", self.style.colors.info)?;
                for suggestion in &error.suggestions {
                    writeln!(self.stdout)?;
                    writeln!(self.stdout, "  • {}", suggestion.description)?;
                    if let Some(fix) = &suggestion.fix {
                        writeln!(self.stdout, "    Try: {}", fix)?;
                    }
                }
            }

            // Navigation instructions
            writeln!(self.stdout)?;
            writeln!(self.stdout, "Navigation: [p]revious | [n]ext | [q]uit")?;

            // Handle input
            match self.term.read_char()? {
                'p' | 'P' if current > 0 => current -= 1,
                'n' | 'N' if current < errors.len() - 1 => current += 1,
                'q' | 'Q' => break,
                _ => {}
            }
        }

        self.term.clear_screen()?;
        Ok(())
    }
}

/// Machine-readable output formatter
#[derive(Debug)]
pub struct MachineOutput {
    format: OutputFormat,
}

/// Output format for machine-readable output
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    /// JSON output
    Json,
    /// YAML output
    Yaml,
    /// Custom format
    Custom,
}

impl MachineOutput {
    /// Create a new machine output formatter
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    /// Format data for machine consumption
    pub fn format<T: serde::Serialize>(&self, data: &T) -> Result<String, serde_json::Error> {
        match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(data),
            OutputFormat::Yaml => Ok(serde_yaml::to_string(data).unwrap()),
            OutputFormat::Custom => Ok(format!("{:?}", data)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel, DiagnosticCategory, DiagnosticSuggestion};

    #[test]
    fn test_ui_manager() {
        let mut ui = UiManager::new();
        
        // Test progress bar creation
        let pb = ui.create_progress(100, ProgressType::Bar);
        pb.set_message("Testing progress");
        pb.inc(50);
        pb.finish_with_message("Done");

        // Test colored output
        ui.success("Operation completed successfully").unwrap();
        ui.error("An error occurred").unwrap();
        ui.warning("Warning message").unwrap();
        ui.info("Information message").unwrap();
    }

    #[test]
    fn test_machine_output() {
        let formatter = MachineOutput::new(OutputFormat::Json);
        
        let data = serde_json::json!({
            "status": "success",
            "message": "Operation completed",
            "data": {
                "value": 42
            }
        });

        let output = formatter.format(&data).unwrap();
        assert!(output.contains("success"));
        assert!(output.contains("42"));
    }
} 