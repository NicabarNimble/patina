#!/bin/sh
# Install: ln -sf ../../resources/git/post-merge.sh .git/hooks/post-merge
command -v patina >/dev/null 2>&1 || exit 0
patina hook post-merge
