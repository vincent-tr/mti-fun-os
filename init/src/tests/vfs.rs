use libruntime::{
    timer,
    vfs::{self, VfsObject},
};
use log::info;

/// Tests for the virtual file system (VFS).
#[allow(dead_code)]
pub fn test_vfs() {
    info!("Starting VFS tests...");

    test_directories();
    test_files();
    test_metadata();
    test_symlinks();
    test_mounts();

    info!("All VFS tests completed successfully!");
}

/// Test directory operations
fn test_directories() {
    info!("Testing directory operations...");

    // Test 1: Create directories
    info!("Test 1: Create directories");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;

        vfs::Directory::create("/test1", perms).expect("Failed to create /test1");
        vfs::Directory::create("/test1/subdir", perms).expect("Failed to create /test1/subdir");
        vfs::Directory::create("/test1/subdir/nested", perms)
            .expect("Failed to create /test1/subdir/nested");

        info!("Test 1: PASSED");
    }

    // Test 2: List directory contents
    info!("Test 2: List directory contents");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;

        let dir = vfs::Directory::create("/test2", perms).expect("Failed to create /test2");
        vfs::Directory::create("/test2/dir1", perms).expect("Failed to create /test2/dir1");
        vfs::Directory::create("/test2/dir2", perms).expect("Failed to create /test2/dir2");
        vfs::File::create(
            "/test2/file1",
            vfs::Permissions::READ | vfs::Permissions::WRITE,
        )
        .expect("Failed to create /test2/file1");

        let entries = dir.list().expect("Failed to list directory");

        assert_eq!(entries.len(), 3, "Directory should have 3 entries");

        let names: alloc::vec::Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"dir1"), "Should contain dir1");
        assert!(names.contains(&"dir2"), "Should contain dir2");
        assert!(names.contains(&"file1"), "Should contain file1");

        // Verify types
        for entry in &entries {
            match entry.name.as_str() {
                "dir1" | "dir2" => {
                    assert_eq!(
                        entry.r#type,
                        vfs::NodeType::Directory,
                        "dir should be Directory type"
                    );
                }
                "file1" => {
                    assert_eq!(
                        entry.r#type,
                        vfs::NodeType::File,
                        "file1 should be File type"
                    );
                }
                _ => panic!("Unexpected entry: {}", entry.name),
            }
        }

        info!("Test 2: PASSED");
    }

    // Test 3: Remove directories
    info!("Test 3: Remove directories");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;

        let dir = vfs::Directory::create("/test3", perms).expect("Failed to create /test3");
        vfs::Directory::create("/test3/toremove", perms).expect("Failed to create /test3/toremove");

        // Verify it exists
        let entries = dir.list().expect("Failed to list directory");
        assert_eq!(entries.len(), 1, "Directory should have 1 entry");

        // Remove it
        vfs::remove("/test3/toremove").expect("Failed to remove directory");

        // Verify it's gone
        let entries = dir.list().expect("Failed to list directory");
        assert_eq!(entries.len(), 0, "Directory should be empty after removal");

        info!("Test 3: PASSED");
    }

    // Test 4: Open existing directory
    info!("Test 4: Open existing directory");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;

        vfs::Directory::create("/test4", perms).expect("Failed to create /test4");

        // Open it again (should succeed)
        let _dir = vfs::Directory::open("/test4").expect("Failed to open existing directory");

        info!("Test 4: PASSED");
    }

    // Test 5: Cannot remove non-empty directory
    info!("Test 5: Cannot remove non-empty directory");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;

        vfs::Directory::create("/test5", perms).expect("Failed to create /test5");

        // Add a file to the directory
        let file_perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        vfs::File::create("/test5/somefile", file_perms)
            .expect("Failed to create file in directory");

        // Try to remove the non-empty directory (should fail)
        let result = vfs::remove("/test5");
        assert!(
            result.is_err(),
            "Should not be able to remove non-empty directory"
        );

        // Also test with a subdirectory
        vfs::Directory::create("/test5b", perms).expect("Failed to create /test5b");
        vfs::Directory::create("/test5b/subdir", perms).expect("Failed to create subdirectory");

        let result = vfs::remove("/test5b");
        assert!(
            result.is_err(),
            "Should not be able to remove directory with subdirectory"
        );

        info!("Test 5: PASSED");
    }

    info!("Directory operations tests completed");
}

/// Test file operations
fn test_files() {
    info!("Testing file operations...");

    // Test 1: Create and write to file
    info!("Test 1: Create and write to file");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/testfile1", perms).expect("Failed to create file");

        let data = b"Hello, VFS!";
        // Resize file to allocate space before writing
        file.resisze(data.len()).expect("Failed to resize file");
        let written = file.write(0, data).expect("Failed to write to file");
        assert_eq!(written, data.len(), "Should write all bytes");

        info!("Test 1: PASSED");
    }

    // Test 2: Read from file
    info!("Test 2: Read from file");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/testfile2", perms).expect("Failed to create file");

        let data = b"Test data for reading";
        // Resize file to allocate space before writing
        file.resisze(data.len()).expect("Failed to resize file");
        file.write(0, data).expect("Failed to write to file");

        // Read it back
        let mut buffer = [0u8; 100];
        let read = file.read(0, &mut buffer).expect("Failed to read from file");
        assert_eq!(read, data.len(), "Should read correct number of bytes");
        assert_eq!(&buffer[..read], data, "Read data should match written data");

        info!("Test 2: PASSED");
    }

    // Test 3: Write at offset
    info!("Test 3: Write at offset");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/testfile3", perms).expect("Failed to create file");

        // Resize file to allocate space before writing
        file.resisze(4).expect("Failed to resize file");
        file.write(0, b"AAAA").expect("Failed to write");
        file.write(2, b"BB").expect("Failed to write at offset");

        let mut buffer = [0u8; 4];
        file.read(0, &mut buffer).expect("Failed to read");
        assert_eq!(&buffer, b"AABB", "Offset write should work correctly");

        info!("Test 3: PASSED");
    }

    // Test 4: Resize file
    info!("Test 4: Resize file");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/testfile4", perms).expect("Failed to create file");

        // Resize file to allocate space before writing
        file.resisze(5).expect("Failed to resize file");
        file.write(0, b"Hello").expect("Failed to write");

        // Get size
        let metadata = file.stat().expect("Failed to stat file");
        assert_eq!(metadata.size, 5, "File size should be 5");

        // Resize to larger
        file.resisze(10).expect("Failed to resize file");
        let metadata = file.stat().expect("Failed to stat file");
        assert_eq!(metadata.size, 10, "File size should be 10 after resize");

        // Verify the first 5 bytes are still "Hello" and the rest are zeros
        let mut buffer = [0xFFu8; 10];
        file.read(0, &mut buffer).expect("Failed to read");
        assert_eq!(&buffer[..5], b"Hello", "Original data should be preserved");
        assert_eq!(&buffer[5..], &[0u8; 5], "Extended bytes should be zeros");

        // Resize to smaller (truncate)
        file.resisze(3).expect("Failed to resize file");
        let metadata = file.stat().expect("Failed to stat file");
        assert_eq!(metadata.size, 3, "File size should be 3 after truncate");

        info!("Test 4: PASSED");
    }

    // Test 5: Open existing file
    info!("Test 5: Open existing file");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/testfile5", perms).expect("Failed to create file");
        // Resize file to allocate space before writing
        file.resisze(8).expect("Failed to resize file");
        file.write(0, b"existing").expect("Failed to write");
        drop(file);

        // Open existing file
        let file = vfs::File::open("/testfile5", vfs::HandlePermissions::READ)
            .expect("Failed to open existing file");

        let mut buffer = [0u8; 8];
        let read = file.read(0, &mut buffer).expect("Failed to read");
        assert_eq!(&buffer[..read], b"existing", "Should read existing data");

        info!("Test 5: PASSED");
    }

    // Test 6: Remove file
    info!("Test 6: Remove file");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        vfs::File::create("/testfile6", perms).expect("Failed to create file");

        vfs::remove("/testfile6").expect("Failed to remove file");

        // Try to open removed file (should fail)
        let result = vfs::File::open("/testfile6", vfs::HandlePermissions::READ);
        assert!(result.is_err(), "Should not be able to open removed file");

        info!("Test 6: PASSED");
    }

    // Test 7: Move file
    info!("Test 7: Move file");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        vfs::Directory::create("/testmove", perms).expect("Failed to create directory");

        let file_perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file =
            vfs::File::create("/testmove/oldname", file_perms).expect("Failed to create file");
        // Resize file to allocate space before writing
        file.resisze(7).expect("Failed to resize file");
        file.write(0, b"move me").expect("Failed to write");
        drop(file);

        vfs::r#move("/testmove/oldname", "/testmove/newname").expect("Failed to move file");

        // Old name should not exist
        let result = vfs::File::open("/testmove/oldname", vfs::HandlePermissions::READ);
        assert!(result.is_err(), "Old name should not exist");

        // New name should exist with same content
        let file = vfs::File::open("/testmove/newname", vfs::HandlePermissions::READ)
            .expect("Failed to open moved file");
        let mut buffer = [0u8; 7];
        file.read(0, &mut buffer).expect("Failed to read");
        assert_eq!(&buffer, b"move me", "Moved file should have same content");

        info!("Test 7: PASSED");
    }

    info!("File operations tests completed");
}

/// Test metadata operations
fn test_metadata() {
    info!("Testing metadata operations...");

    // Test 1: File metadata
    info!("Test 1: File metadata");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/metafile", perms).expect("Failed to create file");

        // Resize file to allocate space before writing
        file.resisze(13).expect("Failed to resize file");
        file.write(0, b"metadata test").expect("Failed to write");

        let metadata = file.stat().expect("Failed to stat file");
        assert_eq!(metadata.r#type, vfs::NodeType::File, "Should be a file");
        assert_eq!(metadata.size, 13, "File size should be 13");
        assert!(
            metadata.permissions.contains(vfs::Permissions::READ),
            "Should have read permission"
        );
        assert!(
            metadata.permissions.contains(vfs::Permissions::WRITE),
            "Should have write permission"
        );

        info!("Test 1: PASSED");
    }

    // Test 2: Directory metadata
    info!("Test 2: Directory metadata");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        let dir = vfs::Directory::create("/metadir", perms).expect("Failed to create directory");

        let metadata = dir.stat().expect("Failed to stat directory");
        assert_eq!(
            metadata.r#type,
            vfs::NodeType::Directory,
            "Should be a directory"
        );
        assert!(
            metadata.permissions.contains(vfs::Permissions::READ),
            "Should have read permission"
        );
        assert!(
            metadata.permissions.contains(vfs::Permissions::EXECUTE),
            "Should have execute permission"
        );

        info!("Test 2: PASSED");
    }

    // Test 3: Change permissions
    info!("Test 3: Change permissions");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/metafile2", perms).expect("Failed to create file");

        let metadata = file.stat().expect("Failed to stat file");
        assert!(
            metadata.permissions.contains(vfs::Permissions::WRITE),
            "Should have write permission initially"
        );

        // Change permissions to read-only
        file.set_permissions(vfs::Permissions::READ)
            .expect("Failed to set permissions");

        let metadata = file.stat().expect("Failed to stat file");
        assert!(
            metadata.permissions.contains(vfs::Permissions::READ),
            "Should have read permission"
        );
        assert!(
            !metadata.permissions.contains(vfs::Permissions::WRITE),
            "Should not have write permission after change"
        );

        info!("Test 3: PASSED");
    }

    // Test 4: Timestamps
    info!("Test 4: Timestamps");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/metafile3", perms).expect("Failed to create file");

        let metadata1 = file.stat().expect("Failed to stat file");
        assert!(metadata1.created > 0, "Created timestamp should be set");
        assert!(metadata1.modified > 0, "Modified timestamp should be set");

        // Write to file
        timer::sleep(timer::Duration::from_milliseconds(10));
        // Resize file to allocate space before writing
        file.resisze(6).expect("Failed to resize file");
        file.write(0, b"update").expect("Failed to write");

        let metadata2 = file.stat().expect("Failed to stat file");
        assert_eq!(
            metadata2.created, metadata1.created,
            "Created timestamp should not change"
        );
        assert!(
            metadata2.modified >= metadata1.modified,
            "Modified timestamp should be updated"
        );

        info!("Test 4: PASSED");
    }

    info!("Metadata operations tests completed");
}

/// Test symlink operations
fn test_symlinks() {
    info!("Testing symlink operations...");

    // Test 1: Create symlink
    info!("Test 1: Create symlink");
    {
        let dir_perms =
            vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        vfs::Directory::create("/linktest", dir_perms).expect("Failed to create directory");

        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/linktest/target", perms).expect("Failed to create file");
        // Resize file to allocate space before writing
        file.resisze(14).expect("Failed to resize file");
        file.write(0, b"target content").expect("Failed to write");
        drop(file);

        vfs::Symlink::create("/linktest/link", "/linktest/target")
            .expect("Failed to create symlink");

        let symlink = vfs::Symlink::open("/linktest/link").expect("Failed to open symlink");
        let target = symlink.target().expect("Failed to read symlink target");
        assert_eq!(target, "/linktest/target", "Symlink target should match");

        info!("Test 1: PASSED");
    }

    // Test 2: Follow symlink
    info!("Test 2: Follow symlink");
    {
        let dir_perms =
            vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        vfs::Directory::create("/linktest2", dir_perms).expect("Failed to create directory");

        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/linktest2/target", perms).expect("Failed to create file");
        // Resize file to allocate space before writing
        file.resisze(12).expect("Failed to resize file");
        file.write(0, b"through link").expect("Failed to write");
        drop(file);

        vfs::Symlink::create("/linktest2/link", "/linktest2/target")
            .expect("Failed to create symlink");

        // Open file through symlink
        let file = vfs::File::open("/linktest2/link", vfs::HandlePermissions::READ)
            .expect("Failed to open file through symlink");

        let mut buffer = [0u8; 100];
        let read = file.read(0, &mut buffer).expect("Failed to read");
        assert_eq!(
            &buffer[..read],
            b"through link",
            "Should read through symlink"
        );

        info!("Test 2: PASSED");
    }

    // Test 3: Relative symlink
    info!("Test 3: Relative symlink");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        vfs::Directory::create("/linktest3", perms).expect("Failed to create directory");

        let file_perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        vfs::File::create("/linktest3/target", file_perms).expect("Failed to create file");

        vfs::Symlink::create("/linktest3/link", "target").expect("Failed to create symlink");

        let symlink = vfs::Symlink::open("/linktest3/link").expect("Failed to open symlink");
        let target = symlink.target().expect("Failed to read symlink target");
        assert_eq!(target, "target", "Relative symlink target should match");

        info!("Test 3: PASSED");
    }

    // Test 4: Symlink chain
    info!("Test 4: Symlink chain");
    {
        let dir_perms =
            vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        vfs::Directory::create("/linktest4", dir_perms).expect("Failed to create directory");

        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/linktest4/target", perms).expect("Failed to create file");
        // Resize file to allocate space before writing
        file.resisze(12).expect("Failed to resize file");
        file.write(0, b"end of chain").expect("Failed to write");
        drop(file);

        vfs::Symlink::create("/linktest4/link1", "/linktest4/target")
            .expect("Failed to create symlink");
        vfs::Symlink::create("/linktest4/link2", "/linktest4/link1")
            .expect("Failed to create symlink");

        // Open through chain
        let file = vfs::File::open("/linktest4/link2", vfs::HandlePermissions::READ)
            .expect("Failed to open file through symlink chain");

        let mut buffer = [0u8; 100];
        let read = file.read(0, &mut buffer).expect("Failed to read");
        assert_eq!(
            &buffer[..read],
            b"end of chain",
            "Should follow symlink chain"
        );

        info!("Test 4: PASSED");
    }

    info!("Symlink operations tests completed");
}

/// Test mount operations
fn test_mounts() {
    info!("Testing mount operations...");

    // Test 1: List mounts
    info!("Test 1: List mounts");
    {
        let mounts = vfs::list_mounts().expect("Failed to list mounts");
        assert!(!mounts.is_empty(), "Should have at least one mount");

        // Root should be mounted
        let root_mount = mounts.iter().find(|m| m.mount_point == "/");
        assert!(root_mount.is_some(), "Root should be mounted");
        assert_eq!(
            root_mount.unwrap().fs_port_name,
            "memfs-server",
            "Root should be memfs"
        );

        info!("Test 1: PASSED");
    }

    // Test 2: Mount and unmount
    info!("Test 2: Mount and unmount");
    {
        let perms = vfs::Permissions::READ | vfs::Permissions::WRITE | vfs::Permissions::EXECUTE;
        vfs::Directory::create("/mountpoint", perms).expect("Failed to create mount point");

        vfs::mount("/mountpoint", "memfs-server", &[]).expect("Failed to mount filesystem");

        // Verify it's in the mount list
        let mounts = vfs::list_mounts().expect("Failed to list mounts");
        let new_mount = mounts.iter().find(|m| m.mount_point == "/mountpoint");
        assert!(new_mount.is_some(), "New mount should be in list");

        // Create a file in the mounted filesystem
        let file_perms = vfs::Permissions::READ | vfs::Permissions::WRITE;
        let file = vfs::File::create("/mountpoint/testfile", file_perms)
            .expect("Failed to create file in mounted fs");
        // Resize file to allocate space before writing
        file.resisze(8).expect("Failed to resize file");
        file.write(0, b"in mount").expect("Failed to write");
        drop(file);

        // Unmount
        vfs::unmount("/mountpoint").expect("Failed to unmount filesystem");

        // Verify it's no longer in the mount list
        let mounts = vfs::list_mounts().expect("Failed to list mounts");
        let removed_mount = mounts.iter().find(|m| m.mount_point == "/mountpoint");
        assert!(removed_mount.is_none(), "Mount should be removed from list");

        info!("Test 2: PASSED");
    }

    info!("Mount operations tests completed");
}
