use dobby_sys::ffi as dobby_ffi;
use std::{ffi::CString, ptr};

mod errors;
pub use errors::*;

/// Resolve the address of the specified symbol in the specified image.
/// Returns [`None`] if the symbol could not be found or if the image
/// has not been loaded yet
pub fn symbol_resolver<S>(image: S, symbol: S) -> Option<*mut ()>
where
    S: AsRef<str>,
{
    _symbol_resolver(image.as_ref(), symbol.as_ref())
}

fn _symbol_resolver(image: &str, symbol: &str) -> Option<*mut ()> {
    let image = CString::new(image).unwrap();
    let symbol = CString::new(symbol).unwrap();

    let symbol_address = unsafe { dobby_ffi::DobbySymbolResolver(image.as_ptr(), symbol.as_ptr()) };

    if symbol_address.is_null() {
        None
    } else {
        Some(symbol_address as *mut _)
    }
}

/// Apply inline hook to the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn hook(target: *mut (), replacement: *mut ()) -> Result<Option<*mut ()>, HookError> {
    let mut origin = ptr::null_mut();
    match dobby_ffi::DobbyHook(target as *mut _, replacement as *mut _, &mut origin) {
        0 => Ok(if origin.is_null() {
            None
        } else {
            Some(origin as *mut _)
        }),
        err => Err(HookError::Hook(err)),
    }
}

/// Remove any hook on the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn unhook(target: *mut ()) -> Result<(), HookError> {
    match dobby_ffi::DobbyDestroy(target as *mut _) {
        0 => Ok(()),
        err => Err(HookError::Hook(err)),
    }
}
