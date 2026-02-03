#!/bin/bash
# Helper script for creating PRs via GitHub API
# Usage: ./scripts/pr.sh <branch-name> <title> <body>

set -e

REPO="AustinEral/bosun"
TOKEN=$(cat /root/.openclaw/credentials/github.json | grep token | cut -d'"' -f4)

BRANCH=$1
TITLE=$2
BODY=$3

if [ -z "$BRANCH" ] || [ -z "$TITLE" ]; then
  echo "Usage: $0 <branch-name> <title> [body]"
  exit 1
fi

# Create PR
curl -s -X POST \
  -H "Authorization: token $TOKEN" \
  -H "Accept: application/vnd.github.v3+json" \
  "https://api.github.com/repos/$REPO/pulls" \
  -d "{
    \"title\": \"$TITLE\",
    \"head\": \"$BRANCH\",
    \"base\": \"main\",
    \"body\": \"$BODY\"
  }"
