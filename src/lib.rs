pub use dobbyhook_sys as ffi;
use std::{
    ffi::CString,
    marker::PhantomData,
    mem,
    ptr::{self, NonNull},
};

mod errors;
pub use errors::*;

#[cfg(test)]
mod test;

struct ShapeAssertion<T, U>(T, U);
impl<T, U> ShapeAssertion<T, U> {
    const ASSERT: () = {
        assert!(mem::size_of::<T>() == mem::size_of::<U>());
        assert!(mem::align_of::<T>() % mem::align_of::<U>() == 0)
    };
}

const unsafe fn transmute<T, U>(value: T) -> U {
    #[allow(clippy::let_unit_value)]
    let _ = ShapeAssertion::<T, U>::ASSERT;

    #[repr(C)]
    union Transmute<T, U> {
        src: mem::ManuallyDrop<T>,
        dst: mem::ManuallyDrop<U>,
    }

    mem::ManuallyDrop::into_inner(unsafe {
        Transmute::<T, U> {
            src: mem::ManuallyDrop::new(value),
        }
        .dst
    })
}

/// Wrapper for a function pointer handle, which can be hooked into or dispatched as a function call
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Handle<'a, FnPtr>
where
    FnPtr: Copy + 'a,
{
    ptr: NonNull<FnPtr>,
    _marker: PhantomData<&'a FnPtr>,
}

impl<'a, FnPtr> Handle<'a, FnPtr>
where
    FnPtr: Copy,
{
    /// Create a new handle from a non-null pointer to a function pointer
    ///
    /// ### Usage example
    /// ```rust
    /// use dobbyhook::Handle;
    /// use std::ptr::NonNull;
    ///
    /// fn binary_add(a: f32, b: f32) -> f32 {
    ///     a + b
    /// }
    ///
    /// let handle = Handle::new(NonNull::new(binary_add as *mut fn(f32, f32) -> f32).unwrap());
    /// assert_eq!(unsafe { handle.dispatch() }(1.0, 2.0), binary_add(1.0, 2.0));
    /// ```
    pub const fn new(ptr: NonNull<FnPtr>) -> Self {
        #[allow(clippy::let_unit_value)]
        let _ = ShapeAssertion::<FnPtr, *mut ()>::ASSERT;

        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Create a new handle from a raw pointer to a function pointer. Returns [`Option::None`] if the pointer is null.
    ///
    /// ### Usage example
    /// ```rust
    /// use dobbyhook::Handle;
    /// use std::ptr::NonNull;
    ///
    /// fn binary_add(a: f32, b: f32) -> f32 {
    ///     a + b
    /// }
    ///
    /// let raw = binary_add as *mut fn(f32, f32) -> f32;
    /// let handle = Handle::from_mut(raw);
    /// assert_eq!(unsafe { handle.unwrap().dispatch() }(1.0, 2.0), binary_add(1.0, 2.0));
    /// ```
    pub const fn from_mut(raw: *mut FnPtr) -> Option<Self> {
        #[allow(clippy::let_unit_value)]
        let _ = ShapeAssertion::<FnPtr, *mut ()>::ASSERT;

        match raw {
            p if p.is_null() => None,
            // Safety: `raw` is guaranteed to be non-null here as per the match arm
            p => Some(unsafe { Self::from_mut_unchecked(p) }),
        }
    }

    /// Create a new handle from a raw pointer to a function pointer without checking for null
    ///
    /// # Safety
    /// `raw` must be non-null and point to a valid function pointer of type `FnPtr`.
    /// Using this function with a null pointer or an invalid pointer may lead to undefined behavior.
    ///
    /// ### Usage example
    /// ```rust
    /// use dobbyhook::Handle;
    /// use std::ptr::NonNull;
    ///
    /// fn binary_add(a: f32, b: f32) -> f32 {
    ///    a + b
    /// }
    ///
    /// let raw = binary_add as *mut fn(f32, f32) -> f32;
    /// let handle = unsafe { Handle::from_mut_unchecked(raw) };
    /// assert_eq!(unsafe { handle.dispatch() }(1.0, 2.0), binary_add(1.0, 2.0));
    /// ```
    pub const unsafe fn from_mut_unchecked(raw: *mut FnPtr) -> Self {
        #[allow(clippy::let_unit_value)]
        let _ = ShapeAssertion::<FnPtr, *mut ()>::ASSERT;

        Self {
            ptr: unsafe { NonNull::new_unchecked(raw) },
            _marker: PhantomData,
        }
    }

    /// Consume the handle and return the non-null pointer to the function pointer.
    /// This is useful for functions that require a pointer to the function.
    pub const fn into_ptr(self) -> NonNull<FnPtr> {
        self.ptr
    }

    /// Consume the handle and return the raw pointer to the function pointer.
    /// This is useful for functions that require a raw pointer.
    pub const fn into_raw(self) -> *mut FnPtr {
        self.ptr.as_ptr()
    }

    /// Dispatch the function pointer for calling.
    /// This is useful for calling "trampolines" of hooked functions so that the original function can still be called.
    ///
    /// # Safety
    /// This function is inherently unsafe because it assumes that the function pointer is valid and callable.
    /// Calling an invalid function pointer may lead to undefined behavior
    pub const unsafe fn dispatch(&self) -> FnPtr {
        unsafe { transmute(self.ptr.as_ptr()) }
    }
}

/// Resolve the address of the specified symbol in the specified image.
/// Returns [`None`] if the symbol could not be found or if the image
/// has not been loaded yet
pub fn symbol_resolver<S, F>(image: Option<S>, symbol: S) -> Option<Handle<'static, F>>
where
    S: AsRef<str>,
    F: Copy,
{
    _symbol_resolver(image.as_ref().map(AsRef::as_ref), symbol.as_ref())
}

fn _symbol_resolver<F>(image: Option<&str>, symbol: &str) -> Option<Handle<'static, F>>
where
    F: Copy + 'static,
{
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
        Some(unsafe { Handle::from_mut_unchecked(symbol_address.cast()) })
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
    unsafe { _patch_code(address, buffer.as_ref()) }
}

unsafe fn _patch_code(address: NonNull<()>, buffer: &[u8]) -> Result<(), HookError> {
    let buffer_size = buffer.len();

    match unsafe {
        ffi::DobbyCodePatch(
            address.as_ptr().cast(),
            buffer.as_ptr().cast_mut(),
            buffer_size.try_into().unwrap(),
        )
    } {
        0 => Ok(()),
        1 => Err(HookError::MemoryOperationError),
        2 => Err(HookError::NotSupportAllocateExecutableMemory),
        3 => Err(HookError::MemoryOperationErrorNotEnough),
        4 => Err(HookError::MemoryOperationErrorNone),
        e => Err(HookError::Unknown(e)),
    }
}

/// Apply inline hook to the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn hook<'a, F>(
    target: Handle<'a, F>,
    replacement: Handle<'a, F>,
) -> Result<Option<Handle<'a, F>>, HookError>
where
    F: Copy + 'a,
{
    let mut origin = ptr::null_mut();
    match unsafe {
        ffi::DobbyHook(
            target.into_raw().cast(),
            replacement.into_raw().cast(),
            &mut origin,
        )
    } {
        -1 => Err(HookError::FailedToHook),
        _ => Ok(match origin {
            p if p.is_null() => None,
            p => Some(unsafe { Handle::from_mut_unchecked(p.cast()) }),
        }),
    }
}

/// Remove any hook on the specified target address
///
/// # Safety
/// This function is inherently unsafe due to its nature, and may unexpectedly
/// crash the process if used incorrectly
pub unsafe fn unhook<F>(target: Handle<'_, F>) -> Result<(), HookError>
where
    for<'a> F: Copy + 'a,
{
    match unsafe { ffi::DobbyDestroy(target.into_raw().cast()) } {
        -1 => Err(HookError::FailedToUndoHook),
        _ => Ok(()),
    }
}
