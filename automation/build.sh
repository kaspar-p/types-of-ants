#!/bin/bash

# Build the Go CLI binary
cd cli
go build
cd ..

# Move the binary to the top level
mv cli/add add