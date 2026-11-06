---
title: watch
description: Watches a directory for changes.
---

# watch

Starts watching a file or directory for filesystem changes. Returns an `FSWatcher` handle that can be used to stop watching.

## FSWatcher

The `FSWatcher` type has a single method:

- `close(): void`, stops watching and releases all resources.

<SignatureBlock label="Signature">watch(path: string): Result&lt;FSWatcher, FsError&gt;</SignatureBlock>

## FSWatcherEvent

`FSWatcherEvent` describes an event record with readonly `kind: string` and `path: string` fields. The current `watch` API returns only an `FSWatcher` handle and does not expose an event subscription method.

## Example

<!-- @include: ../../.fragments/modules/fs/watch/example.md -->
