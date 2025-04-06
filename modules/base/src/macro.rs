#[macro_export]
macro_rules! hook_err {
    ($expr:expr, $callback:expr) => {{
        match $expr {
            Ok(val) => Ok(val),
            Err(e) => {
                ($callback)(&e);
                Err(e)
            }
        }
    }};
}
