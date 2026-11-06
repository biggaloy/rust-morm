#!/usr/bin/env bash
# Fetches the last successful run's measurements into previous/.
#
# No earlier run, or an expired artifact, counts as a first run. Failing there
# would keep the job red for good, because a failed run never becomes the last
# success. Any other failure fails the step.
set -euo pipefail

last=$(gh run list --workflow bench.yml --branch "$GITHUB_REF_NAME" --status success --limit 1 --json databaseId --jq '.[0].databaseId')
if [ -z "$last" ]; then
  echo "No earlier successful run, so this counts as a first run."
  exit 0
fi

artifact=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/$last/artifacts" --jq '.artifacts[] | select(.name == "bench-results" and (.expired | not)) | .name')
if [ -z "$artifact" ]; then
  echo "::warning::Run $last no longer has its bench-results artifact, so this counts as a first run."
  exit 0
fi

gh run download "$last" --name bench-results --dir previous
