use console::{style, Term, Style};
use std::fmt::Display;

pub struct Logger {
    term: Term,
    no_color: bool,
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

    pub fn color(&self) -> Style {
        match self {
            HealthStatus::Good => Style::new().green(),
            HealthStatus::Okay => Style::new().yellow(),
            HealthStatus::Bad => Style::new().red(),
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
            no_color: std::env::var("NO_COLOR").is_ok() 
                || std::env::var("CLICOLOR").map(|v| v == "0").unwrap_or(false)
                || std::env::var("CLICOLOR_FORCE").map(|v| v == "0").unwrap_or(false),
        }
    }

    fn style(&self, text: impl Display) -> String {
        if self.no_color {
            text.to_string()
        } else {
            style(text.to_string()).to_string()
        }
    }

    fn style_bold(&self, text: impl Display) -> String {
        if self.no_color {
            text.to_string()
        } else {
            style(text.to_string()).bold().to_string()
        }
    }

    pub fn header(&self, text: &str) {
        let width = self.term.size().1 as usize;
        let padding = "=".repeat((width - text.len() - 2) / 2);
        println!("\n{} {} {}\n", padding, self.style_bold(text), padding);
    }

    pub fn section(&self, text: &str) {
        println!("\n{}", self.style_bold(text));
    }

    pub fn status(&self, label: &str, status: HealthStatus) {
        println!("{}: {}", 
            self.style_bold(label),
            if self.no_color {
                status.to_string()
            } else {
                status.color().apply_to(status.to_string()).to_string()
            }
        );
    }

    pub fn info(&self, label: &str, value: impl Display) {
        println!("{}: {}", self.style_bold(label), value);
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

        let dot = if self.no_color {
            "●".to_string()
        } else {
            status.color().apply_to("●").to_string()
        };

        println!("{}: {} / {} ({:.1}%) {}",
            self.style_bold(label),
            self.format_bytes(used),
            self.format_bytes(total),
            percentage,
            dot
        );
    }

    pub fn success(&self, text: impl Display) {
        if self.no_color {
            println!("✓ {}", text);
        } else {
            println!("{} {}", style("✓").green().bold(), text);
        }
    }

    pub fn warning(&self, text: impl Display) {
        if self.no_color {
            println!("WARNING: {}", text);
        } else {
            println!("{} {}", 
                style("WARNING:").yellow().bold(),
                text
            );
        }
    }

    pub fn error(&self, text: impl Display) {
        if self.no_color {
            println!("ERROR: {}", text);
        } else {
            println!("{} {}", 
                style("ERROR:").red().bold(),
                text
            );
        }
    }

    pub fn progress(&self, text: impl Display) {
        print!("{} {} ... ", style("→").cyan().bold(), text);
        self.term.flush().unwrap();
    }

    pub fn done(&self) {
        println!("{}", style("done").green().bold());
    }

    pub fn failed(&self) {
        println!("{}", style("failed").red().bold());
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