#!/bin/sh
# Every .rs file under a crate's src/ is declared by its parent: src/a/b.rs needs `mod b;` in
# src/a.rs, a top-level file needs it in the crate root. rustc silently skips a file nobody
# declares, and there are no mod.rs files. A binary's main.rs is a root of its own.
# Usage: lint/orphans.sh <src dir> <crate root file name>, e.g. lint/orphans.sh desktop/src main.rs
set -eu

src=$1
root=$2
status=0
if find "$src" -name mod.rs | grep .; then
    echo "mod.rs found: declare the folder with a sibling <name>.rs instead"
    status=1
fi
for file in $(find "$src" -name '*.rs' ! -path "$src/$root" ! -path "$src/main.rs"); do
    dir=$(dirname "$file")
    parent="$dir.rs"
    [ "$dir" = "$src" ] && parent="$src/$root"
    name=$(basename "$file" .rs)
    if ! grep -qE "^[[:space:]]*(pub(\([a-z]+\))? )?mod $name;" "$parent" 2>/dev/null; then
        echo "not declared anywhere, so never compiled: $file"
        status=1
    fi
done
exit $status
