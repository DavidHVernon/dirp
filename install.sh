#!/bin/bash
set -x
cd /usr/local/bin
if test -f "dirp.zip"; then
    rm dirp.zip
fi
curl https://raw.githubusercontent.com/DavidHVernon/dirp/master/release/0.1.0/dirp.zip --output dirp.zip
if test -f "dirp"; then
    rm dirp
fi
unzip dirp.zip
rm dirp.zip 
