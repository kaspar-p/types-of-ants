#!/bin/bash
# From: https://medium.com/swlh/compiling-rust-for-raspberry-pi-arm-922b55dbb050

# The build script to start the server with the right website content in it
readonly TARGET_ARCH=armv7-unknown-linux-musleabihf

readonly TEMP_DIR=$ANTHILL_ROOT/ant-building-projects/temp
readonly DEST_DIR=$ANTHILL_ROOT/ant-building-projects/tars

if [[ -z $ANTHILL_ROOT ]]; then
  echo "No \$ANTHILL_ROOT variable defined! Needs to be the root of the git repository!"
  exit 1
fi

cd $ANTHILL_ROOT

mkdir -p $TEMP_DIR
mkdir -p $DEST_DIR

# Build the website
cd ant-on-the-web/website
mv .env.local .env.local-temp
echo "NEXT_PUBLIC_ENVIRONMENT=beta" >> .env.local
npm run build
rm -rf .env.local
mv .env.local-temp .env.local
cd ../..

# Move the output of the website into static directory
mkdir -p $TEMP_DIR/static
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
cd $TEMP_DIR
cd ..
tar -czf $DEST_DIR/$ARTIFACT_NAME -C temp .

# Add the new tarfile for staging
git add $DEST_DIR/$ARTIFACT_NAME