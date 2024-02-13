#!/bin/bash
set -e
cargo build --release
pushd ./target/release
if test -f "dirp.zip"; then
    rm dirp.zip
fi
codesign -s  "Developer ID Application: David Vernon (3CT7AJ22D9)" --options=runtime dirp
zip dirp.zip dirp
codesign -s  "Developer ID Application: David Vernon (3CT7AJ22D9)" --options=runtime dirp.zip
xcrun notarytool submit dirp.zip  --keychain-profile "rust-notarize-app" --wait
popd
