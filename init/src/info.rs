use core::mem;

use libruntime::kobject;

/// Memory-managed implementation of the syscalls InitInfo
#[derive(Debug)]
pub struct InitInfo {
    archive_mapping: kobject::Mapping<'static>,
    framebuffer: syscalls::init::Framebuffer,
}

impl InitInfo {
    /// Create a new InitInfo from the raw init info, mapping the archive and such.
    ///
    /// # Safety
    /// - This should only be called once, and it takes ownership of the raw init info.
    pub unsafe fn from_raw(init_info_ptr: *const syscalls::init::InitInfo) -> Self {
        let process = kobject::Process::current();
        let info = unsafe { &*(init_info_ptr) };

        assert!(
            info.info_mapping.size > 0,
            "Info mapping size must be greater than 0"
        );
        assert!(
            info.archive_mapping.size > 0,
            "Archive mapping size must be greater than 0"
        );
        assert!(
            info.info_mapping.address == info as *const _ as usize,
            "Info mapping address must be the same as the info pointer"
        );

        let info_mapping = unsafe {
            kobject::Mapping::unleak(
                process,
                info.info_mapping.address..info.info_mapping.address + info.info_mapping.size,
                kobject::Permissions::READ,
            )
        };

        let archive_mapping = unsafe {
            kobject::Mapping::unleak(
                process,
                info.archive_mapping.address
                    ..info.archive_mapping.address + info.archive_mapping.size,
                kobject::Permissions::READ,
            )
        };

        // We cannot manage the init mapping, as we need it to keep it alive as long as init lives.

        let info = Self {
            archive_mapping,
            framebuffer: info.framebuffer.clone(),
        };

        // We can drop init info mapping now (we copied the data we need from it)
        // Let's be explicit
        mem::drop(info_mapping);

        info
    }

    /// Get the archive buffer, which contains the initial ramdisk contents.
    pub fn archive_buffer(&self) -> &[u8] {
        unsafe {
            self.archive_mapping
                .as_buffer()
                .expect("Could not get archive buffer")
        }
    }

    /// Get the framebuffer info, which contains the information about the framebuffer that we can use to setup the display server.
    pub fn framebuffer(&self) -> &syscalls::init::Framebuffer {
        &self.framebuffer
    }
}
