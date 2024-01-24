#!/bin/bash

# static EXEC
# go build hello.go

# static PIE
# go build -ldflags '-linkmode external -s -w -extldflags "--static-pie"' -buildmode=pie hello.go

# static EXEC
# gcc -static -o hello hello.c

# no libc static
# gcc -static -nostdlib -Wno-builtin-declaration-mismatch -o hello hello_nolibc.c

# no libc PIE
# gcc -nostdlib -Wno-builtin-declaration-mismatch -o hello hello_nolibc.c

# no libc shared
gcc -nostdlib -Wno-builtin-declaration-mismatch -shared -o shared.so shared_nolibc.c
gcc -nostdlib -Wno-builtin-declaration-mismatch -L. -l:shared.so -o hello hello_dyn_nolibc.c

# run with LD_LIBRARY_PATH=. ./hello