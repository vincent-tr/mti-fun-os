use x86_64::{
  structures::paging::PhysFrame,
  PhysAddr,
};


// https://wiki.osdev.org/Page_Frame_Allocation

pub struct FrameAllocator<'a> {
  bitmap: &'a mut [u64],
}

impl<'a> FrameAllocator<'a> {
  pub fn allocate(&mut self) -> PhysFrame {
      for (word_index, word) in self.bitmap.iter_mut().enumerate() {
          let bit_index = word.leading_ones() as usize;
          if bit_index < 64 {
              *word |= 1u64 << bit_index;

              let page_offset = word_index * 64 + bit_index;
              let address = PhysAddr::new((page_offset * PAGE_SIZE) as u64);
              return PhysFrame::from_start_address(address).unwrap();
          }
      }

      panic!("Frame allocator: allocation failure, no more free frame");
  }

  pub fn deallocate(&mut self, frame: PhysFrame) {
      let page_offset = frame.start_address().as_u64() as usize / PAGE_SIZE;
      let word_index = page_offset / 64;
      let bit_index = (page_offset - (word_index * 64)) as usize;
      let word = unsafe { self.bitmap.get_unchecked_mut(word_index) };
      *word &= !(1u64 << bit_index);
  }
}
