use std::{
    ffi::c_char,
    sync::atomic::{self, AtomicUsize},
};

use crate::Handle;

mod add_i32 {
    pub extern "C" fn original(a: i32, b: i32) -> i32 {
        a + b
    }

    pub extern "C" fn replacement(a: i32, b: i32) -> i32 {
        a + b + 2
    }
}

#[test]
pub fn local_replacement() {
    assert_eq!(add_i32::original(1, 2), 3);

    let original: Handle<extern "C" fn(i32, i32) -> i32> = unsafe {
        crate::hook(
            Handle::from_mut_unchecked(add_i32::original as *mut _),
            Handle::from_mut_unchecked(add_i32::replacement as *mut _),
        )
        .expect("Failed to hook add_i32")
        .expect("msg: Hooked function should return original function")
    };

    assert_eq!(add_i32::original(1, 2), 5);
    assert_eq!(unsafe { original.dispatch() }(1, 2), 3);

    unsafe {
        crate::unhook::<extern "C" fn(i32, i32) -> i32>(Handle::from_mut_unchecked(
            add_i32::original as *mut _,
        ))
    }
    .unwrap();
    assert_eq!(add_i32::original(1, 2), 3);
}

#[test]
pub fn local_trampoline() {
    static mut ORIGINAL_ADD_I32_SYM: AtomicUsize = const { AtomicUsize::new(0) };

    extern "C" fn replacement_trampolined(a: i32, b: i32) -> i32 {
        let trampoline = unsafe {
            Handle::<extern "C" fn(i32, i32) -> i32>::from_mut_unchecked(
                (&raw const ORIGINAL_ADD_I32_SYM)
                    .as_ref()
                    .unwrap()
                    .load(atomic::Ordering::Relaxed) as *mut _,
            )
            .dispatch()
        };

        trampoline(a, b) + trampoline(a, b)
    }

    let original: Handle<extern "C" fn(i32, i32) -> i32> = unsafe {
        crate::hook(
            Handle::from_mut_unchecked(add_i32::original as *mut _),
            Handle::from_mut_unchecked(replacement_trampolined as *mut _),
        )
        .expect("Failed to hook add_i32")
        .expect("msg: Hooked function should return original function")
    };

    unsafe { (&raw const ORIGINAL_ADD_I32_SYM).as_ref() }
        .unwrap()
        .store(original.into_raw() as _, atomic::Ordering::Relaxed);

    assert_eq!(add_i32::original(1, 2), 6);
    assert_eq!(
        unsafe {
            Handle::<extern "C" fn(i32, i32) -> i32>::from_mut_unchecked(
                (&raw const ORIGINAL_ADD_I32_SYM)
                    .as_ref()
                    .unwrap()
                    .load(atomic::Ordering::Relaxed) as *mut _,
            )
            .dispatch()(1, 2)
        },
        3
    );

    unsafe {
        crate::unhook::<extern "C" fn(i32, i32) -> i32>(Handle::from_mut_unchecked(
            add_i32::original as *mut _,
        ))
    }
    .unwrap();
}

#[test]
pub fn libc_replacement() {
    // find strcmp
    let strcmp: Handle<unsafe extern "C" fn(*const c_char, *const c_char) -> i32> =
        crate::symbol_resolver(Some("libc.so"), "strcmp").expect("Failed to resolve symbol");

    // test strcmp
    assert_eq!(
        unsafe {
            strcmp.dispatch()(
                c"lorem".as_ptr() as *const c_char,
                c"lorem".as_ptr() as *const c_char,
            )
        },
        0
    );

    unsafe extern "C" fn jailbroken_strcmp(_: *const c_char, _: *const c_char) -> i32 {
        // return 0 to indicate that the strings are equal
        0
    }

    // hook strcmp
    _ = unsafe {
        crate::hook::<unsafe extern "C" fn(*const c_char, *const c_char) -> i32>(
            strcmp,
            Handle::from_mut_unchecked(jailbroken_strcmp as *mut _),
        )
    }
    .expect("Failed to hook strcmp")
    .expect("msg: Hooked function should return original function");

    // test hooked strcmp
    assert_eq!(
        unsafe {
            strcmp.dispatch()(
                c"ipsum".as_ptr() as *const c_char,
                c"lorem".as_ptr() as *const c_char,
            )
        },
        0
    );

    // unhook strcmp
    unsafe { crate::unhook::<unsafe extern "C" fn(*const c_char, *const c_char) -> i32>(strcmp) }
        .expect("Failed to unhook strcmp");

    // test unhooked strcmp
    assert_ne!(
        unsafe {
            strcmp.dispatch()(
                c"ipsum".as_ptr() as *const c_char,
                c"lorem".as_ptr() as *const c_char,
            )
        },
        0
    );
}
