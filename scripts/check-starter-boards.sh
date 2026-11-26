#!/usr/bin/env bash
# Checks that every starter board in crates/ceevee-search/src/starter.rs still answers.
# Prints one line per board and exits 1 if any fail. Needs network access.
set -u
f=crates/ceevee-search/src/starter.rs
bad=0
while read -r kind token; do
  case "$kind" in
    G) url="https://boards-api.greenhouse.io/v1/boards/$token/jobs" ;;
    L) url="https://api.lever.co/v0/postings/$token?mode=json" ;;
    *) continue ;;
  esac
  code=$(curl -s -o /dev/null -m 20 -w '%{http_code}' "$url")
  echo "$kind $token $code"
  [ "$code" = 200 ] || bad=1
done < <(grep -E '^\s+\((G|L), "' "$f" | sed -E 's/^\s+\((G|L), "([^"]+)".*/\1 \2/')
exit $bad
