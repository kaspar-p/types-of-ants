#!/bin/bash
# From: https://medium.com/swlh/compiling-rust-for-raspberry-pi-arm-922b55dbb050

# The build script to start the server with the right website content in it
readonly TARGET_ARCH=armv7-unknown-linux-musleabihf

readonly TEMP_DIR=$ANTHILL_ROOT/ant-building-projects/temp
readonly DEST_DIR=$ANTHILL_ROOT/ant-building-projects/tars

mkdir -p $TEMP_DIR
mkdir -p $DEST_DIR

cwd="$(basename $(pwd))"
if [[ "$cwd" != "types-of-ants" ]]; then
  echo "ERROR: This script needs to be run from types-of-ants/ directory, the root of the project!"
  exit 1
fi

# Build the website
cd ant-on-the-web/website && npm run build
cd ../..

# Move the output of the website into static directory
cp -R ant-on-the-web/website/out/* "$TEMP_DIR/static"

# Compile
cd ant-on-the-web/server
set -o errexit
set -o nounset
set -o pipefail
set -o xtrace
cargo build --release --target=${TARGET_ARCH}
cd ../..
cp ./target/$TARGET_ARCH/release/ant-on-the-web $TEMP_DIR/ant-on-the-web

# Package ant-on-the-web
readonly ARTIFACT_NAME=artifact-ant-on-the-web.tar
tar -cf $ARTIFACT_NAME $TEMP_DIR
mv $ARTIFACT_NAME $DEST_DIR/$ARTIFACT_NAME

# Add the new tarfile for staging
git add $DEST_DIR/$ARTIFACT_NAME