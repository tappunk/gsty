#!/bin/bash
set -euo pipefail

BIN_NAME="gsty"
TARGET_ARCH="macos-arm64"
RUST_TARGET="aarch64-apple-darwin"
TAP_REPO="tappunk/homebrew-gsty"

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
  DRY_RUN=true
  shift
fi

BUMP="${1:-patch}"
NOTES="${2:-}"

echo "[PROC] Verifying deployment dependencies..."
for tool in cargo gh shasum tar awk git perl; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[ERR] Required CLI tool '$tool' is missing."
    exit 1
  fi
done

if ! gh auth status >/dev/null 2>&1; then
  echo "[ERR] GitHub CLI is not authenticated. Run 'gh auth login'."
  exit 1
fi

if [[ ! "$BUMP" =~ ^(patch|minor|major)$ ]]; then
  echo "[ERR] Invalid bump type '$BUMP'. Use: patch, minor, or major"
  exit 1
fi

if [[ -n $(git ls-files --others --exclude-standard) ]]; then
  echo "[ERR] Untracked files found in workspace. Add or remove them before releasing."
  git ls-files --others --exclude-standard
  exit 1
fi

if [[ -n $(git status --porcelain) ]]; then
  echo "[ERR] Uncommitted changes detected. Stash or commit before releasing."
  exit 1
fi

if [[ $(git branch --show-current) != "main" ]]; then
  echo "[ERR] You must be on the 'main' branch to cut a release."
  exit 1
fi

git fetch origin
if [[ -n $(git log HEAD..origin/main --oneline) ]]; then
  echo "[ERR] Local 'main' is behind 'origin/main'. Pull latest changes first."
  exit 1
fi

echo "[PROC] Executing strict code quality gates..."
cargo fmt --all -- --check || {
  echo "[ERR] Code formatting violations found. Run 'cargo fmt'."
  exit 1
}
cargo clippy --all-targets -- -D warnings || {
  echo "[ERR] Clippy warnings detected. Fix them before releasing."
  exit 1
}
cargo test --all-targets || {
  echo "[ERR] Test suite execution failed."
  exit 1
}

cargo publish --dry-run || {
  echo "[ERR] Cargo publish dry run failed. Fix packaging before releasing."
  exit 1
}

CURRENT_VERSION=$(perl -ne 'print "$1\n" if /^version = "(.*)"/' Cargo.toml | awk 'NF { print; exit }')
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "[ERR] Could not read current version from Cargo.toml"
  exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<<"$CURRENT_VERSION"
case "$BUMP" in
patch) PATCH=$((PATCH + 1)) ;;
minor)
  MINOR=$((MINOR + 1))
  PATCH=0
  ;;
major)
  MAJOR=$((MAJOR + 1))
  MINOR=0
  PATCH=0
  ;;
esac
NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"

echo "Preparing Apple Silicon Release: v$CURRENT_VERSION -> v$NEW_VERSION ($BUMP)"
if $DRY_RUN; then
  echo "[INFO] Dry run complete. Code is pristine and ready for release."
  exit 0
fi

BACKUP_CARGO_TOML=""
rollback() {
  echo ""
  echo "[CRIT] Release pipeline interrupted!"
  if git rev-parse "v$NEW_VERSION" >/dev/null 2>&1; then
    git tag -d "v$NEW_VERSION"
  fi
  if [[ -n "$BACKUP_CARGO_TOML" && -f "$BACKUP_CARGO_TOML" ]]; then
    cp "$BACKUP_CARGO_TOML" Cargo.toml
  fi
  rm -f Cargo.toml.bak "$BACKUP_CARGO_TOML" 2>/dev/null || true
  if [[ -n "${DIST_DIR:-}" && -d "${DIST_DIR}" ]]; then
    rm -rf "${DIST_DIR}"
  fi
  if [[ -n "${STAGING_DIR:-}" && -d "${STAGING_DIR}" ]]; then
    rm -rf "${STAGING_DIR}"
  fi
  echo "[WARN] Rolled back. Re-run scripts/release.sh to try again."
}
trap rollback ERR

echo "[PROC] Updating versioning configuration..."
BACKUP_CARGO_TOML=$(mktemp)
cp Cargo.toml "$BACKUP_CARGO_TOML"
perl -0pi -e "s/^version = \"\Q$CURRENT_VERSION\E\"/version = \"$NEW_VERSION\"/m" Cargo.toml

cargo update -p "$BIN_NAME"

echo "[PROC] Compiling optimized release binary for Apple Silicon..."
cargo build --release --target "$RUST_TARGET"

echo "[PROC] Packaging distribution archives..."
ARCHIVE_BASENAME="${BIN_NAME}-${NEW_VERSION}-bin-${TARGET_ARCH}.tar.gz"
CHECKSUM_BASENAME="${ARCHIVE_BASENAME}.sha256"
DIST_DIR="$(mktemp -d)"
ARCHIVE_NAME="${DIST_DIR}/${ARCHIVE_BASENAME}"
CHECKSUM_NAME="${DIST_DIR}/${CHECKSUM_BASENAME}"
STAGING_DIR="$(mktemp -d)"

mkdir -p "${STAGING_DIR}/${BIN_NAME}"
cp "target/${RUST_TARGET}/release/${BIN_NAME}" "${STAGING_DIR}/${BIN_NAME}/"
cp README.md LICENSE "${STAGING_DIR}/${BIN_NAME}/" 2>/dev/null || true

tar -czf "$ARCHIVE_NAME" -C "$STAGING_DIR" "${BIN_NAME}"
shasum -a 256 "$ARCHIVE_NAME" >"$CHECKSUM_NAME"
rm -rf "$STAGING_DIR"
rm -f "$BACKUP_CARGO_TOML"
BACKUP_CARGO_TOML=""

echo "[PROC] Recording version changes to Git history..."
git add Cargo.toml Cargo.lock
git commit -m "chore: release v$NEW_VERSION [skip ci]"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

echo "[PROC] Publishing crate package to crates.io..."
cargo publish

trap - ERR

echo "[PROC] Synchronizing changes with remote origin..."
git push origin main
git push origin "v$NEW_VERSION"

echo "[PROC] Deploying GitHub Release and assets..."
if [[ -n "$NOTES" ]]; then
  gh release create "v$NEW_VERSION" "$ARCHIVE_NAME" "$CHECKSUM_NAME" \
    --title "v$NEW_VERSION" \
    --notes "$NOTES"
else
  gh release create "v$NEW_VERSION" "$ARCHIVE_NAME" "$CHECKSUM_NAME" \
    --title "v$NEW_VERSION" \
    --generate-notes
fi

echo "[PROC] Propagating release configuration to Homebrew tap..."
RAW_SHA=$(awk '{print $1}' "${CHECKSUM_NAME}")
TAP_DIR="$(mktemp -d)"

git clone --depth 1 "https://github.com/${TAP_REPO}.git" "$TAP_DIR"

mkdir -p "${TAP_DIR}/Formula"
cat <<EOF >"${TAP_DIR}/Formula/gsty.rb"
class Gsty < Formula
  desc "Ghostty live preview theme browser and installer TUI"
  homepage "https://github.com/tappunk/gsty"
  version "${NEW_VERSION}"

  depends_on arch: :arm64
  depends_on :macos

  url "https://github.com/tappunk/gsty/releases/download/v#{version}/gsty-#{version}-bin-macos-arm64.tar.gz"
  sha256 "${RAW_SHA}"

  def install
    bin.install "gsty"
  end

  test do
    output = shell_output("#{bin}/gsty --help")
    assert_match "Ghostty theme browser and installer", output
    assert_match "gsty --list", output
  end
end
EOF

(
  cd "$TAP_DIR"
  git add Formula/gsty.rb
  git commit -m "bump: gsty v${NEW_VERSION}"
  git push origin main
)
rm -rf "$TAP_DIR"

echo "[PROC] Cleaning up local packaging assets..."
rm -rf "$DIST_DIR"

echo "[ SUCCESS ] Release v$NEW_VERSION fully deployed!"
