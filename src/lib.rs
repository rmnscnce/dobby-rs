pub use dobbyhook_sys as ffi;
use std::{
    ffi::CString,
    ptr::{self, NonNull},
};

mod errors;
pub use errors::*;

/// Resolve the address of the specified symbol in the specified image.
/// Returns [`None`] if the symbol could not be found or if the image
/// has not been loaded yet
pub fn symbol_resolver<S>(image: Option<S>, symbol: S) -> Option<*mut ()>
where
    S: AsRef<str>,
{
    _symbol_resolver(image.as_ref().map(AsRef::as_ref), symbol.as_ref())
}

fn _symbol_resolver(image: Option<&str>, symbol: &str) -> Option<*mut ()> {
    let image = image.map(|image| CString::new(image).unwrap());
    let symbol = CString::new(symbol).unwrap();

    let symbol_address = unsafe {
        ffi::DobbySymbolResolver(
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

/// Patch the code at the specified address with the provided buffer
/// Returns an error if the operation failed
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn patch_code<B>(address: NonNull<()>, buffer: B) -> Result<(), HookError>
where
    B: AsRef<[u8]>,
{
    _patch_code(address, buffer.as_ref())
}

unsafe fn _patch_code(address: NonNull<()>, buffer: &[u8]) -> Result<(), HookError> {
    let buffer_size = buffer.len();

    match ffi::DobbyCodePatch(
        address.as_ptr().cast(),
        buffer.as_ptr().cast_mut(),
        buffer_size.try_into().unwrap(),
    ) {
        0 => Ok(()),
        1 => Err(HookError::MemoryOperationError),
        2 => Err(HookError::NotSupportAllocateExecutableMemory),
        3 => Err(HookError::MemoryOperationErrorNotEnough),
        4 => Err(HookError::MemoryOperationErrorNone),
        _ => unreachable!(),
    }
}

/// Apply inline hook to the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn hook<T>(
    target: NonNull<T>,
    replacement: NonNull<T>,
) -> Result<Option<NonNull<T>>, HookError> {
    let mut origin = ptr::null_mut();
    match ffi::DobbyHook(
        target.as_ptr().cast(),
        replacement.as_ptr().cast(),
        &mut origin,
    ) {
        -1 => Err(HookError::FailedToHook),
        _ => Ok(if origin.is_null() {
            None
        } else {
            Some(NonNull::new_unchecked(origin.cast()))
        }),
    }
}

/// Remove any hook on the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn unhook<T>(target: NonNull<T>) -> Result<(), HookError> {
    match ffi::DobbyDestroy(target.as_ptr().cast()) {
        -1 => Err(HookError::FailedToUndoHook),
        _ => Ok(()),
    }
}
