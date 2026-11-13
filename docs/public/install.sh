#!/usr/bin/env sh
set -eu

REPOSITORY="tyrus-lang/tyrus"

fail() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

command -v curl >/dev/null 2>&1 || fail 'curl is required.'
command -v tar >/dev/null 2>&1 || fail 'tar is required.'

case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64) suffix="linux-x86_64" ;;
    Darwin:arm64|Darwin:aarch64) suffix="macos-aarch64" ;;
    *) fail "Unsupported platform: $(uname -s) $(uname -m). See https://tyruslang.org/docs/getting-started." ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
    verify_checksum() { sha256sum --check "$1"; }
elif command -v shasum >/dev/null 2>&1; then
    verify_checksum() { shasum -a 256 --check "$1"; }
else
    fail 'sha256sum or shasum is required to verify the release package.'
fi

tag="$(curl -fsSL -H 'Accept: application/vnd.github+json' "https://api.github.com/repos/$REPOSITORY/releases/latest" | sed -n 's/^[[:space:]]*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
[ -n "$tag" ] || fail 'Could not determine the latest Tyrus release.'
printf '%s\n' "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$' || fail 'GitHub returned an invalid Tyrus release tag.'
printf 'Installing Tyrus %s. Please wait...\n' "$tag"
archive="tyrus-$tag-$suffix.tar.gz"
release_url="https://github.com/$REPOSITORY/releases/download/$tag"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT HUP INT TERM

printf 'Downloading %s...\n' "$archive"
curl -fsSL --retry 3 "$release_url/$archive" -o "$temporary/$archive"
printf 'Downloading checksum...\n'
curl -fsSL --retry 3 "$release_url/$archive.sha256" -o "$temporary/$archive.sha256"
printf 'Verifying release package checksum...\n'
(cd "$temporary" && verify_checksum "$archive.sha256" >/dev/null) || fail 'Release package checksum verification failed.'
printf 'Checksum verified.\n'

install_root="${XDG_DATA_HOME:-$HOME/.local/share}/tyrus"
bin_dir="${HOME}/.local/bin"
mkdir -p "$install_root" "$bin_dir"
rm -rf "$install_root/$tag"
printf 'Extracting and installing Tyrus %s...\n' "$tag"
tar -xzf "$temporary/$archive" -C "$install_root"
ln -sfn "$install_root/tyrus-$tag-$suffix/bin/tyrus" "$bin_dir/tyrus"

printf 'Tyrus %s installed successfully.\n' "$tag"
case ":${PATH}:" in
    *":$bin_dir:"*) ;;
    *) printf 'Add %s to PATH to run tyrus.\n' "$bin_dir" ;;
esac
