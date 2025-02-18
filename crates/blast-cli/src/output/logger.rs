use console::{style, Term};
use std::fmt::Display;

pub struct Logger {
    term: Term,
}

#[derive(Debug, Clone, Copy)]
pub enum HealthStatus {
    Good,
    Okay,
    Bad,
}

impl HealthStatus {
    pub fn from_resource_usage(cpu_percent: f32, memory_percent: f32, disk_percent: f32) -> Self {
        if cpu_percent > 90.0 || memory_percent > 90.0 || disk_percent > 90.0 {
            HealthStatus::Bad
        } else if cpu_percent > 70.0 || memory_percent > 70.0 || disk_percent > 70.0 {
            HealthStatus::Okay
        } else {
            HealthStatus::Good
        }
    }

    pub fn color(&self) -> console::Style {
        match self {
            HealthStatus::Good => style().green(),
            HealthStatus::Okay => style().yellow(),
            HealthStatus::Bad => style().red(),
        }
    }
}

impl Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Good => write!(f, "good"),
            HealthStatus::Okay => write!(f, "okay"),
            HealthStatus::Bad => write!(f, "bad"),
        }
    }
}

impl Logger {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
        }
    }

    pub fn header(&self, text: &str) {
        let width = self.term.size().1 as usize;
        let padding = "=".repeat((width - text.len() - 2) / 2);
        println!("\n{} {} {}\n", padding, style(text).bold(), padding);
    }

    pub fn section(&self, text: &str) {
        println!("\n{}", style(text).bold().underlined());
    }

    pub fn status(&self, label: &str, status: HealthStatus) {
        println!("{}: {}", 
            style(label).bold(),
            status.color().apply_to(status.to_string())
        );
    }

    pub fn info(&self, label: &str, value: impl Display) {
        println!("{}: {}", style(label).bold(), value);
    }

    pub fn resource(&self, label: &str, used: u64, total: u64) {
        let percentage = (used as f32 / total as f32) * 100.0;
        let status = if percentage > 90.0 {
            HealthStatus::Bad
        } else if percentage > 70.0 {
            HealthStatus::Okay
        } else {
            HealthStatus::Good
        };

        println!("{}: {} / {} ({:.1}%) {}",
            style(label).bold(),
            self.format_bytes(used),
            self.format_bytes(total),
            percentage,
            status.color().apply_to("●")
        );
    }

    pub fn warning(&self, text: impl Display) {
        println!("{} {}", 
            style("WARNING:").yellow().bold(),
            text
        );
    }

    pub fn error(&self, text: impl Display) {
        println!("{} {}", 
            style("ERROR:").red().bold(),
            text
        );
    }

    fn format_bytes(&self, bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1}GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1}MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1}KB", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }
} 