use core::{arch::asm, ops::Range, slice};

use libruntime::kobject::{self, Error, KObject, Permissions, ThreadPriority, PAGE_SIZE};

use super::offsets;

pub fn create_idle_process() -> Result<(), Error> {
    let (mobj, map_range) = prepare_mobj()?;

    // Create idle process
    let process = kobject::Process::create("idle").expect("Could not create idle process");
    let idle_mapping = process.map_mem(
        Some(map_range.start),
        map_range.len(),
        Permissions::READ | Permissions::EXECUTE,
        &mobj,
        0,
    )?;

    idle_mapping.leak();

    let entry_point = unsafe { core::mem::transmute(idle as unsafe extern "C" fn() -> _) };

    // Use raw API, no runtime management
    libsyscalls::thread::create(
        Some("idle"),
        unsafe { &process.handle() },
        true, // need privileged to run "hlt"
        ThreadPriority::Idle,
        entry_point, // same vaddr in idle process
        0,           // no stack
        0,           // no argument
        0,           // no TLS
    )?;

    Ok(())
}

fn prepare_mobj() -> Result<(kobject::MemoryObject, Range<usize>), Error> {
    let idle_range = offsets::idle();
    let idle_range_aligned = (idle_range.start / PAGE_SIZE * PAGE_SIZE)
        ..(((idle_range.end + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE);

    // Get memory object with section copied
    let mobj = kobject::MemoryObject::create(idle_range_aligned.len())?;

    let current_process = kobject::Process::current();
    let mapping = current_process.map_mem(
        None,
        idle_range_aligned.len(),
        Permissions::READ | Permissions::WRITE,
        &mobj,
        0,
    )?;
    let data = unsafe { mapping.as_buffer_mut() }.expect("Could not get mapping data");
    // Only copy relevant part inside slide (section is not aligned)
    let data = &mut data[(idle_range.start - idle_range_aligned.start)
        ..(idle_range.end - idle_range_aligned.start)];
    let source_data =
        unsafe { slice::from_raw_parts(idle_range.start as *const u8, idle_range.len()) };
    data.copy_from_slice(source_data);

    Ok((mobj, idle_range_aligned))
}

// Will run as idle process
#[naked]
#[no_mangle]
#[link_section = ".text_idle"]
unsafe extern "C" fn idle() -> ! {
    asm!(
        "
  2:
      hlt;
      jmp 2b;
  ",
        options(noreturn)
    );
}
