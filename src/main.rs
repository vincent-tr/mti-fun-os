fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    let bios_path = env!("BIOS_PATH");

    // choose whether to start the UEFI or BIOS image
    let uefi = true;

    let mut cmd = std::process::Command::new("qemu-system-x86_64");
    if uefi {
        cmd.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
        cmd.arg("-drive")
            .arg(format!("format=raw,file={uefi_path}"));
    } else {
        cmd.arg("-drive")
            .arg(format!("format=raw,file={bios_path}"));
    }
    cmd.args(&["-m", "512M"])
        .args(&["-display", "none"])
        .arg("-nographic")
        .args(&["-monitor", "stdio"])
        .args(&["-qmp", "unix:/tmp/qmp-socket,server,nowait"])
        .args(&["-serial", "file:serial.log"])
        .args(&["-device", "edu"])
        .args(&[
            "-nic",
            "tap,ifname=tap0,br=br0,model=e1000e,script=no,downscript=no",
        ])
        .arg("-s");

    if std::env::var("MTI_FUN_OS_DEBUG").is_ok() {
        // Do not start the CPU immediately, so that we have time to attach gdb
        cmd.arg("-S");
    }

    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}
