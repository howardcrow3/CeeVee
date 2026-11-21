# Trying CeeVee before release (Mac)

## Get the build
GitHub, Actions tab, "Test build", open the newest green run, download **CeeVee-test-macos** at the bottom of the page (a zip with the .dmg inside). To make a fresh one: Actions, Test build, "Run workflow".

## Install and open it
The test build is ad-hoc signed (its seal is valid) but not signed by an Apple developer or notarized, so macOS asks for one extra approval the first time.
1. Download the zip from Actions, double-click it to get the .dmg, open the .dmg, and drag CeeVee onto Applications.
2. In Applications, right-click CeeVee, choose Open, then Open in the dialog.
3. If macOS instead says "CeeVee can't be opened because Apple cannot check it for malicious software": System Settings, Privacy & Security, scroll down, "Open Anyway" next to CeeVee.
4. If it says "CeeVee is damaged and can't be opened. You should move it to the Trash": do not trash it. Run this in Terminal, then open it again:
   `xattr -dr com.apple.quarantine /Applications/CeeVee.app`
   (Builds made before this fix hit this error every time. Current builds are checked in CI with `codesign --verify --deep --strict` and a launch test, so tell us if you still see it.)

## Keychain prompt
On first start macOS asks whether CeeVee may use the keychain ("CeeVee wants to use your confidential information stored in ..."). Enter your Mac login password and choose **Always Allow**. This is expected on test builds. CeeVee keeps the trial start date and the Google sign-in token there, under the service name `studio.turtlebox.ceevee` (visible in Keychain Access). Because test builds are ad-hoc signed, every new build looks like a different app to macOS, so you will be asked again after installing a newer build. Notarized release builds keep one identity and ask once. Choosing Deny is safe: CeeVee still runs, but the trial date is not anchored and Google sign-in will not stay signed in.

## Local AI model
CeeVee downloads its model itself from Hugging Face (about 2.5 GB for 8 GB Macs, 5 GB for 16 GB or more). Settings, Local AI, "Download". It resumes if interrupted. Nothing needs to be set up by hand.

## What to try first
1. Start CeeVee. Does the window open, and is there a menu bar icon?
2. Settings, Local AI: download the model, then ask it something.
3. Welcome page (from Today): paste a resume, keep the changes it finds.
4. Settings: add a job category, and "Add suggested companies". Jobs, "Search". Do jobs appear?
5. Close the window: CeeVee should stay in the menu bar. Quit from there.
6. Settings, License: it should say trial with days left.
7. Settings, About: version shows, "Open log folder" works.
8. Note anything confusing or broken, with what you clicked.

Data lives in `~/Library/Application Support/studio.turtlebox.ceevee`. Delete that folder to start fresh.

## Telling builds apart

Test builds (`test-build.yml`, `signed-mac-build.yml`) number themselves by the count of commits on the branch, so the number only goes up. The number and short commit appear in the download name (`CeeVee_0.1.0_build57_abc1234_aarch64.dmg`), the artifact name, Settings > About, and crash reports. Tagged releases show just the version. The same commit built twice gets the same number, because it is the same code.

Both workflows compile in the license public key (`CEEVEE_LICENSE_PUBKEY`). A build without it says "License checks are not available in this build" and cannot activate a key.

## What CI checks on every Test build
`scripts/verify-mac-dmg.sh` runs on the macOS runner before the artifact is uploaded. It mounts the .dmg, copies the app out with the download quarantine flag set, runs `codesign --verify --deep --strict`, checks the bundle is sealed and arm64, then launches the app and fails if it exits within 12 seconds. The Release workflow also verifies the ad-hoc seal when no Apple certificate is configured.
