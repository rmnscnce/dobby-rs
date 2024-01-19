use std::ffi::c_int;

#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("Hook failure ({0})")]
    Hook(c_int),
}
