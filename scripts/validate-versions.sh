#!/bin/bash
set -e

echo "Validating version consistency..."

# TypeScript version
TS_VERSION=$(node -p "require('./packages/typescript/package.json').version")
echo "TypeScript: $TS_VERSION"

# Python version
PY_VERSION=$(python3 -c "import tomllib; print(tomllib.load(open('packages/python/pyproject.toml', 'rb'))['project']['version'])")
echo "Python: $PY_VERSION"

# Validate TypeScript version format
if [[ ! "$TS_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "ERROR: Invalid TypeScript version format: $TS_VERSION"
  exit 1
fi

# Validate Python version format
if [[ ! "$PY_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.a-zA-Z0-9]+)?$ ]]; then
  echo "ERROR: Invalid Python version format: $PY_VERSION"
  exit 1
fi

echo "All versions valid"