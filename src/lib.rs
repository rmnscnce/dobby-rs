use dobby_sys::ffi as dobby_ffi;
use std::{borrow::Cow, ffi::CString, ptr};

mod errors;
pub use errors::*;

/// Resolve the address of the specified symbol in the specified image.
/// Returns [`None`] if the symbol could not be found or if the image
/// has not been loaded yet
pub fn symbol_resolver<S>(image: Option<S>, symbol: S) -> Option<*mut ()>
where
    S: for<'a> Into<Cow<'a, str>>,
{
    _symbol_resolver(image.map(|s| s.into()).as_deref(), &symbol.into())
}

fn _symbol_resolver(image: Option<&str>, symbol: &str) -> Option<*mut ()> {
    let image = image.map(|image| CString::new(image).unwrap());
    let symbol = CString::new(symbol).unwrap();

    let symbol_address = unsafe {
        dobby_ffi::DobbySymbolResolver(
            match image {
                Some(image) => image.as_ptr(),
                None => ptr::null(),
            },
            symbol.as_ptr(),
        )
    };

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
