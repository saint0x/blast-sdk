use console::{style, Style};
use std::fmt::Display;
use std::io::Write;

pub struct Logger {
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
            no_color: std::env::var("NO_COLOR").is_ok() 
                || std::env::var("CLICOLOR").map(|v| v == "0").unwrap_or(false)
                || std::env::var("CLICOLOR_FORCE").map(|v| v == "0").unwrap_or(false),
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
        println!("\n{}\n", self.style_bold(text));
    }

    pub fn section(&self, text: &str) {
        println!("\n{}", self.style_bold(text));
    }

    pub fn status(&self, label: &str, status: HealthStatus) {
        let status_str = if self.no_color {
            status.to_string()
        } else {
            status.color().apply_to(status.to_string()).to_string()
        };
        println!("{}: {}", self.style_bold(label), status_str);
    }

    pub fn info(&self, label: &str, value: impl Display) {
        println!("{}: {}", self.style_bold(label), value);
    }

    pub fn resource(&self, label: &str, used: u64, total: u64) {
        let percent = (used as f64 / total as f64) * 100.0;
        let status = HealthStatus::from_resource_usage(
            percent as f32,
            percent as f32,
            percent as f32,
        );
        let status_str = if self.no_color {
            format!("{:.1}%", percent)
        } else {
            status.color().apply_to(format!("{:.1}%", percent)).to_string()
        };
        println!(
            "{}: {} ({} / {})",
            self.style_bold(label),
            status_str,
            self.format_bytes(used),
            self.format_bytes(total),
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
            println!("{} {}", style("WARNING:").yellow().bold(), text);
        }
    }

    pub fn error(&self, text: impl Display) {
        if self.no_color {
            eprintln!("ERROR: {}", text);
        } else {
            eprintln!("{} {}", style("ERROR:").red().bold(), text);
        }
    }

    pub fn progress(&self, text: impl Display) {
        if self.no_color {
            print!("→ {} ... ", text);
        } else {
            print!("{} {} ... ", style("→").cyan().bold(), text);
        }
        std::io::stdout().flush().unwrap();
    }

    pub fn done(&self) {
        if self.no_color {
            println!("done");
        } else {
            println!("{}", style("done").green().bold());
        }
    }

    pub fn failed(&self) {
        if self.no_color {
            println!("failed");
        } else {
            println!("{}", style("failed").red().bold());
        }
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