/// Allows const expansion until https://github.com/rust-lang/rust/issues/67441
/// Cannot be a function with generic type because of https://github.com/rust-lang/rust/issues/73255
macro_rules! const_unwrap_or {
    ($op:path, $def:expr) => {{
        // not yet stable clippy lint
        #[allow(clippy::manual_unwrap_or)]
        match $op {
            Some(value) => value,
            None => $def,
        }
    }};
}

/// Allows const expansion until https://github.com/rust-lang/rust/issues/67441
/// Cannot be a function with generic type because of https://github.com/rust-lang/rust/issues/73255
macro_rules! const_is_none {
    ($op:path) => {
        match &$op {
            Some(_) => false,
            None => true,
        }
    };
}

/// Allows const expansion until https://github.com/rust-lang/rust/issues/67441
/// Cannot be a function with generic type because of https://github.com/rust-lang/rust/issues/73255
macro_rules! const_is_some {
    ($op:path) => {
        match &$op {
            Some(_) => true,
            None => false,
        }
    };
}

/// Allows const expansion until https://github.com/rust-lang/rust/issues/67441
/// Cannot be a function with generic type because of https://github.com/rust-lang/rust/issues/73255
macro_rules! const_map_or {
    ($op:expr, $fn:expr, $def:expr) => {
        match &$op {
            Some(v) => $fn(v),
            None => $def,
        }
    };
}
