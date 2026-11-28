# Trying CeeVee before release (Mac)

## Get the build
GitHub, Actions tab, "Test build", open the newest green run, download **CeeVee-test-macos** at the bottom of the page (a zip with the .dmg inside). To make a fresh one: Actions, Test build, "Run workflow".

## Install and open it
Test builds are signed with CeeVee's Developer ID certificate and notarized by Apple, the same as a release. They install like any Mac app:
1. Download the zip from Actions, double-click it to get the .dmg, open the .dmg, and drag CeeVee onto Applications.
2. Double-click CeeVee in Applications. That is all: no right-click Open, no Terminal, no "Open Anyway".

Test builds only run from `main` (the signing secrets live in the `release-build` environment, which allows only main): Actions, Test build, "Run workflow", branch main.

Every Test build is checked in CI before you get it (`scripts/verify-mac-dmg.sh --require-notarized`): Developer ID signature, hardened runtime, stapled notarization ticket, Gatekeeper `spctl --assess`, and a launch test. If any check fails the run goes red and no download is produced. Pull request builds use an ad-hoc signature only to check the bundle seals and launches; you are not meant to install those.

If macOS ever says "damaged" or asks you to right-click Open on a Test build, that is a bug: tell us the build number from the file name.

## Keychain
CeeVee keeps the trial start date and the Google sign-in token in the macOS Keychain under the service name `studio.turtlebox.ceevee`. Because every build is signed by the same Developer ID certificate, macOS recognizes each new build as the same app and does not ask for access again.

One-time cleanup if you tried the older ad-hoc builds: those built entries that macOS ties to the old unsigned app, so the first signed build may ask once. Choose **Always Allow**, or open Keychain Access, search `studio.turtlebox.ceevee`, delete those entries, and sign in to Google again. After that, no prompts.

## Google sign-in
Settings, Email, "Sign in with Google" opens your browser. When it says CeeVee is connected, Settings shows "Signed in with Google as <address>". If you close the browser tab first, press **Cancel** next to "Waiting for Google" and start again (it also gives up by itself after 5 minutes). To sign out, press **Sign out of Google** next to the signed-in line: CeeVee deletes its saved token and mail, and tells Google to revoke its access.

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

Test builds (`test-build.yml`) number themselves by the count of commits on the branch, so the number only goes up. The number and short commit appear in the download name (`CeeVee_0.1.0_build57_abc1234_aarch64.dmg`), the artifact name, Settings > About, and crash reports. Tagged releases show just the version. The same commit built twice gets the same number, because it is the same code.

Both workflows compile in the license public key (`CEEVEE_LICENSE_PUBKEY`). A build without it says "License checks are not available in this build" and cannot activate a key.
