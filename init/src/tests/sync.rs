use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use libruntime::{
    ipc::Handles,
    kobject::{self, MemoryObject, Permissions, Process, ThreadOptions, PAGE_SIZE},
    sync::{Mutex, RwLock},
    timer::{self, Duration},
};
use libsyscalls::futex;
use log::info;

/// Test futex operations
#[allow(dead_code)]
pub fn test_futex() {
    info!("Testing futex operations...");

    // Allocate shared memory for futex
    let futex_var = AtomicU32::new(0);
    let futex_ptr = &futex_var as *const AtomicU32 as *const u32;
    let futex = unsafe { &*futex_ptr };

    // Test 1: Basic wait/wake
    info!("Test 1: Basic wait/wake");
    {
        let (ready_reader, ready_sender) =
            kobject::Port::create(None).expect("failed to create port");
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");

        let futex_addr = futex_ptr as usize;
        let waiter = move || {
            let futex = unsafe { &*(futex_addr as *const u32) };
            let futex_atomic = unsafe { &*(futex_addr as *const AtomicU32) };

            // Signal ready
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            ready_sender.send(&mut msg).expect("send failed");

            // Wait on futex
            let result = futex::wait(futex, 0);
            assert!(result.is_ok(), "futex wait failed: {:?}", result);

            // Check value changed
            assert_eq!(futex_atomic.load(Ordering::SeqCst), 1);

            // Signal done
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            done_sender.send(&mut msg).expect("send failed");
        };

        let mut options = ThreadOptions::default();
        options.name("futex-waiter");
        let _thread = kobject::Thread::start(waiter, options).expect("failed to start thread");

        // Wait for waiter to be ready
        ready_reader.blocking_receive().expect("receive failed");

        // Give it time to sleep on futex
        timer::sleep(Duration::from_milliseconds(100));

        // Change value and wake
        futex_var.store(1, Ordering::SeqCst);
        let woken = futex::wake(futex, 1).expect("futex wake failed");
        assert_eq!(woken, 1, "should have woken 1 thread");

        // Wait for completion
        done_reader.blocking_receive().expect("receive failed");
        info!("Test 1: PASSED");
    }

    // Test 2: Wake with no waiters
    info!("Test 2: Wake with no waiters");
    {
        let woken = futex::wake(futex, 1).expect("futex wake failed");
        assert_eq!(woken, 0, "should have woken 0 threads");
        info!("Test 2: PASSED");
    }

    // Test 3: Multiple waiters
    info!("Test 3: Multiple waiters (wake all)");
    {
        futex_var.store(0, Ordering::SeqCst);

        const NUM_WAITERS: usize = 5;
        let mut done_readers = alloc::vec::Vec::new();

        for i in 0..NUM_WAITERS {
            let (ready_reader, ready_sender) =
                kobject::Port::create(None).expect("failed to create port");
            let (done_reader, done_sender) =
                kobject::Port::create(None).expect("failed to create port");
            done_readers.push(done_reader);

            let futex_addr = futex_ptr as usize;
            let waiter = move || {
                let futex = unsafe { &*(futex_addr as *const u32) };

                // Signal ready
                let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
                ready_sender.send(&mut msg).expect("send failed");

                // Wait on futex
                let result = futex::wait(futex, 0);
                assert!(result.is_ok(), "futex wait failed: {:?}", result);

                // Signal done
                let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
                done_sender.send(&mut msg).expect("send failed");
            };

            let mut options = ThreadOptions::default();
            let name = alloc::format!("futex-waiter-{}", i);
            options.name(&name);
            kobject::Thread::start(waiter, options).expect("failed to start thread");

            ready_reader.blocking_receive().expect("receive failed");
        }

        // Give them time to sleep
        timer::sleep(Duration::from_milliseconds(100));

        // Wake all
        let woken = futex::wake(futex, NUM_WAITERS).expect("futex wake failed");
        assert_eq!(woken, NUM_WAITERS, "should have woken all threads");

        // Wait for all to complete
        for done_reader in done_readers {
            done_reader.blocking_receive().expect("receive failed");
        }
        info!("Test 3: PASSED");
    }

    // Test 4: Value changed (spurious wakeup check)
    info!("Test 4: Value mismatch");
    {
        futex_var.store(42, Ordering::SeqCst);

        let result = futex::wait(futex, 0);
        assert!(result.is_err(), "futex wait should fail on value mismatch");
        info!("Test 4: PASSED");
    }

    // Test 5: Unmap wakes blocked threads
    info!("Test 5: Unmap wakes blocked threads");
    {
        // Create shared memory for futex
        let mem_obj = MemoryObject::create(PAGE_SIZE).expect("failed to create memory object");
        let process = Process::current();

        let mapping = process
            .map_mem(
                None,
                PAGE_SIZE,
                Permissions::READ | Permissions::WRITE,
                &mem_obj,
                0,
            )
            .expect("failed to map memory");

        let mapped_futex_addr = mapping.address() as *mut AtomicU32;
        let mapped_futex = unsafe { &*mapped_futex_addr };

        // Initialize futex value
        mapped_futex.store(0, Ordering::SeqCst);

        let (ready_reader, ready_sender) =
            kobject::Port::create(None).expect("failed to create port");
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");

        let futex_addr = mapped_futex_addr as usize;
        let waiter = move || {
            let futex = unsafe { &*(futex_addr as *const u32) };

            // Signal ready
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            ready_sender.send(&mut msg).expect("send failed");

            // Wait on futex - should be woken by unmap
            let result = futex::wait(futex, 0);
            assert!(
                result.is_ok(),
                "futex wait should succeed when woken by unmap: {:?}",
                result
            );

            // Signal done
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            done_sender.send(&mut msg).expect("send failed");
        };

        let mut options = ThreadOptions::default();
        options.name("futex-unmap-waiter");
        let _thread = kobject::Thread::start(waiter, options).expect("failed to start thread");

        // Wait for waiter to be ready
        ready_reader.blocking_receive().expect("receive failed");

        // Give it time to sleep on futex
        timer::sleep(Duration::from_milliseconds(100));

        // Unmap the region - this should wake the waiter
        drop(mapping);

        // Wait for completion
        done_reader.blocking_receive().expect("receive failed");
        info!("Test 5: PASSED");
    }

    info!("All futex tests PASSED!");
}

/// Test Mutex synchronization primitive
#[allow(dead_code)]
pub fn test_mutex() {
    info!("Testing Mutex operations...");

    // Test 1: Basic lock/unlock
    info!("Test 1: Basic lock/unlock");
    {
        let mutex = Mutex::new(0);
        {
            let mut guard = mutex.lock();
            *guard = 42;
        }
        let guard = mutex.lock();
        assert_eq!(*guard, 42);
        info!("Test 1: PASSED");
    }

    // Test 2: try_lock
    info!("Test 2: try_lock");
    {
        let mutex = Mutex::new(0);
        let guard1 = mutex.lock();
        assert!(
            mutex.try_lock().is_none(),
            "try_lock should fail when locked"
        );
        drop(guard1);
        assert!(
            mutex.try_lock().is_some(),
            "try_lock should succeed when unlocked"
        );
        info!("Test 2: PASSED");
    }

    // Test 3: Multiple threads contention
    info!("Test 3: Multiple threads contention");
    {
        let mutex = Arc::new(Mutex::new(0));
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");
        let done_sender = Arc::new(done_sender);

        const NUM_THREADS: usize = 5;
        const INCREMENTS_PER_THREAD: usize = 100;

        for i in 0..NUM_THREADS {
            let mutex = Arc::clone(&mutex);
            let done_sender = Arc::clone(&done_sender);

            let worker = move || {
                for _ in 0..INCREMENTS_PER_THREAD {
                    let mut guard = mutex.lock();
                    *guard += 1;
                }

                let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
                done_sender.send(&mut msg).expect("send failed");
            };

            let mut options = ThreadOptions::default();
            let name = alloc::format!("mutex-worker-{}", i);
            options.name(&name);
            kobject::Thread::start(worker, options).expect("failed to start thread");
        }

        // Wait for all threads
        for _ in 0..NUM_THREADS {
            done_reader.blocking_receive().expect("receive failed");
        }

        let final_value = *mutex.lock();
        assert_eq!(
            final_value,
            (NUM_THREADS * INCREMENTS_PER_THREAD) as i32,
            "mutex failed to protect shared data"
        );
        info!("Test 3: PASSED");
    }

    // Test 4: Lock ordering (no deadlock with single mutex)
    info!("Test 4: Lock ordering");
    {
        let mutex = Arc::new(Mutex::new(0));
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");

        let mutex_clone = Arc::clone(&mutex);
        let worker = move || {
            for _ in 0..10 {
                let _guard = mutex_clone.lock();
                timer::sleep(Duration::from_milliseconds(1));
            }
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            done_sender.send(&mut msg).expect("send failed");
        };

        let mut options = ThreadOptions::default();
        options.name("mutex-sleeper");
        kobject::Thread::start(worker, options).expect("failed to start thread");

        // Main thread also acquires lock
        for _ in 0..10 {
            let _guard = mutex.lock();
            timer::sleep(Duration::from_milliseconds(1));
        }

        done_reader.blocking_receive().expect("receive failed");
        info!("Test 4: PASSED");
    }

    info!("All Mutex tests PASSED!");
}

/// Test RwLock synchronization primitive
#[allow(dead_code)]
pub fn test_rwlock() {
    info!("Testing RwLock operations...");

    // Test 1: Basic read/write
    info!("Test 1: Basic read/write");
    {
        let lock = RwLock::new(alloc::vec![1, 2, 3]);
        {
            let r = lock.read();
            assert_eq!(*r, alloc::vec![1, 2, 3]);
        }
        {
            let mut w = lock.write();
            w.push(4);
        }
        {
            let r = lock.read();
            assert_eq!(*r, alloc::vec![1, 2, 3, 4]);
        }
        info!("Test 1: PASSED");
    }

    // Test 2: Multiple concurrent readers
    info!("Test 2: Multiple concurrent readers");
    {
        let lock = Arc::new(RwLock::new(42));
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");
        let done_sender = Arc::new(done_sender);

        const NUM_READERS: usize = 10;

        for i in 0..NUM_READERS {
            let lock = Arc::clone(&lock);
            let done_sender = Arc::clone(&done_sender);

            let reader = move || {
                let guard = lock.read();
                assert_eq!(*guard, 42);
                timer::sleep(Duration::from_milliseconds(10));

                let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
                done_sender.send(&mut msg).expect("send failed");
            };

            let mut options = ThreadOptions::default();
            let name = alloc::format!("reader-{}", i);
            options.name(&name);
            kobject::Thread::start(reader, options).expect("failed to start thread");
        }

        // Wait for all readers
        for _ in 0..NUM_READERS {
            done_reader.blocking_receive().expect("receive failed");
        }

        info!("Test 2: PASSED");
    }

    // Test 3: Writer excludes readers
    info!("Test 3: Writer excludes readers");
    {
        let lock = Arc::new(RwLock::new(0));
        let (ready_reader, ready_sender) =
            kobject::Port::create(None).expect("failed to create port");
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");

        let lock_clone = Arc::clone(&lock);
        let reader = move || {
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            ready_sender.send(&mut msg).expect("send failed");

            let guard = lock_clone.read();
            let value = *guard;
            assert!(
                value == 0 || value == 100,
                "reader saw intermediate value: {}",
                value
            );

            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            done_sender.send(&mut msg).expect("send failed");
        };

        let mut options = ThreadOptions::default();
        options.name("reader");
        kobject::Thread::start(reader, options).expect("failed to start thread");

        // Wait for reader to be ready
        ready_reader.blocking_receive().expect("receive failed");

        // Acquire write lock
        let mut guard = lock.write();
        *guard = 100;
        timer::sleep(Duration::from_milliseconds(50));
        drop(guard);

        // Wait for reader
        done_reader.blocking_receive().expect("receive failed");
        info!("Test 3: PASSED");
    }

    // Test 4: Reader/writer alternation
    info!("Test 4: Reader/writer alternation");
    {
        let lock = Arc::new(RwLock::new(0));
        let (done_reader, done_sender) =
            kobject::Port::create(None).expect("failed to create port");
        let done_sender = Arc::new(done_sender);

        // Start writer thread
        let lock_clone = Arc::clone(&lock);
        let done_sender_clone = Arc::clone(&done_sender);
        let writer = move || {
            for i in 0..5 {
                let mut guard = lock_clone.write();
                *guard = i * 10;
                timer::sleep(Duration::from_milliseconds(5));
            }
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            done_sender_clone.send(&mut msg).expect("send failed");
        };

        let mut options = ThreadOptions::default();
        options.name("writer");
        kobject::Thread::start(writer, options).expect("failed to start thread");

        // Start reader thread
        let lock_clone = Arc::clone(&lock);
        let reader = move || {
            for _ in 0..5 {
                let guard = lock_clone.read();
                let value = *guard;
                assert!(value % 10 == 0, "reader saw invalid value: {}", value);
                timer::sleep(Duration::from_milliseconds(5));
            }
            let mut msg = unsafe { kobject::Message::new::<u32>(&0, Handles::new().into()) };
            done_sender.send(&mut msg).expect("send failed");
        };

        let mut options = ThreadOptions::default();
        options.name("reader");
        kobject::Thread::start(reader, options).expect("failed to start thread");

        // Wait for both
        done_reader.blocking_receive().expect("receive failed");
        done_reader.blocking_receive().expect("receive failed");

        info!("Test 4: PASSED");
    }

    // Test 5: try_read and try_write
    info!("Test 5: try_read and try_write");
    {
        let lock = RwLock::new(0);

        let r1 = lock.try_read();
        assert!(r1.is_some(), "try_read should succeed");

        let r2 = lock.try_read();
        assert!(r2.is_some(), "multiple try_read should succeed");

        let w = lock.try_write();
        assert!(w.is_none(), "try_write should fail with active readers");

        drop(r1);
        drop(r2);

        let w = lock.try_write();
        assert!(w.is_some(), "try_write should succeed when unlocked");

        let r = lock.try_read();
        assert!(r.is_none(), "try_read should fail with active writer");

        info!("Test 5: PASSED");
    }

    info!("All RwLock tests PASSED!");
}
