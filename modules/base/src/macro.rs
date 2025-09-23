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

#[macro_export]
macro_rules! ensure_err {
    ($condition:expr, $callback:expr) => {{
        if !($condition) {
            let e = ($callback)();
            return Err(e);
        }
    }};
}
