#[macro_export]
macro_rules! ensure_err {
    ($condition:expr, $callback:expr) => {{
        if !($condition) {
            let e = ($callback)();
            return Err(e);
        }
    }};
}
