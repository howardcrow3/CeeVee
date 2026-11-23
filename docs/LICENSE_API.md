# License API (turtlebox.studio)

What the CeeVee app expects from the turtlebox.studio backend. The backend lives in the highturtle repo and is not touched from here. Payments are Paddle; the app links out to the purchase page and calls the three endpoints below. Base address: `https://turtlebox.studio/api/ceevee/v1`. All bodies are JSON.

## Signed tokens

Licenses and trial tokens share one envelope: `base64url(json) + "." + base64url(signature)`, no padding. The signature is Ed25519 over the raw JSON bytes. The app holds only the public key (build setting `CEEVEE_LICENSE_PUBKEY`, 64 hex characters); a build without it cannot unlock anything. `ceevee_license::token::sign` is the reference signer.

```json
{"kind":"license","key":"CEEV-7KQ2M-9XB4D-TR5VW-H3NZ8","machine":"<hash>","issued_at":"2026-12-05T00:00:00Z","major":1,"seats":2}
{"kind":"trial","machine":"<hash>","started_at":"2026-12-01T00:00:00Z"}
```

`machine` is the SHA-256 of the OS machine ID with an app salt; the raw ID never leaves the computer. `major` is the highest app major version the license covers.

## Endpoints

| Call | Body | Success | Failure |
| --- | --- | --- | --- |
| `POST /trial` | `{machine}` | `200 {token}` (a `trial` token; the server keeps the first `started_at` it ever saw for that machine) | any |
| `POST /activate` | `{key, machine, app_version, platform}` | `200 {license}` (a `license` token for that machine) | `409` or `{error:"seats_full"}` when 2 seats are used; `404` or `{error:"unknown_key"}`; other errors may carry `{message}`, which is shown to the person |
| `POST /deactivate` | `{key, machine}` | `204` | as above |

Key shape: `CEEV-` plus four groups of five characters from `0-9A-Z` without `I L O U`. The app forgives case, spaces and missing dashes and treats `O` as `0` and `I`/`L` as `1`.

## Pages the app opens

- `https://turtlebox.studio/ceevee/buy?from=app`: Paddle checkout. The Paddle fulfillment webhook creates the key; the email carries the key and an `ceevee://activate?key=…` link.
- `https://turtlebox.studio/ceevee/license`: "My license" (seats, free a seat, downloads).

## Client behavior

- First run records the trial start in the database and the OS keychain; the earliest start of database, keychain and server token wins, so wiping one place does not restart the trial.
- The newest time ever seen is kept in the database; the clock cannot move the trial backwards.
- After 14 days with no valid license the app is read-only (view and export).
- Activation checks the returned license against the key, machine and app major version before saving it. Deactivation asks the server first, so a failed call keeps the license.
- Every later check is offline.
