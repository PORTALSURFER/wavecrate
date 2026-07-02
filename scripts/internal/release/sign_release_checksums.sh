#!/usr/bin/env bash
set -euo pipefail

CHECKSUM_FILE=""
SIGNATURE_FILE=""
EXPECTED_PUBKEY=""

usage() {
  cat <<'USAGE'
Usage: sign_release_checksums.sh --checksum-file <path> --signature-file <path> [--verify-public-key <base64-ed25519-pubkey>]

Requires CHECKSUMS_SIGNING_KEY to contain the Ed25519 private key PEM.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --checksum-file)
      CHECKSUM_FILE="${2:-}"
      shift 2
      ;;
    --signature-file)
      SIGNATURE_FILE="${2:-}"
      shift 2
      ;;
    --verify-public-key)
      EXPECTED_PUBKEY="${2:-}"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$CHECKSUM_FILE" || -z "$SIGNATURE_FILE" ]]; then
  echo "Missing --checksum-file or --signature-file" >&2
  usage >&2
  exit 1
fi
if [[ -z "${CHECKSUMS_SIGNING_KEY:-}" ]]; then
  echo "Missing CHECKSUMS_SIGNING_KEY." >&2
  exit 1
fi
if [[ ! -s "$CHECKSUM_FILE" ]]; then
  echo "Checksum file is missing or empty: $CHECKSUM_FILE" >&2
  exit 1
fi

work_dir="$(dirname "$SIGNATURE_FILE")"
mkdir -p "$work_dir"
key_path="$(mktemp "${work_dir}/checksums-signing-key.XXXXXX.pem")"
sig_bin="$(mktemp "${work_dir}/checksums-signature.XXXXXX.bin")"
pub_der="$(mktemp "${work_dir}/checksums-signing-key.XXXXXX.pub.der")"
cleanup() {
  rm -f "$key_path" "$sig_bin" "$pub_der"
}
trap cleanup EXIT

printf "%s" "$CHECKSUMS_SIGNING_KEY" > "$key_path"
openssl pkeyutl -sign -inkey "$key_path" -rawin \
  -in "$CHECKSUM_FILE" \
  -out "$sig_bin"
openssl base64 -A -in "$sig_bin" -out "$SIGNATURE_FILE"

if [[ -n "$EXPECTED_PUBKEY" ]]; then
  openssl pkey -in "$key_path" -pubout -outform DER -out "$pub_der"
  actual_pubkey="$(tail -c 32 "$pub_der" | openssl base64 -A)"
  if [[ "$actual_pubkey" != "$EXPECTED_PUBKEY" ]]; then
    echo "Public key mismatch: $actual_pubkey" >&2
    exit 1
  fi
  openssl pkeyutl -verify -pubin -inkey "$pub_der" -rawin \
    -in "$CHECKSUM_FILE" \
    -sigfile "$sig_bin"
fi
