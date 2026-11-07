#!/usr/bin/env bash
# Fetches the last successful run's measurements into previous/.
#
# No earlier run, or an expired artifact, counts as a first run. Failing there
# would keep the job red for good, because a failed run never becomes the last
# success. Anything else fails the step, including a download that holds no
# metrics.json, which would otherwise look like a first run.
set -euo pipefail

last=$(gh run list --workflow bench.yml --branch "$GITHUB_REF_NAME" --status success --limit 1 --json databaseId --jq '.[0].databaseId')
if [ -z "$last" ]; then
  echo "No earlier successful run, so this counts as a first run."
else
  artifact=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/$last/artifacts" --jq '.artifacts[] | select(.name == "bench-results" and (.expired | not)) | .name')
  if [ -z "$artifact" ]; then
    echo "::warning::Run $last no longer has its bench-results artifact, so this counts as a first run."
  else
    gh run download "$last" --name bench-results --dir previous
    if [ ! -f previous/metrics.json ]; then
      echo "::error::Run $last's bench-results artifact has no metrics.json."
      exit 1
    fi
  fi
fi
