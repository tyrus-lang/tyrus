#!/usr/bin/env bash
# Install the generated Tyrus syntax definition for a native Linux Kate setup.
# Set KATE_SYNTAX_DIR to install into a Flatpak, Snap, or custom Kate data path.

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source_file="${script_dir}/tyrus.xml"
default_data_dir="${XDG_DATA_HOME:-${HOME}/.local/share}"
target_dir="${KATE_SYNTAX_DIR:-${default_data_dir}/org.kde.syntax-highlighting/syntax}"

if [[ ! -f "${source_file}" ]]; then
    echo "error: generated Kate syntax definition not found: ${source_file}" >&2
    exit 1
fi

mkdir -p "${target_dir}"
install -m 0644 "${source_file}" "${target_dir}/tyrus.xml"
echo "Installed Kate syntax definition: ${target_dir}/tyrus.xml"
echo "Restart Kate or refresh syntax highlighting in an open .tyr file."
