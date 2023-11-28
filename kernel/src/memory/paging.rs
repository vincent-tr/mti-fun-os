/*

Kernel space : L4 set to 100000000
  => only one entry of L4 has to be copied accross processes
  => other levels can be shared


User space : L4 set from 000000000 to 011111111
 (discard page at 0)


Bootloader state:
INFO - Kernel      0xFFFF_8000_0000_0000
INFO - Stack       0xFFFF_8080_0000_0000
INFO - Framebuffer 0xFFFF_8100_0000_0000
INFO - Phys mem    0xFFFF_8180_0000_0000
INFO - Boot info   0xFFFF_8200_0000_0000

- kernel loaded at 0xFFFF_8000_0000_0000
- pysical memory mapping at 0xFFFF_C000_0000_0000
- kernel stack at ???
other structs we can drop for now

*/