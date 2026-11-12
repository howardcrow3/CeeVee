# Release setup: GitHub environments and secrets

Click-by-click steps for the one-time setup of the release pipeline (`.github/workflows/release.yml`). Do the Apple part now. The Azure part waits for your Trusted Signing account. Nothing here goes in the repo: secrets live only in GitHub.

## 1. Create the environments

GitHub repo -> Settings -> Environments -> New environment. Create three:

| Environment | Holds | Protection |
|---|---|---|
| `release-build` | Signing secrets (build jobs) | Deployment branches and tags: only `main` and tags `v*` |
| `release-beta` | Beta publish, AWS role | None |
| `release` | Stable publish, AWS role | Required reviewers: add yourself. Tick "Prevent self-review" off (you are the only reviewer) |

## 2. Apple secrets (add to `release-build`)

Settings -> Environments -> `release-build` -> Add environment secret. Add each of these:

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | Your "Developer ID Application" certificate exported from Keychain Access as a .p12 file, then base64-encoded. On a Mac: `base64 -i cert.p12 \| pbcopy`, then paste |
| `APPLE_CERTIFICATE_PASSWORD` | The password you set when exporting the .p12 |
| `APPLE_SIGNING_IDENTITY` | The certificate's full name, for example `Developer ID Application: Your Name (TEAMID)`. Find it with `security find-identity -v -p codesigning` |
| `APPLE_API_ISSUER` | App Store Connect -> Users and Access -> Integrations -> App Store Connect API: the Issuer ID at the top |
| `APPLE_API_KEY` | The Key ID of an API key you create there (role: Developer) |
| `APPLE_API_KEY_CONTENT` | The full text of the downloaded `AuthKey_XXXX.p8` file (you can download it only once) |

The certificate must be a Developer ID Application certificate (for apps outside the App Store). Notarization uses the API key.

## 3. Updater signing key (add to `release-build`)

`TAURI_SIGNING_PRIVATE_KEY`: the full contents of the private key that pairs with `backer-updater.key.pub`. `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password (if it has one). Note: the workflow does not read these yet; a later PR wires them in together with the updater client.

## 4. Azure Trusted Signing (when your account is approved)

Secrets in `release-build`: `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET` (an app registration with the role "Trusted Signing Certificate Profile Signer" on your certificate profile).
Variables (Environments -> Variables, not secrets): `AZURE_SIGNING_ENDPOINT` (for example `https://eus.codesigning.azure.net`), `AZURE_SIGNING_ACCOUNT`, `AZURE_SIGNING_PROFILE`.
Until `AZURE_CLIENT_ID` is set, the build warns and produces an unsigned Windows installer.

## 5. AWS (later, with the download site)

Variables in `release-beta` and `release`: `AWS_ROLE_ARN`, `AWS_REGION`, `DOWNLOAD_BUCKET`. Until `AWS_ROLE_ARN` is set, files are only attached to a draft GitHub Release.

## 6. Check it

Run the Release workflow on a test tag (for example `v0.0.1-beta.1`) and read the first lines of each build job: they print "Windows signing: on" and "macOS signing: on" when the secrets are found.
