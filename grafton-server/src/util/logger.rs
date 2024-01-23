use {
    tracing::{debug, error, info, subscriber::set_global_default, trace, warn, Level},
    tracing_appender::non_blocking::WorkerGuard,
    tracing_subscriber::fmt::format::FmtSpan,
};

use crate::util::config::{Config, Verbosity};

pub struct Logger {
    _guard: WorkerGuard, // Keeps the background worker alive
}

impl Logger {
    fn new(level: Level) -> Self {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

        let subscriber = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_max_level(level)
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        set_global_default(subscriber).expect("Failed to set global default logger");

        Self { _guard: guard }
    }

    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let level = get_log_level_from_verbosity(&config.logger.verbosity);
        log_initialization_message(&config.logger.verbosity);
        Self::new(level)
    }
}

#[allow(clippy::missing_const_for_fn)]
fn get_log_level_from_verbosity(verbosity: &Verbosity) -> Level {
    match verbosity {
        Verbosity::Trace => Level::TRACE,
        Verbosity::Info => Level::INFO,
        Verbosity::Debug => Level::DEBUG,
        Verbosity::Warn => Level::WARN,
        Verbosity::Error => Level::ERROR,
    }
}

#[allow(clippy::cognitive_complexity)]
fn log_initialization_message(verbosity: &Verbosity) {
    match verbosity {
        Verbosity::Trace => trace!("Logger initialized with verbosity: Trace"),
        Verbosity::Info => info!("Logger initialized with verbosity: Info"),
        Verbosity::Debug => debug!("Logger initialized with verbosity: Debug"),
        Verbosity::Warn => warn!("Logger initialized with verbosity: Warn"),
        Verbosity::Error => error!("Logger initialized with verbosity: Error"),
    }
}
