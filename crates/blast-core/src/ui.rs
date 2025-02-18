use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use console::Term;
use serde::{Serialize, Deserialize};
use crate::logging::{LogLevel, StructuredLogger};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::error::BlastResult;

/// User interface style configuration
#[derive(Clone)]
pub struct UiStyle {
    /// Color scheme for different elements
    pub colors: ColorScheme,
    /// Progress bar style
    #[allow(dead_code)]
    progress_style: ProgressStyle,
    /// Whether to use unicode characters
    pub use_unicode: bool,
    /// Whether to use colors
    pub use_colors: bool,
}

impl std::fmt::Debug for UiStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiStyle")
            .field("colors", &self.colors)
            .field("use_unicode", &self.use_unicode)
            .field("use_colors", &self.use_colors)
            // Skip progress_style since it doesn't implement Debug
            .finish()
    }
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
    pub fn format<T: Serialize + std::fmt::Debug>(&self, data: &T) -> Result<String, serde_json::Error> {
        match self.format {
            OutputFormat::Json => serde_json::to_string_pretty(data),
            OutputFormat::Yaml => Ok(serde_yaml::to_string(data).unwrap()),
            OutputFormat::Custom => Ok(format!("{data:?}")),
        }
    }
}

/// UI theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Primary color (hex)
    pub primary_color: String,
    /// Secondary color (hex)
    pub secondary_color: String,
    /// Success color (hex)
    pub success_color: String,
    /// Warning color (hex)
    pub warning_color: String,
    /// Error color (hex)
    pub error_color: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary_color: "#4A90E2".to_string(),
            secondary_color: "#86C1B9".to_string(),
            success_color: "#7ED321".to_string(),
            warning_color: "#F5A623".to_string(),
            error_color: "#D0021B".to_string(),
        }
    }
}

/// Progress indicator for long-running operations
#[derive(Debug)]
pub struct ProgressIndicator {
    bar: ProgressBar,
}

impl ProgressIndicator {
    /// Create a new progress indicator
    pub fn new(total: u64, message: &str) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        
        bar.set_message(message.to_string());
        
        Self { bar }
    }

    /// Update progress
    pub fn update(&self, progress: u64) {
        self.bar.set_position(progress);
    }

    /// Set progress message
    pub fn set_message(&self, message: &str) {
        self.bar.set_message(message.to_string());
    }

    /// Mark as finished
    pub fn finish(&self) {
        self.bar.finish_with_message("Done!");
    }
}

/// Console output formatter
#[derive(Debug)]
pub struct Console {
    stdout: StandardStream,
    theme: Theme,
}

impl Console {
    /// Create a new console with default theme
    pub fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
            theme: Theme::default(),
        }
    }

    /// Set console theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Print success message
    pub fn success(&mut self, message: &str) -> BlastResult<()> {
        self.stdout.reset()?;
        self.stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Green)))?;
        writeln!(&mut self.stdout, "✓ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }

    /// Print warning message
    pub fn warning(&mut self, message: &str) -> BlastResult<()> {
        self.stdout.reset()?;
        self.stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Yellow)))?;
        writeln!(&mut self.stdout, "⚠ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }

    /// Print error message
    pub fn error(&mut self, message: &str) -> BlastResult<()> {
        self.stdout.reset()?;
        self.stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Red)))?;
        writeln!(&mut self.stdout, "✗ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }

    /// Print info message
    pub fn info(&mut self, message: &str) -> BlastResult<()> {
        self.stdout.reset()?;
        self.stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Blue)))?;
        writeln!(&mut self.stdout, "ℹ {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }

    /// Print debug message
    pub fn debug(&mut self, message: &str) -> BlastResult<()> {
        self.stdout.reset()?;
        self.stdout.set_color(ColorSpec::new().set_fg(Some(termcolor::Color::Magenta)))?;
        writeln!(&mut self.stdout, "🔍 {}", message)?;
        self.stdout.reset()?;
        Ok(())
    }

    /// Create a new progress indicator
    pub fn progress(&self, total: u64, message: &str) -> ProgressIndicator {
        ProgressIndicator::new(total, message)
    }

    /// Clear the screen
    pub fn clear(&mut self) -> BlastResult<()> {
        Term::stdout().clear_screen()?;
        Ok(())
    }

    /// Move cursor up
    pub fn move_up(&mut self, lines: u16) -> BlastResult<()> {
        Term::stdout().move_cursor_up(lines.into())?;
        Ok(())
    }

    /// Move cursor down
    pub fn move_down(&mut self, lines: u16) -> BlastResult<()> {
        Term::stdout().move_cursor_down(lines.into())?;
        Ok(())
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

/// Format data for display
pub trait DisplayFormat {
    /// Format data as string
    fn format(&self) -> String;
}

impl<T: std::fmt::Display + Serialize + std::fmt::Debug> DisplayFormat for T {
    fn format(&self) -> String {
        format!("{self}")
    }
}

pub fn init_logging(level: LogLevel) -> StructuredLogger {
    let tracing_level: Level = level.clone().into();
    
    // Set up tracing subscriber with JSON formatting
    tracing_subscriber::fmt()
        .with_level(true)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::FULL)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string()))
        .with_env_filter(tracing_level.to_string())
        .json()
        .flatten_event(true)
        .try_init()
        .expect("Failed to set global subscriber");

    StructuredLogger::new(level)
} 