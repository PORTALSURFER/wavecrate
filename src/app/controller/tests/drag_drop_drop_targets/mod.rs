mod list;
mod transfer;

trait Must<T> {
    fn must(self) -> T;
}

impl<T, E: std::fmt::Display> Must<T> for Result<T, E> {
    fn must(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("unexpected error: {err}"),
        }
    }
}

impl<T> Must<T> for Option<T> {
    fn must(self) -> T {
        match self {
            Some(value) => value,
            None => panic!("expected value, found none"),
        }
    }
}
