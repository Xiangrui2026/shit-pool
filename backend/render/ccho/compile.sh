#!/bin/sh
set -eu
umask 077

case "$1" in
    questions|answers) ;;
    *) exit 2 ;;
esac

cat > /tmp/input.zip
unzip -q /tmp/input.zip
rm /tmp/input.zip
for pass in 1 2; do
    if ! xelatex -no-shell-escape -interaction=nonstopmode -halt-on-error \
        -jobname="$1" "$1.tex" >> /tmp/compile.log 2>&1; then
        if [ "${SHIT_POOL_CI_TEX_DEBUG:-}" = 1 ]; then
            tail -n 15 /tmp/compile.log >&2
        fi
        exit 1
    fi
done
if grep -Eq 'Missing character:|Overfull .box' "$1.log"; then
    if [ "${SHIT_POOL_CI_TEX_DEBUG:-}" = 1 ]; then
        grep -E 'Missing character:|Overfull .box' "$1.log" >&2
    fi
    exit 1
fi
cat "$1.pdf"
