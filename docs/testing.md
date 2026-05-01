# Testing and CI

This project keeps tests local-first and deterministic. Tests must not touch a real
Desktop, Downloads, Documents, cloud drive, or user home directory. File-operation
tests must create temporary directories and pass those paths explicitly through the
scanner, planner, executor, and rollback layers.

## CI Jobs

GitHub Actions runs on both `macos-latest` and `windows-latest`.

- Rust: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test --workspace`.
- Frontend: `pnpm --filter desktop lint`, `pnpm --filter desktop test`, and
  `pnpm --filter desktop build` once `apps/desktop/package.json` exists.

The workflow sets:

- `FILE_ORGANIZER_AI_MODE=mock`
- `FILE_ORGANIZER_DISABLE_CLOUD_AI=1`
- `FILE_ORGANIZER_TEST_TEMP_ONLY=1`

These variables are CI guardrails. Tests may use them to select mock AI providers
and reject fixtures that point outside temporary directories.

## Local Verification

Run the same commands locally:

```sh
bash scripts/ci-rust.sh
bash scripts/ci-frontend.sh
```

If the full frontend package is available, also run the root scripts:

```sh
pnpm lint
pnpm test
pnpm build
```

## Fixture Rules

Fixtures should be small metadata-only trees created at test runtime. Prefer
`tempfile` in Rust tests and the test runner's temporary directory helper in
frontend tests.

Allowed fixture shape:

```text
temp-root/
  inbox/
    report.pdf
    photo.jpg
    notes.txt
  expected/
    Documents/
    Images/
```

Do not commit real personal files, absolute user paths, API keys, raw document
contents, or copied Desktop state. When a test needs file contents, use synthetic
strings such as `fixture document body` and assert behavior through generated
`OrganizationPlan` values and rollback records.

## AI and File-System Boundaries

- AI, classifier, planner, and rule-engine tests should assert that they only
  produce `OrganizationPlan` data.
- Executor tests should require an explicitly confirmed plan and should assert a
  rollback record is written.
- Cloud AI tests must be opt-in and excluded from default CI.
- Windows desktop-coordinate behavior is preview-only for MVP tests.
- macOS tests must not assert pixel-perfect desktop icon coordinates.
