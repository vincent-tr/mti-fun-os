// The init binary embeds some of the servers used for bootstrapping the system.

// https://docs.rs/include_bytes_aligned/latest/src/include_bytes_aligned/lib.rs.html#1-37
macro_rules! include_bytes_aligned {
    ($align_to:expr, $path:expr) => {{
        #[repr(C, align($align_to))]
        struct __Aligned<T: ?Sized>(T);

        static __DATA: &'static __Aligned<[u8]> = &__Aligned(*include_bytes!($path));

        &__DATA.0
    }};
}

// Make it 8 bytes aligned so that we can read headers properly
macro_rules! include_elf_bytes {
    ($path:expr) => {{
        include_bytes_aligned!(8, $path)
    }};
}

macro_rules! include_userland_binary {
    ($name:expr) => {{
        include_elf_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../target/",
            env!("MTI_FUN_OS_SERVERS_TARGET"),
            "/",
            env!("MTI_FUN_OS_SERVERS_PROFILE"),
            "/",
            $name
        ))
    }};
}

pub static PROCESS_SERVER: &[u8] = include_userland_binary!("process-server");
pub static VFS_SERVER: &[u8] = include_userland_binary!("vfs-server");
pub static MEMFS_SERVER: &[u8] = include_userland_binary!("memfs-server");
