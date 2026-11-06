```json
{ "imports_sort": true }
```

```tyrus
import { readFileSync } from "tyrus:fs";

import { add } from "./math";
import { formatDate } from "./utils";
```

With `true`, `tyrus:*` imports come first, followed by code imports sorted alphabetically.

```json
{ "imports_sort": false }
```

```tyrus
import { add } from "./math";
import { readFileSync } from "tyrus:fs";
import { formatDate } from "./utils";
```

With `false`, the original import order from the source file is preserved.
