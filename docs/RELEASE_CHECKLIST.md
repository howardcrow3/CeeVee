# First customer release: checklist

What the code does is in `.github/workflows/release.yml`; the one-time account setup is in `docs/RELEASE_SETUP.md`. This page is the order to do things in.

## Needs Howard (accounts and content)

- [ ] Environments and secrets from `RELEASE_SETUP.md` (Apple, updater key, later Azure). Until Azure is set up the Windows installer is unsigned and SmartScreen will warn.
- [ ] AWS: `AWS_ROLE_ARN`, `AWS_REGION`, `DOWNLOAD_BUCKET` variables, and `dl.turtlebox.studio` pointing at the bucket (CloudFront). Without them a release is only a draft on GitHub and installed apps cannot update.
- [ ] Model files hosted on `dl.turtlebox.studio` (Q10) and `crates/ceevee-ai/catalog.json` updated with those URLs and hashes. Until then the app downloads from Hugging Face.
- [ ] Website pages: `turtlebox.studio/ceevee`, `/buy`, `/license`, `/privacy`, `/terms`, `/help` (text in `docs/customer/`). Terms need legal review.
- [ ] License backend live in highturtle (`docs/LICENSE_API.md`) and FastSpring connected.
- [ ] Windows and Mac test machines for the pass below.

## Cut the release

1. Put the release notes in `CHANGELOG.md` under `## X.Y.Z`.
2. `node scripts/bump-version.mjs X.Y.Z`, open a PR, merge it.
3. Tag the merge commit `vX.Y.Z-beta.1` first. The workflow builds, signs, checks signatures and publishes to Beta. Install it on both test machines.
4. Tag `vX.Y.Z` for Stable. The publish job waits for your approval in the `release` environment.
5. Publish the draft GitHub release. The website's download buttons point at `dl.turtlebox.studio/ceevee/latest/CeeVee-setup.exe` and `CeeVee.dmg`.

## Test pass on a clean machine (Windows 11 and Apple silicon Mac)

- [ ] Install from the downloaded file. No security warning on a signed build.
- [ ] First start: trial begins, the model downloads and the Settings test question is answered.
- [ ] Paste a resume on the Welcome page; profile is saved.
- [ ] Run a search with a few companies; jobs appear.
- [ ] Buy with a FastSpring test order; click the emailed `ceevee://activate` link; the license shows as active. Remove from this computer frees the seat.
- [ ] Close the window: CeeVee stays in the tray or menu bar. Quit from there.
- [ ] Install an older beta, publish a newer one, and check that "Check for updates" finds and installs it.
- [ ] Uninstall removes the app; the data folder stays unless deleted.
