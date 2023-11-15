use tracing::{debug, error, info, subscriber::set_global_default, trace, warn, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;

use super::config::{Config, Verbosity};

pub struct TracingLogger {
    _guard: WorkerGuard, // Keeps the background worker alive
}

impl TracingLogger {
    fn new(level: Level) -> Self {
        let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

        let subscriber = tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_max_level(level)
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        set_global_default(subscriber).expect("Failed to set global default logger");

        TracingLogger { _guard: guard }
    }

    pub fn from_config(config: &Config) -> Self {
        let level = match &config.logger.verbosity {
            Verbosity::Trace => Level::TRACE,
            Verbosity::Info => Level::INFO,
            Verbosity::Debug => Level::DEBUG,
            Verbosity::Warn => Level::WARN,
            Verbosity::Error => Level::ERROR,
        };
        match &config.logger.verbosity {
            Verbosity::Trace => trace!("Logger initialized with verbosity: Trace"),
            Verbosity::Info => info!("Logger initialized with verbosity: Info"),
            Verbosity::Debug => debug!("Logger initialized with verbosity: Debug"),
            Verbosity::Warn => warn!("Logger initialized with verbosity: Warn"),
            Verbosity::Error => error!("Logger initialized with verbosity: Error"),
        }
        TracingLogger::new(level)
    }
}
