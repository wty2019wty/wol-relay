//! Command-line interface definition and validation.

use clap::Parser;

/// Parsed command-line arguments.
#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// Log level
    #[arg(
        short = 'l',
        long,
        value_enum,
        env = "WOL_RELAY_LOG_LEVEL",
        default_value_t = LogLevel::Info
    )]
    pub log_level: LogLevel,

    /// Log output format (auto: text on a terminal, JSON otherwise)
    #[arg(
        short = 'F',
        long,
        value_enum,
        env = "WOL_RELAY_LOG_FORMAT",
        default_value_t = LogFormat::Auto
    )]
    pub log_format: LogFormat,

    /// Networks or interfaces to relay between, by interface name or CIDR notation
    /// (comma-separated when set via env)
    #[arg(required = true, env = "WOL_RELAY_TARGETS", value_delimiter = ',')]
    pub targets: Vec<String>,
}

/// Log verbosity, selected via `-l`.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Log output format, selected via `-F`.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Auto,
    Json,
    Text,
}

/// Map the CLI log level onto the `log` crate's filter.
impl From<LogLevel> for log::LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Error => log::LevelFilter::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use clap::error::ErrorKind;

    /// Serializes tests: parsing reads the process-global environment, which the env
    /// tests mutate.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Removes relay environment variables so explicit CLI tests are not affected by
    /// the developer's shell (or by a previous test that panicked mid-cleanup).
    fn clear_relay_env() {
        // SAFETY: ENV_LOCK serializes every test that reads or writes the environment.
        unsafe {
            std::env::remove_var("WOL_RELAY_TARGETS");
            std::env::remove_var("WOL_RELAY_LOG_LEVEL");
            std::env::remove_var("WOL_RELAY_LOG_FORMAT");
        }
    }

    fn parse<const N: usize>(args: [&str; N]) -> Result<Cli, clap::Error> {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_relay_env();
        Cli::try_parse_from(args)
    }

    #[test]
    fn defaults_are_info_level_and_auto_format() {
        let cli = parse(["wol-relay", "eno1"]).unwrap();
        assert_eq!(cli.log_level, LogLevel::Info);
        assert_eq!(cli.log_format, LogFormat::Auto);
        assert_eq!(cli.targets, vec!["eno1"]);
    }

    #[test]
    fn multiple_targets_are_kept_in_order() {
        let cli = parse(["wol-relay", "eno1", "10.0.0.0/24", "eno2"]).unwrap();
        assert_eq!(cli.targets, vec!["eno1", "10.0.0.0/24", "eno2"]);
    }

    #[test]
    fn log_format_flag_selects_text() {
        let cli = parse(["wol-relay", "-F", "text", "eno1"]).unwrap();
        assert_eq!(cli.log_format, LogFormat::Text);
        let cli = parse(["wol-relay", "--log-format", "json", "eno1"]).unwrap();
        assert_eq!(cli.log_format, LogFormat::Json);
    }

    #[test]
    fn version_flag_requests_display_version() {
        let err = parse(["wol-relay", "--version"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    }

    #[test]
    fn no_targets_is_argument_error() {
        assert!(parse(["wol-relay"]).is_err());
    }

    #[test]
    fn env_vars_configure_targets_and_log_level() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: ENV_LOCK serializes every test that reads the environment.
        unsafe {
            std::env::set_var("WOL_RELAY_TARGETS", "eno1,10.0.0.0/24");
            std::env::set_var("WOL_RELAY_LOG_LEVEL", "debug");
        }

        // Env alone is enough.
        let cli = Cli::try_parse_from(["wol-relay"]).unwrap();
        assert_eq!(cli.targets, vec!["eno1", "10.0.0.0/24"]);
        assert_eq!(cli.log_level, LogLevel::Debug);

        // CLI takes precedence over env.
        let cli = Cli::try_parse_from(["wol-relay", "-l", "error", "eno2"]).unwrap();
        assert_eq!(cli.targets, vec!["eno2"]);
        assert_eq!(cli.log_level, LogLevel::Error);

        unsafe {
            std::env::remove_var("WOL_RELAY_TARGETS");
            std::env::remove_var("WOL_RELAY_LOG_LEVEL");
        }
    }

    #[test]
    fn env_var_configures_log_format() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: ENV_LOCK serializes every test that reads the environment.
        unsafe {
            std::env::set_var("WOL_RELAY_TARGETS", "eno1");
            std::env::set_var("WOL_RELAY_LOG_FORMAT", "json");
        }

        let cli = Cli::try_parse_from(["wol-relay"]).unwrap();
        assert_eq!(cli.log_format, LogFormat::Json);

        unsafe {
            std::env::remove_var("WOL_RELAY_TARGETS");
            std::env::remove_var("WOL_RELAY_LOG_FORMAT");
        }
    }
}
