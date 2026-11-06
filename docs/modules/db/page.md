---
title: Page
description: Pagination result type exported by tyrus:db.
---

# Page

Import the types used in annotations from `tyrus:db`. In particular, import `Page` explicitly whenever a function signature uses the paginated result type.

<SignatureBlock label="Signature">interface Page&lt;T&gt; {<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly items: T[];<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly total: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly pageSize: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly page?: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly totalPages?: u64;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly nextCursor?: string;<br />&nbsp;&nbsp;&nbsp;&nbsp;readonly hasMore?: boolean;<br />}</SignatureBlock>

Numbered pagination returns `items`, `total`, `pageSize`, `page`, and `totalPages`. Cursor pagination returns `items`, `total`, `pageSize`, and `hasMore`, plus `nextCursor` when another page is available. Fields for the inactive pagination mode are omitted.

The page rows and `total` are read from the same transaction snapshot across relational backends, so concurrent writes cannot make the count and items describe different database states. MongoDB uses its generated `_id` for stable ordering; count and item reads are separate operations, so concurrent writes may affect consistency between them. Its cursor token is the serialized 24-character ObjectId of the last returned item; pass `nextCursor` unchanged to `after`.

## Numbered pagination example

Pass the page number with `paginate: true`. The first page is page 1; use the returned `totalPages` to determine whether more numbered pages are available.

<!-- @include: ../../.fragments/modules/db/page/numbered-example.md -->

## Cursor pagination example

Omit `after` to fetch the first page. Pass its `nextCursor` unchanged as `after` to fetch the next page. The cursor is the serialized value of the last returned row's `@id`, not an item count or page number. For an integer `@id`, `"25"` means continue after the row whose id is 25; `after: "10"` starts after id 10, not at the tenth result. Do not calculate the cursor from `pageSize`.

<!-- @include: ../../.fragments/modules/db/page/cursor-example.md -->
