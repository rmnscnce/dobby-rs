pub use dobbyhook_sys as ffi;
use std::{
    ffi::CString,
    marker::PhantomData,
    mem,
    os::raw::c_void,
    ptr::{self, NonNull},
};

mod errors;
pub use errors::*;

struct AssertSize<F>(PhantomData<F>);
impl<F> AssertSize<F> {
    const ASSERT: () = if mem::size_of::<F>() != mem::size_of::<*mut c_void>() {
        panic!("Size mismatch, must not be a valid function pointer");
    };
}

/// Resolve the address of the specified symbol in the specified image.
/// Returns [`None`] if the symbol could not be found or if the image
/// has not been loaded yet
pub fn symbol_resolver<S, F>(image: Option<S>, symbol: S) -> Option<NonNull<F>>
where
    S: AsRef<str>,
    F: Sized,
{
    _symbol_resolver(image.as_ref().map(AsRef::as_ref), symbol.as_ref())
}

fn _symbol_resolver<F>(image: Option<&str>, symbol: &str) -> Option<F>
where
    F: Sized,
{
    #[allow(clippy::let_unit_value)]
    let _ = AssertSize::<F>::ASSERT;

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

    if symbol_address.is_null() || symbol_address.align_offset(mem::align_of::<F>()) != 0 {
        None
    } else {
        Some(unsafe { mem::transmute_copy::<_, F>(&symbol_address) })
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
pub unsafe fn hook<F>(target: F, replacement: F) -> Result<Option<F>, HookError>
where
    F: Sized,
{
    #[allow(clippy::let_unit_value)]
    let _ = AssertSize::<F>::ASSERT;

    let mut origin = ptr::null_mut();
    match ffi::DobbyHook(
        mem::transmute_copy::<_, *mut c_void>(&target),
        mem::transmute_copy::<_, *mut c_void>(&replacement),
        &mut origin,
    ) {
        -1 => Err(HookError::FailedToHook),
        _ => Ok(
            if origin.is_null() || origin.align_offset(mem::align_of::<F>()) != 0 {
                None
            } else {
                Some(mem::transmute_copy::<_, F>(&origin))
            },
        ),
    }
}

/// Remove any hook on the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn unhook<F>(target: F) -> Result<(), HookError>
where
    F: Sized,
{
    #[allow(clippy::let_unit_value)]
    let _ = AssertSize::<F>::ASSERT;

    match ffi::DobbyDestroy(mem::transmute_copy::<_, *mut c_void>(&target)) {
        -1 => Err(HookError::FailedToUndoHook),
        _ => Ok(()),
    }
}
