#!/usr/bin/env bash
set -euo pipefail

dist_dir="${1:-dist}"
checksum_name="SHA256SUMS.txt"
checksum_path="${dist_dir}/${checksum_name}"
asset_list="$(mktemp)"

cleanup() {
  rm -f "${asset_list}"
}
trap cleanup EXIT

hash_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1"
  else
    shasum -a 256 "$1"
  fi
}

check_sha256sums() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum --check "$1"
  else
    shasum -a 256 -c "$1"
  fi
}

if [ ! -d "${dist_dir}" ]; then
  echo "Release asset directory does not exist: ${dist_dir}" >&2
  exit 1
fi

rm -f "${checksum_path}"

find "${dist_dir}" -maxdepth 1 -type f ! -name "${checksum_name}" -exec basename {} \; \
  | LC_ALL=C sort > "${asset_list}"

asset_count="$(wc -l < "${asset_list}" | tr -d '[:space:]')"

if [ "${asset_count}" -eq 0 ]; then
  echo "No release assets found for checksum generation." >&2
  exit 1
fi

(
  cd "${dist_dir}"

  while IFS= read -r asset; do
    hash_sha256 "${asset}"
  done < "${asset_list}" > "${checksum_name}"

  checksum_count="$(
    wc -l < "${checksum_name}" \
      | tr -d '[:space:]'
  )"

  if [ "${checksum_count}" -ne "${asset_count}" ]; then
    echo "Checksum count (${checksum_count}) does not match asset count (${asset_count})." >&2
    exit 1
  fi

  check_sha256sums "${checksum_name}"
)

echo "Generated ${checksum_path} for ${asset_count} release assets."
