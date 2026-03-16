#!/usr/bin/env bash
set -euo pipefail

SECRET_NAME="${1:-github-cli-token}"

echo "Adding/updating global Patina secret '${SECRET_NAME}' as GH_TOKEN..."
echo "Paste your GitHub token when prompted."
patina secrets add "${SECRET_NAME}" --env GH_TOKEN --global

echo
echo "Verifying GitHub CLI auth through Patina secret injection..."
patina secrets run --global -- gh auth status
patina secrets run --global -- gh api user --jq '.login'

echo
echo "Done. Use this wrapper for gh commands:"
echo "  patina secrets run --global -- gh <args>"
echo
echo "Optional shell alias (add to ~/.zshrc or ~/.bashrc):"
echo "  alias ghs='patina secrets run --global -- gh'"
