# macOS Nightly Signing

Wavecrate nightly releases should be unattended after the Apple credentials are
stored in GitHub repository secrets. The nightly workflow builds the app from
`main`, packages macOS artifacts as `Wavecrate.app`, signs with Developer ID,
submits to Apple notarization, staples the ticket, then publishes the same
notarized zip to GitHub and PortalSurfer.

## One-Time Apple Setup

1. Create a Developer ID Application certificate in the Apple Developer portal.
2. Install the certificate in Keychain Access on a trusted Mac.
3. Export the certificate and private key as a password-protected `.p12`.
4. Create an App Store Connect API key with access to notarization.
5. Download the `AuthKey_<key-id>.p8` file.

## GitHub Secrets

Add these repository secrets:

- `APPLE_DEVELOPER_ID_APPLICATION_CERT_BASE64`
- `APPLE_DEVELOPER_ID_APPLICATION_CERT_PASSWORD`
- `APPLE_NOTARY_KEY_BASE64`
- `APPLE_NOTARY_KEY_ID`
- `APPLE_NOTARY_ISSUER_ID`

`APPLE_CODESIGN_IDENTITY` is optional. Use it only if the imported keychain has
more than one Developer ID Application identity.

On macOS, copy the base64 secret values without line breaks:

```bash
base64 -i DeveloperIDApplication.p12 | tr -d '\n' | pbcopy
```

```bash
base64 -i AuthKey_XXXXXXXXXX.p8 | tr -d '\n' | pbcopy
```

## Nightly Behavior

The scheduled and manual nightly workflow use the same release path. If any
Apple secret is missing, the macOS build fails before upload with the missing
secret name. Windows artifacts are unaffected by Apple signing.

Local release packaging remains simple:

```bash
scripts/internal/release/build_release_zip.sh \
  --target aarch64-apple-darwin \
  --platform macos \
  --arch aarch64 \
  --channel nightly \
  --version "0.19.1-nightly.$(date -u +%Y%m%d)+$(git rev-parse --short=8 HEAD)" \
  --target-version 0.19.1 \
  --build-number 1 \
  --git-sha "$(git rev-parse HEAD)" \
  --build-date "$(date -u +%Y-%m-%d)"
```

Set `WAVECRATE_MACOS_SIGNING=1` only when intentionally testing the full
sign-and-notarize flow locally.
