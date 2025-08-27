#!/bin/bash

echo "=== Testing patina-index database ==="
echo

echo "1. Database statistics:"
sqlite3 .patina/semantic.db <<EOF
SELECT 'Files:', COUNT(DISTINCT file) FROM functions;
SELECT 'Functions:', COUNT(*) FROM functions;
SELECT 'Types:', COUNT(*) FROM types;
SELECT 'Imports:', COUNT(*) FROM imports;
SELECT 'Calls:', COUNT(*) FROM calls;
EOF

echo
echo "2. Top 10 most called functions:"
sqlite3 .patina/semantic.db <<EOF
.mode column
.headers on
SELECT target, COUNT(*) as call_count 
FROM calls 
GROUP BY target 
ORDER BY call_count DESC 
LIMIT 10;
EOF

echo
echo "3. Find all parse-related functions:"
sqlite3 .patina/semantic.db <<EOF
.mode column
.headers on
SELECT file, name, line_start
FROM functions  
WHERE name LIKE '%parse%'
ORDER BY file, line_start
LIMIT 10;
EOF

echo
echo "4. Type definitions in pipeline modules:"
sqlite3 .patina/semantic.db <<EOF
.mode column
.headers on
SELECT file, name, kind
FROM types
WHERE file LIKE '%pipeline%'
ORDER BY file, line_start;
EOF