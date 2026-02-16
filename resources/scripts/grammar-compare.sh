#!/usr/bin/env bash
#
# grammar-compare.sh — Compare grammar plugin output vs compiled-in processor
#
# Usage:
#   grammar-compare.sh <grammar> [ref-repo-path]
#   grammar-compare.sh all
#
# Examples:
#   grammar-compare.sh go                              # uses default ref repo
#   grammar-compare.sh go ~/.patina/cache/repos/marcus/sidecar
#   grammar-compare.sh all                             # run all 7 grammars
#
# Exit codes:
#   0 = all thresholds met
#   1 = one or more thresholds exceeded
#   2 = usage error

set -euo pipefail

REPOS_BASE="${HOME}/.patina/cache/repos"
PIPELINE_DIR="${HOME}/.patina/pipeline"
BACKUP_DIR="/tmp/grammar-compare-backup-$$"

# Default ref repos per grammar
declare -A DEFAULT_REPOS=(
  [go]="marcus/sidecar"
  [c]="libsdl-org/SDL"
  [cpp]="unum-cloud/USearch"
  [python]="openai/codex"
  [javascript]="litecanvas/game-engine"
  [solidity]="dustproject/dust"
  [typescript]="google-gemini/gemini-cli"
)

# Plugin directory names
declare -A PLUGIN_DIRS=(
  [go]="grammar-go"
  [c]="grammar-c"
  [cpp]="grammar-cpp"
  [python]="grammar-python"
  [javascript]="grammar-javascript"
  [solidity]="grammar-solidity"
  [typescript]="grammar-typescript"
)

# Thresholds (percentage delta)
declare -A THRESHOLDS=(
  [code_search]=5
  [function_facts]=2
  [type_vocabulary]=5
  [import_facts]=5
  [call_graph]=10
  [constant_facts]=5
  [member_facts]=5
)

# Tables to compare
TABLES="code_search function_facts type_vocabulary import_facts call_graph constant_facts member_facts"

# ─── Helpers ──────────────────────────────────────────────────────────────

usage() {
  echo "Usage: grammar-compare.sh <grammar|all> [ref-repo-path]"
  echo ""
  echo "Grammars: go c cpp python javascript solidity typescript"
  echo ""
  echo "Compares plugin extraction vs compiled-in processor on a ref repo."
  echo "Reports per-table deltas and pass/fail against thresholds."
  exit 2
}

get_db_path() {
  local repo_path="$1"
  echo "${repo_path}/.patina/local/data/patina.db"
}

count_rows() {
  local db="$1"
  local table="$2"
  sqlite3 "$db" "SELECT COUNT(*) FROM ${table};" 2>/dev/null || echo "0"
}

count_files_with_data() {
  local db="$1"
  local table="$2"
  local file_col="file"
  # code_search uses 'path' not 'file'
  if [ "$table" = "code_search" ]; then
    file_col="path"
  fi
  sqlite3 "$db" "SELECT COUNT(DISTINCT ${file_col}) FROM ${table};" 2>/dev/null || echo "0"
}

# ─── Core comparison ─────────────────────────────────────────────────────

run_comparison() {
  local grammar="$1"
  local repo_path="$2"
  local plugin_dir="${PLUGIN_DIRS[$grammar]}"
  local plugin_path="${PIPELINE_DIR}/${plugin_dir}"
  local db_path
  db_path=$(get_db_path "$repo_path")

  if [ ! -d "$repo_path" ]; then
    echo "ERROR: Ref repo not found: $repo_path"
    return 1
  fi

  if [ ! -d "$plugin_path" ]; then
    echo "ERROR: Plugin not installed: $plugin_path"
    return 1
  fi

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  GRAMMAR: ${grammar}"
  echo "  PLUGIN:  ${plugin_dir}"
  echo "  REPO:    ${repo_path}"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""

  # ── Pass 1: Scrape with plugin ──────────────────────────────────────
  echo "▸ Pass 1: Scraping with plugin..."
  (cd "$repo_path" && patina scrape 2>&1 | tail -1)

  # Save plugin counts
  declare -A plugin_counts
  for table in $TABLES; do
    plugin_counts[$table]=$(count_rows "$db_path" "$table")
  done
  local plugin_files
  plugin_files=$(count_files_with_data "$db_path" "function_facts")

  # ── Disable plugin ──────────────────────────────────────────────────
  echo "▸ Disabling plugin (moving aside)..."
  mkdir -p "$BACKUP_DIR"
  mv "$plugin_path" "${BACKUP_DIR}/${plugin_dir}"

  # ── Pass 2: Scrape without plugin (compiled-in) ─────────────────────
  echo "▸ Pass 2: Scraping with compiled-in processor..."
  (cd "$repo_path" && patina scrape 2>&1 | tail -1)

  # Save compiled-in counts
  declare -A compiled_counts
  for table in $TABLES; do
    compiled_counts[$table]=$(count_rows "$db_path" "$table")
  done
  local compiled_files
  compiled_files=$(count_files_with_data "$db_path" "function_facts")

  # ── Restore plugin ──────────────────────────────────────────────────
  echo "▸ Restoring plugin..."
  mv "${BACKUP_DIR}/${plugin_dir}" "$plugin_path"
  echo ""

  # ── Report ──────────────────────────────────────────────────────────
  local all_pass=true

  printf "  %-20s %8s %10s %8s %6s\n" "TABLE" "PLUGIN" "COMPILED" "DELTA" "STATUS"
  printf "  %-20s %8s %10s %8s %6s\n" "─────────────────────" "────────" "──────────" "────────" "──────"

  for table in $TABLES; do
    local p_count="${plugin_counts[$table]}"
    local c_count="${compiled_counts[$table]}"
    local delta=0
    local pct="0.0"
    local status="PASS"

    if [ "$c_count" -gt 0 ]; then
      delta=$((p_count - c_count))
      # Use awk for floating point
      pct=$(awk "BEGIN { printf \"%.1f\", ($delta / $c_count) * 100 }")
    elif [ "$p_count" -gt 0 ]; then
      pct="+∞"
      status="PASS"  # Plugin found data that compiled-in didn't — acceptable
    fi

    local threshold="${THRESHOLDS[$table]}"
    local abs_pct
    abs_pct=$(awk "BEGIN { v = $delta; if (v < 0) v = -v; if ($c_count > 0) printf \"%.1f\", (v / $c_count) * 100; else print 0 }")

    # Only fail if plugin MISSES data (negative delta exceeds threshold)
    if [ "$c_count" -gt 0 ]; then
      local neg_pct
      neg_pct=$(awk "BEGIN { if ($delta < 0) printf \"%.1f\", (-$delta / $c_count) * 100; else print 0 }")
      if awk "BEGIN { exit !($neg_pct > $threshold) }"; then
        status="FAIL"
        all_pass=false
      fi
    fi

    local delta_str
    if [ "$delta" -ge 0 ]; then
      delta_str="+${delta} (${pct}%)"
    else
      delta_str="${delta} (${pct}%)"
    fi

    printf "  %-20s %8d %10d %14s %6s\n" "$table" "$p_count" "$c_count" "$delta_str" "$status"
  done

  echo ""
  printf "  Files processed: plugin=%d compiled=%d\n" "$plugin_files" "$compiled_files"
  echo ""

  if $all_pass; then
    echo "  ✓ RESULT: PASS — all thresholds met"
  else
    echo "  ✗ RESULT: FAIL — one or more thresholds exceeded"
  fi
  echo ""

  $all_pass
}

# ─── Main ─────────────────────────────────────────────────────────────────

if [ $# -lt 1 ]; then
  usage
fi

grammar="$1"
overall_pass=true

if [ "$grammar" = "all" ]; then
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║  Grammar Plugin Comparison — All 7 Grammars                    ║"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  for g in go c cpp python javascript solidity typescript; do
    repo_path="${REPOS_BASE}/${DEFAULT_REPOS[$g]}"
    if run_comparison "$g" "$repo_path"; then
      : # pass
    else
      overall_pass=false
    fi
  done

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  if $overall_pass; then
    echo "  ✓ ALL 7 GRAMMARS PASS"
  else
    echo "  ✗ SOME GRAMMARS FAILED — see above"
  fi
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  $overall_pass
else
  if [ -z "${DEFAULT_REPOS[$grammar]+x}" ]; then
    echo "ERROR: Unknown grammar '$grammar'"
    usage
  fi

  repo_path="${2:-${REPOS_BASE}/${DEFAULT_REPOS[$grammar]}}"
  run_comparison "$grammar" "$repo_path"
fi

# Cleanup backup dir
rmdir "$BACKUP_DIR" 2>/dev/null || true
