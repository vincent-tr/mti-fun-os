/*

Bootloader state:
INFO - Kernel      0xFFFF_8000_0000_0000
INFO - Phys mem    0xFFFF_8080_0000_0000
INFO - Stack       0xFFFF_8100_0000_0000
INFO - Framebuffer 0xFFFF_8180_0000_0000
INFO - Boot info   0xFFFF_8200_0000_0000

Must keep Kernel + Phys mem mapping
Must keep stack until context switch
Can drop Framebuffer and bootinfo

64 bits virtual address

64         48         39         30         21         12         0
   Unused   | Level 4  | Level 3  | Level 2  | Level 1  | Page Offset   

bits 48 -> 64 must match bit 47

Level 4 entry size: 0x80_0000_0000 512G
Level 3 entry size: 0x4000_0000      1G
Level 2 entry size: 0x20_0000        2M
Level 1 entry size: 0x1000           4K

*/
