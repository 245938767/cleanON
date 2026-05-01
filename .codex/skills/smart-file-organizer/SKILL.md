---
name: smart-file-organizer
description: Project workflow for implementing the intelligent file and desktop organizer with strict plan-first safety boundaries.
---

# Smart File Organizer Workflow

Use this workflow for all project work in this repository.

## Operating Rule

Every feature must preserve the boundary:

```text
scan -> classify -> generate plan -> user review -> confirmed execute -> rollback / learn skill
```

No module before `executor` may mutate user files.

## Implementation Order

1. Update `TASKS.md` before starting a scoped slice.
2. Use `crates/core` types instead of inventing duplicate local shapes.
3. Keep side effects behind traits so tests can use temp directories and mocks.
4. Add focused tests for every safety-sensitive branch.
5. Update `TASKS.md` with completion evidence.

## UI Rules

- Chinese-first copy for MVP.
- Home screen has two icon modules: `文件整理` and `桌面整理`.
- No visible text buttons on the home screen.
- Secondary pages may use normal controls for review and confirmation.
- Never imply automatic cleanup or automatic file movement.

## Safety Checklist

- Does this code move, rename, create, or delete files?
- If yes, is it only in executor after confirmation?
- Does it write rollback data before or during execution?
- Does it avoid sensitive directories by default?
- Does it avoid storing API keys or raw file content?
- Can the behavior be tested in a temp directory?
