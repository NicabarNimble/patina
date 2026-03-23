#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SESSIONS_DIR="$ROOT/layer/sessions"

python - "$ROOT" "$SESSIONS_DIR" <<'PY'
import pathlib
import re
import subprocess
import sys

root = pathlib.Path(sys.argv[1])
sessions_dir = pathlib.Path(sys.argv[2])

created = []
skipped = []

for path in sorted(sessions_dir.glob("*.md")):
    text = path.read_text(errors="ignore")
    if not text.startswith("---\n"):
        continue
    end = text.find("\n---\n", 4)
    if end == -1:
        continue

    fm = text[4:end]
    m_end = re.search(r"^\s*end_tag:\s*(.+)\s*$", fm, flags=re.M)
    m_start = re.search(r"^\s*starting_commit:\s*(.+)\s*$", fm, flags=re.M)
    if not m_end:
        continue

    end_tag = m_end.group(1).strip().strip('"\'')
    if end_tag in ("", "null", "~"):
        continue

    exists = subprocess.run(
        ["git", "rev-parse", "-q", "--verify", f"refs/tags/{end_tag}"],
        cwd=root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if exists.returncode == 0:
        continue

    if not m_start:
        skipped.append((path.name, end_tag, "missing starting_commit"))
        continue

    starting_commit = m_start.group(1).strip().strip('"\'')
    commit_ok = subprocess.run(
        ["git", "cat-file", "-e", f"{starting_commit}^{{commit}}"],
        cwd=root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    if commit_ok.returncode != 0:
        skipped.append((path.name, end_tag, f"unknown commit {starting_commit}"))
        continue

    message = f"Backfilled historical session end tag from {path.name}"
    create = subprocess.run(
        [
            "git",
            "tag",
            "-a",
            end_tag,
            starting_commit,
            "-m",
            message,
        ],
        cwd=root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    if create.returncode != 0:
        skipped.append((path.name, end_tag, create.stderr.strip() or "git tag failed"))
        continue

    created.append((path.name, end_tag, starting_commit))

print(f"created={len(created)}")
for file_name, end_tag, starting_commit in created:
    print(f"CREATED\t{file_name}\t{end_tag}\t{starting_commit}")

print(f"skipped={len(skipped)}")
for file_name, end_tag, reason in skipped:
    print(f"SKIPPED\t{file_name}\t{end_tag}\t{reason}")
PY
