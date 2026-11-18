# Trying CeeVee before release (Mac)

## Get the build
GitHub, Actions tab, "Test build", open the newest green run, download **CeeVee-test-macos** at the bottom of the page (a zip with the .dmg inside). To make a fresh one: Actions, Test build, "Run workflow".

## Open it past Gatekeeper
The test build is not signed or notarized, so macOS blocks it the first time.
1. Open the .dmg and drag CeeVee to Applications.
2. In Applications, right-click CeeVee, choose Open, then Open again in the dialog. Or, in Terminal: `xattr -dr com.apple.quarantine /Applications/CeeVee.app`
3. If it says "damaged", the xattr command above fixes it.

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
