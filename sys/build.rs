use std::{env, path::PathBuf};

use cfg_if::cfg_if;
use cmake::Config;
use tap::Pipe;

fn cmake_config_setup(cfg: &mut Config) -> &mut Config {
    cfg.always_configure(true);

    // For Android
    if let Ok(var) = env::var("CMAKE_ANDROID_NDK") {
        cfg.define("CMAKE_ANDROID_NDK", var);
    }

    cfg.define("CMAKE_C_COMPILER", "clang");
    cfg.define("CMAKE_C_FLAGS", "-fuse-ld=lld");
    cfg.define("CMAKE_CXX_COMPILER", "clang++");
    cfg.define("CMAKE_CXX_FLAGS", "-fuse-ld=lld");
    cfg.define("CMAKE_ASM_COMPILER", "clang");
    cfg.build_target("dobby_static");
    cfg.define("DOBBY_GENERATE_SHARED", "OFF");
    cfg_if! {
        if #[cfg(feature = "debug")] {
            cfg.define("DOBBY_DEBUG", "ON");
            cfg.define("CMAKE_BUILD_TYPE", "Debug");
        } else {
            cfg.define("DOBBY_DEBUG", "OFF");
            cfg.define("CMAKE_BUILD_TYPE", "Release");
        }
    }

    cfg_if! {
        if #[cfg(feature = "near-branch")] {
            cfg.define("NearBranch", "ON");
        } else {
            cfg.define("NearBranch", "OFF");
        }
    }

    cfg_if! {
        if #[cfg(feature = "full-floating-point-register-pack")] {
            cfg.define("FullFloatingPointRegisterPack", "ON");
        } else {
            cfg.define("FullFloatingPointRegisterPack", "OFF");
        }
    }

    cfg_if! {
        if #[cfg(feature = "plugin-symbol-resolver")] {
            cfg.define("Plugin.SymbolResolver", "ON");
        } else {
            cfg.define("Plugin.SymbolResolver", "OFF");
        }
    }

    cfg_if! {
        if #[cfg(feature = "plugin-import-table-replace")] {
            cfg.define("Plugin.ImportTableReplace", "ON");
        } else {
            cfg.define("Plugin.ImportTableReplace", "OFF");
        }
    }

    cfg_if! {
        if #[cfg(feature = "plugin-android-bionic-linker-util")] {
            cfg.define("Plugin.Android.BionicLinkerUtil", "ON");
        } else {
            cfg.define("Plugin.Android.BionicLinkerUtil", "OFF");
        }
    }

    cfg_if! {
        if #[cfg(feature = "build-kernel-mode")] {
            cfg.define("DOBBY_BUILD_KERNEL_MODE", "ON");
        } else {
            cfg.define("DOBBY_BUILD_KERNEL_MODE", "OFF");
        }
    }

    cfg_if! {
        if #[cfg(feature = "private-obfuscation")] {
            cfg.define("Private.Obfuscation", "ON");
        } else {
            cfg.define("Private.Obfuscation", "OFF");
        }
    }

    cfg
}

pub fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let dest = Config::new("external/Dobby")
        .pipe_borrow_mut(cmake_config_setup)
        .build();
    println!("cargo:rustc-link-search=native={}/build", dest.display());
    println!("cargo:rustc-link-lib=static=dobby");
    #[cfg(not(target_os = "android"))]
    println!("cargo:rustc-link-lib=dylib=stdc++");

    let dest = bindgen::Builder::default()
        .header("external/Dobby/include/dobby.h")
        .generate()?;
    dest.write_to_file(PathBuf::from(env::var("OUT_DIR")?).join("dobby.h.rs"))?;

    Ok(())
}
