#!/bin/bash

# static EXEC
# go build hello.go

# static PIE
go build -ldflags '-linkmode external -s -w -extldflags "--static-pie"' -buildmode=pie hello.go