#!/bin/bash
set -e

# Get the bumped version from git-cliff
VERSION=$(git cliff --bumped-version 2>/dev/null)

if [ -z "$VERSION" ]; then
    echo "Could not determine next version"
    exit 1
fi

echo "Releasing version: $VERSION"

# Strip 'v' prefix for Cargo.toml (uses bare version)
BARE_VERSION="${VERSION#v}"

# Update Cargo.toml version
sed -i '' "s/^version = \".*\"/version = \"$BARE_VERSION\"/" Cargo.toml

# Generate changelog
git cliff --bump -o CHANGELOG.md

# Commit and tag
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release $VERSION"
git tag "$VERSION"

echo "Released $VERSION"
echo "Run 'git push && git push --tags' to publish"
