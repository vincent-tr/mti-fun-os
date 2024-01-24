#define STDOUT 1

int write(int fd, const char *buf, int length);
int strlen(const char *str);
_Noreturn void exit(int code);

int main(void)
{
    const char *msg = "Hello nolibc!\n";

    write(STDOUT, msg, strlen(msg));

    return 0;
}

void _start(void)
{
    int main_ret = main();
    exit(main_ret);
}
