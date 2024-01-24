#!/bin/bash

# static EXEC
# go build hello.go

# static PIE
# go build -ldflags '-linkmode external -s -w -extldflags "--static-pie"' -buildmode=pie hello.go

# static EXEC
gcc -static -o hello hello.c

# no libc static
# gcc -static -nostdlib -Wno-builtin-declaration-mismatch -o hello hello_nolibc.c

# no libc PIE
# gcc -nostdlib -Wno-builtin-declaration-mismatch -o hello hello_nolibc.c