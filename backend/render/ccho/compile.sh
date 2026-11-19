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
    xelatex -no-shell-escape -interaction=nonstopmode -halt-on-error \
        -jobname="$1" "$1.tex" >> /tmp/compile.log 2>&1
done
if grep -Eq 'Missing character:|Overfull .box' "$1.log"; then
    exit 1
fi
cat "$1.pdf"
