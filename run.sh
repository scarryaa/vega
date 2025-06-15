#!/bin/sh

echo "Building..."
cargo build

if [ $? -ne 0 ]; then
    echo "The build has failed. Exiting"
    exit 1
fi

./target/debug/scout
