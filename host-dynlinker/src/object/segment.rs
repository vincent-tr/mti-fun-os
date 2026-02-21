use crate::{
    PAGE_SIZE, align_down, align_up,
    kobject::{Mapping, Permissions, Process},
};
use core::ops::Range;
use xmas_elf::program;

#[derive(Debug)]
pub struct Segment<'a> {
    vm_rel_segment: Range<usize>,
    buffer_in_mapping: Range<usize>,
    mapping: Mapping<'a>,
    perms: Permissions,
}

impl<'a> Segment<'a> {
    pub fn new(
        process: &'a Process,
        program_header: &program::ProgramHeader,
        addr_offset: usize,
    ) -> Result<Self, crate::kobject::Error> {
        let vm_rel_segment = program_header.virtual_addr() as usize
            ..(program_header.virtual_addr() + program_header.mem_size()) as usize;
        let vm_rel_segment_aligned =
            align_down(vm_rel_segment.start, PAGE_SIZE)..align_up(vm_rel_segment.end, PAGE_SIZE);

        // while copying data, always setup RW access
        let mapping = process.map_mem(
            Some(vm_rel_segment_aligned.start + addr_offset),
            vm_rel_segment_aligned.len(),
            Permissions::READ | Permissions::WRITE,
        )?;

        let buffer_in_mapping_start = vm_rel_segment.start - vm_rel_segment_aligned.start;
        let buffer_in_mapping_len = program_header.mem_size() as usize;
        let buffer_in_mapping =
            buffer_in_mapping_start..(buffer_in_mapping_start + buffer_in_mapping_len);

        let mut perms = Permissions::NONE;

        if program_header.flags().is_execute() {
            perms |= Permissions::EXECUTE;
        }

        if program_header.flags().is_read() {
            perms |= Permissions::READ;
        }

        if program_header.flags().is_write() {
            perms |= Permissions::WRITE;
        }

        Ok(Self {
            vm_rel_segment,
            buffer_in_mapping,
            mapping,
            perms,
        })
    }

    pub fn vm_range(&self) -> &Range<usize> {
        &self.vm_rel_segment
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        let buffer = unsafe { self.mapping.as_buffer_mut().expect("Could not access data") };
        &mut buffer[self.buffer_in_mapping.clone()]
    }

    pub fn finalize(mut self) -> Result<(), crate::kobject::Error> {
        unsafe { self.mapping.update_permissions(self.perms) }?;
        self.mapping.leak();

        Ok(())
    }
}
