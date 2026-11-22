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
| `APPLE_ID` | The Apple ID email of your developer account (same one Backer uses) |
| `APPLE_PASSWORD` | An app-specific password for that Apple ID, made at appleid.apple.com -> Sign-In and Security -> App-Specific Passwords (same one Backer uses). Not your normal Apple ID password |
| `APPLE_TEAM_ID` | Your 10-character team ID, `X68DUV4W5M` |

This is the same scheme Backer uses, so you can reuse Backer's values for all five Apple secrets. No signing-identity secret is needed: the build finds the identity in the imported certificate. Because the `.p12` may only hold an older certificate for the same key, the public Developer ID certificate is kept in the repo at `.github/signing/developer-id.cer` and imported with it.

The certificate must be a Developer ID Application certificate (for apps outside the App Store). Notarization uses the Apple ID, app-specific password and team ID.

## 3. Updater signing key (add to `release-build`)

`TAURI_SIGNING_PRIVATE_KEY`: the full contents of the private key that pairs with `backer-updater.key.pub`. `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password (if it has one). The release build passes them to Tauri, which signs the update files with them; the build fails if the key is set but no signatures come out.

## 4. Azure Trusted Signing (when your account is approved)

Secrets in `release-build`: `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET` (an app registration with the role "Trusted Signing Certificate Profile Signer" on your certificate profile).
Variables (Environments -> Variables, not secrets): `AZURE_SIGNING_ENDPOINT` (for example `https://eus.codesigning.azure.net`), `AZURE_SIGNING_ACCOUNT`, `AZURE_SIGNING_PROFILE`.
Until `AZURE_CLIENT_ID` is set, the build warns and produces an unsigned Windows installer.

## 5. AWS (later, with the download site)

Variables in `release-beta` and `release`: `AWS_ROLE_ARN`, `AWS_REGION`, `DOWNLOAD_BUCKET`. Until `AWS_ROLE_ARN` is set, files are only attached to a draft GitHub Release.

## 6. Check it

Run the Release workflow on a test tag (for example `v0.0.1-beta.1`) and read the first lines of each build job: they print "Windows signing: on" and "macOS signing: on" when the secrets are found.

## 7. Google sign-in (email tracking)

CeeVee can read recruiter replies in Gmail after a Google sign-in. It asks only for `gmail.readonly` (plus `openid`, `email`, `profile`). Without this setup the "Sign in with Google" button is hidden and people connect any mailbox with an app password instead.

1. Go to console.cloud.google.com, create a project (for example "CeeVee"), and enable the **Gmail API**.
2. **OAuth consent screen** (Google Auth Platform): user type **External**, app name CeeVee, support email, your privacy policy URL, authorized domain `turtlebox.studio`. Add the scopes `openid`, `email`, `profile` and `https://www.googleapis.com/auth/gmail.readonly`.
3. While the app is in **Testing**, add up to 100 **test users** (their Gmail addresses). Only they can sign in. Refresh tokens for Testing apps expire after 7 days, so testers will have to sign in again weekly until the app is verified.
4. **Credentials** -> Create credentials -> OAuth client ID -> **Desktop app**. Copy the client ID and client secret (for installed apps the secret is not confidential).
5. In GitHub, `release-build` environment, add secrets `CEEVEE_GOOGLE_CLIENT_ID` and `CEEVEE_GOOGLE_CLIENT_SECRET`. The release workflow passes them to the build; the build log shows "Google sign-in: on" when both are set. For local testing, export the same two variables before `cargo run`.
6. **Before public release:** click Publish app on the consent screen and submit it for verification. Because Gmail read access is a restricted scope, Google also requires a security assessment (CASA) by an approved assessor each year. Expect weeks to months and a fee. Prepare: a public privacy policy that says what CeeVee reads and that it stays on the computer, a short demo video of the sign-in and of what the app does with mail, and a written justification for `gmail.readonly` ("read replies from employers to update job application status").

## Analytics (GA4)

Usage analytics are opt-in in the app and send nothing unless the build has both values. Create a GA4 property and a web data stream, then a Measurement Protocol API secret (stream details, Events, Measurement Protocol API secrets). In the `release-build` environment add the variable `CEEVEE_GA4_MEASUREMENT_ID` (the `G-...` ID) and the secret `CEEVEE_GA4_API_SECRET`. The build log shows "Analytics: on (opt-in)" when both are set. Set event data retention to 2 months and leave Google signals off.
