#!/bin/bash
set -e

# Get the bumped version from git-cliff
VERSION=$(git cliff --bumped-version 2>/dev/null)

if [ -z "$VERSION" ]; then
    echo "Could not determine next version"
    exit 1
fi

echo "Releasing version: $VERSION"

# Update Cargo.toml version
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# Generate changelog
git cliff --bump -o CHANGELOG.md

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release v$VERSION"
git tag "v$VERSION"

echo "Released v$VERSION"
echo "Run 'git push && git push --tags' to publish"
