#!/bin/bash
# Enable all refactored modules for testing

echo "🔄 Enabling all refactored modules..."

export PATINA_USE_REFACTORED_INDEXER=1
export PATINA_USE_REFACTORED_WORKSPACE=1
export PATINA_USE_REFACTORED_INIT=1
export PATINA_USE_REFACTORED_CLAUDE=1
export PATINA_USE_REFACTORED_AGENT=1
export PATINA_USE_REFACTORED_DAGGER=1
export PATINA_USE_REFACTORED_NAVIGATE=1
export PATINA_USE_REFACTORED_HYBRID_DB=1

echo "✅ All refactored modules enabled!"
echo ""
echo "Environment variables set:"
echo "  PATINA_USE_REFACTORED_INDEXER=1"
echo "  PATINA_USE_REFACTORED_WORKSPACE=1"
echo "  PATINA_USE_REFACTORED_INIT=1"
echo "  PATINA_USE_REFACTORED_CLAUDE=1"
echo "  PATINA_USE_REFACTORED_AGENT=1"
echo "  PATINA_USE_REFACTORED_DAGGER=1"
echo "  PATINA_USE_REFACTORED_NAVIGATE=1"
echo "  PATINA_USE_REFACTORED_HYBRID_DB=1"
echo ""
echo "Run 'source enable_all_refactored.sh' to activate in current shell"