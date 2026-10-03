#!/bin/sh
set -eu
src=${1:?source path required}
dst=${2:?destination path required}
cp "$src" "$dst"
printf '%s\n' "restored:$dst"
