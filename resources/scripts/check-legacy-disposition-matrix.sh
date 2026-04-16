#!/usr/bin/env bash
set -euo pipefail

MATRIX="layer/surface/build/refactor/legacy-and-grammar-disposition/MATRIX.md"

if [[ ! -s "$MATRIX" ]]; then
  echo "error: missing or empty $MATRIX"
  exit 1
fi

service_children="belief-verifier session-writer spec-manager doctor"
grammar_children="grammar-c grammar-cairo grammar-cpp grammar-go grammar-javascript grammar-python grammar-rust grammar-solidity grammar-typescript"

service_count=0
for child in $service_children; do
  if ! grep -Eq "^\| ${child} \|.*\| (KEEP \(bounded\)|MIGRATE \(phased\)|RETIRE/REPLACE) \|" "$MATRIX"; then
    echo "error: service child '${child}' missing disposition decision row"
    exit 1
  fi
  service_count=$((service_count + 1))
done

grammar_count=0
for child in $grammar_children; do
  if ! grep -Eq "^\| ${child} \|.*\| KEEP \(bounded, containment\) \|" "$MATRIX"; then
    echo "error: grammar child '${child}' missing containment decision row"
    exit 1
  fi
  grammar_count=$((grammar_count + 1))
done

if grep -Eiq "\b(TBD|TODO|implicit carryover)\b" "$MATRIX"; then
  echo "error: unresolved placeholder wording found in $MATRIX"
  exit 1
fi

if ! grep -Eq "^\| belief-verifier \|.*\| MIGRATE \(phased\) \| [^|]+ \| [^|]+ \| [^|]+ \|" "$MATRIX"; then
  echo "error: belief-verifier migrate row missing owner/target/dependency fields"
  exit 1
fi
if ! grep -Eq "^\| spec-manager \|.*\| MIGRATE \(phased\) \| [^|]+ \| [^|]+ \| [^|]+ \|" "$MATRIX"; then
  echo "error: spec-manager migrate row missing owner/target/dependency fields"
  exit 1
fi
if ! grep -Eq "^\| doctor \|.*\| RETIRE/REPLACE \| [^|]+ \| [^|]+ \| [^|]+ \|" "$MATRIX"; then
  echo "error: doctor retire row missing owner/target/dependency fields"
  exit 1
fi

if ! grep -Fq "Spec-manager explicit path (lgd5)" "$MATRIX"; then
  echo "error: spec-manager explicit path section missing"
  exit 1
fi

echo "ok: disposition matrix covers ${service_count} service children and ${grammar_count} grammar children with explicit decisions"
