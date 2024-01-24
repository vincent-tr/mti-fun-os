// https://www.reddit.com/r/C_Programming/comments/se3kgi/hello_world_without_libc/

typedef unsigned long long int uint64;

#define SYS_write 1
#define SYS_exit 60

#define STDOUT 1


int write(int fd, const char *buf, int length)
{
    int ret;

    asm("mov %1, %%rax\n\t"
        "mov %2, %%rdi\n\t"
        "mov %3, %%rsi\n\t"
        "mov %4, %%rdx\n\t"
        "syscall\n\t"
        "mov %%eax, %0"
        : "=r" (ret)
        : "r" ((uint64) SYS_write),
          "r" ((uint64) fd),
          "r" ((uint64) buf),
          "r" ((uint64) length)
        : "%rax", "%rdi", "%rsi", "%rdx");

    return ret;
}

int strlen(const char *str)
{
    const char *i = str;
    for (; *i; i++);
    return i - str;
}

_Noreturn void exit(int code)
{
    /* Infinite for-loop since this function can't return */
    for (;;) {
        asm("mov %0, %%rax\n\t"
            "mov %1, %%rdi\n\t"
            "syscall\n\t"
            :
            : "r" ((uint64) SYS_exit),
              "r" ((uint64) code)
            : "%rax", "%rdi");
    }
}
