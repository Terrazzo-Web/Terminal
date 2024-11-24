pub fn do_or_log<T, R: Default, E: std::fmt::Display>(
    f: impl Fn(T) -> Result<R, LogMessage<E>>,
) -> impl Fn(T) -> R {
    move |t| match f(t) {
        Ok(r) => r,
        Err(e) => {
            match e.level {
                Level::Warn => tracing::warn!("{}", e.message),
                Level::Error => tracing::error!("{}", e.message),
            }
            R::default()
        }
    }
}

#[must_use]
pub trait ToLogMessage {
    fn warn(self) -> LogMessage<Self>;
    fn error(self) -> LogMessage<Self>;
}

pub struct LogMessage<T: ?Sized> {
    level: Level,
    message: T,
}

enum Level {
    Warn,
    Error,
}

impl<T: std::fmt::Display> ToLogMessage for T {
    fn warn(self) -> LogMessage<Self> {
        LogMessage {
            level: Level::Warn,
            message: self,
        }
    }

    fn error(self) -> LogMessage<Self> {
        LogMessage {
            level: Level::Error,
            message: self,
        }
    }
}
