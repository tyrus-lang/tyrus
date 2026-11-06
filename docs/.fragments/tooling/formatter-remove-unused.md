```json
{ "remove_unused_imports": true }
```

```tyrus
import { add, subtract } from "./math";

const result = add(1, 2);
```

With `true`, the unused `subtract` specifier is removed.

```json
{ "remove_unused_imports": false }
```

```tyrus
import { add, subtract } from "./math";

const result = add(1, 2);
```

With `false`, unused imports are kept, only their layout is formatted.
