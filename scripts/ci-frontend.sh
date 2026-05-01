#!/usr/bin/env bash
set -euo pipefail

export FILE_ORGANIZER_AI_MODE="${FILE_ORGANIZER_AI_MODE:-mock}"
export FILE_ORGANIZER_DISABLE_CLOUD_AI="${FILE_ORGANIZER_DISABLE_CLOUD_AI:-1}"
export FILE_ORGANIZER_TEST_TEMP_ONLY="${FILE_ORGANIZER_TEST_TEMP_ONLY:-1}"

if [ ! -f "apps/desktop/package.json" ]; then
  echo "Skipping frontend QA: apps/desktop/package.json is not present yet."
  exit 0
fi

echo "Frontend QA uses mock AI and temp-only file fixtures."

pnpm --filter desktop lint
pnpm --filter desktop test
pnpm --filter desktop build
