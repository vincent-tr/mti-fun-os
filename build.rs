use std::path::PathBuf;

fn main() {
    // Skip build script when rust-analyzer runs it (not the primary package being built)
    if std::env::var("RUSTC_WRAPPER")
        .unwrap_or_default()
        .contains("rust-analyzer")
    {
        return;
    }

    // set by cargo, build scripts should use this directory for output files
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));

    // set by cargo's artifact dependency feature, see
    // https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies
    let kernel = PathBuf::from(
        std::env::var("CARGO_BIN_FILE_KERNEL_kernel")
            .expect("CARGO_BIN_FILE_KERNEL_kernel not set"),
    );

    let base_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let profile =
        std::env::var("MTI_FUN_OS_INIT_PROFILE").expect("MTI_FUN_OS_INIT_PROFILE not set");
    let target = std::env::var("MTI_FUN_OS_INIT_TARGET").expect("MTI_FUN_OS_INIT_TARGET not set");

    let init = PathBuf::from(format!("{base_dir}/target/{target}/{profile}/init"));

    println!("cargo:rerun-if-changed={}", init.display());

    // create an UEFI disk image (optional)
    let uefi_path = out_dir.join("uefi.img");
    bootloader::UefiBoot::new(&kernel)
        .set_ramdisk(&init)
        .create_disk_image(&uefi_path)
        .unwrap();

    // create a BIOS disk image
    let bios_path = out_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel)
        .set_ramdisk(&init)
        .create_disk_image(&bios_path)
        .unwrap();

    // pass the disk image paths as env variables to the `main.rs`
    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}
