use std::{env, error, path::PathBuf};

use cfg_if::cfg_if;
use tap::Pipe as _;

fn main() -> Result<(), Box<dyn error::Error>> {
    cfg_if! {
        if #[cfg(feature = "static")] {
            println!("cargo:rustc-link-lib=static=dobby");
        } else {
            println!("cargo:rustc-link-lib=dylib=dobby");
        }
    }

    let output = bindgen::Builder::default()
        .header("Dobby/include/dobby.h")
        .pipe(|builder| {
            if let Ok(ndk_sysroot_path) = env::var("NDK_SYSROOT_PATH") {
                builder.clang_arg(format!("--sysroot={}", ndk_sysroot_path))
            } else if let Ok(sysroot_path) = env::var("SYSROOT") {
                builder.clang_arg(format!("--sysroot={}", sysroot_path))
            } else {
                builder
            }
        })
        .generate()?;
    let output_path = PathBuf::from(env::var("OUT_DIR")?);

    Ok(output.write_to_file(output_path.join("dobby.h.rs"))?)
}
