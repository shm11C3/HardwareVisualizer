//! Structured logging macros that wrap `tracing`.
//!
//! These macros are exported via `#[macro_export]` so any crate that
//! depends on `hardviz-core` can `use hardviz_core::log_debug;` and emit log
//! records without taking a direct dependency on `tracing`.
//!
//! The level-named macros (`log_debug!`, `log_info!`, `log_warn!`,
//! `log_error!`) expand to `$crate::log_internal!(...)`, so callers
//! don't need a separate `log_internal` import. Only files that invoke
//! `log_internal!(...)` directly need to import it.

#[macro_export]
macro_rules! log_internal {
    ($level:ident, $action:expr, $function_name:expr, $custom_message:expr) => {
        match $custom_message {
            Some(msg) => {
                tracing::$level!(
                    message = %msg,
                    function = %$function_name,
                    action = %$action,
                    "{} - {}",
                    $action,
                    msg
                )
            }
            None => tracing::$level!(
                function = %$function_name,
                action = %$action,
                "{}",
                $action
            ),
        }
    };
}

#[macro_export]
macro_rules! log_debug {
  ($action:expr, $function_name:expr, $custom_message:expr) => {
    $crate::log_internal!(debug, $action, $function_name, $custom_message);
  };
}

#[macro_export]
macro_rules! log_info {
  ($action:expr, $function_name:expr, $custom_message:expr) => {
    $crate::log_internal!(info, $action, $function_name, $custom_message);
  };
}

#[macro_export]
macro_rules! log_warn {
  ($action:expr, $function_name:expr, $custom_message:expr) => {
    $crate::log_internal!(warn, $action, $function_name, $custom_message);
  };
}

#[macro_export]
macro_rules! log_error {
  ($action:expr, $function_name:expr, $custom_message:expr) => {
    $crate::log_internal!(error, $action, $function_name, $custom_message)
  };
}
