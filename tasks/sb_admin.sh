#!/bin/bash

# FILE: sb_admin.sh
#
# Downloads an admin dashboard starter site from startbootstrap.com for use in
# testing and benchmarking.
#
# The absolute path to the unzipped site folder is printed to stdout. All other
# messages are printed to stderr so that consuming programs/scripts can easily
# capture the path.

set -e

DOWNLOAD_FROM="https://github.com/startbootstrap/startbootstrap-sb-admin-2/archive/gh-pages.zip"
SB_ADMIN="target/sites/sb_admin"
SB_ADMIN_ZIP="$SB_ADMIN/sb_admin.zip"

function download_sb_admin {
    >&2 echo "Downloading sb_admin.zip to $SB_ADMIN_ZIP"
    curl -sL $DOWNLOAD_FROM -o $SB_ADMIN_ZIP
    >&2 echo "Unzipping archive"
    unzip -q $SB_ADMIN_ZIP -d $SB_ADMIN >&2
    rm $SB_ADMIN_ZIP
}

# get to the project root no matter where this script was run from
pushd . > /dev/null
cd "$(dirname "$0")"
cd ..

mkdir -p target/sites
if [ ! -d $SB_ADMIN ]; then
    mkdir -p $SB_ADMIN
    download_sb_admin
elif [ $(ls -1 $SB_ADMIN | wc -l) -eq 0 ]; then
    download_sb_admin
else
    >&2 echo "sb_admin already downloaded"
fi

# find the folder that was unzipped. It should be the only entry in $SB_ADMIN.
sb_admin_site_folder=$(ls -1 $SB_ADMIN | head -n 1)

echo "$(pwd)/$SB_ADMIN/$sb_admin_site_folder"
popd > /dev/null
