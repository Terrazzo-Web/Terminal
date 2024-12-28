use tracing::error;

pub trait OrElseLog<T, E>: Sized {
    #[track_caller]
    fn or_throw(self, log: impl std::fmt::Display) -> T {
        self.or_else_throw(move |_| log)
    }

    #[track_caller]
    fn or_else_throw<M: std::fmt::Display>(self, log: impl FnOnce(E) -> M) -> T;
}

impl<T> OrElseLog<T, ()> for Option<T> {
    #[track_caller]
    fn or_else_throw<M: std::fmt::Display>(self, log: impl FnOnce(()) -> M) -> T {
        if let Some(r) = self {
            return r;
        }
        let log = log(());
        error!("{log}");
        panic!("{log}")
    }
}

impl<T, E> OrElseLog<T, E> for Result<T, E> {
    #[track_caller]
    fn or_else_throw<M: std::fmt::Display>(self, log: impl FnOnce(E) -> M) -> T {
        match self {
            Ok(ok) => return ok,
            Err(err) => {
                let log = log(err);
                error!("{log}");
                panic!("{log}")
            }
        }
    }
}
