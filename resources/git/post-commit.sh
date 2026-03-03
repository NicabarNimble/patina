#!/bin/sh
# Install: ln -sf ../../resources/git/post-commit.sh .git/hooks/post-commit
command -v patina >/dev/null 2>&1 || exit 0
patina hook post-commit
