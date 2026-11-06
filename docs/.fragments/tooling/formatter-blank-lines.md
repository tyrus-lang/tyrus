```json
{ "blank_lines": "collapse" }
```

```tyrus
import { readFileSync } from "tyrus:fs";

import { add } from "./math";

function main(): void {
    const x = 1;

    if (x > 0) {
        print("positive");
    }

    const y = 2;
}
```

With `"collapse"`, blank lines separate imports, declarations, and control-flow blocks.

```json
{ "blank_lines": "off" }
```

```tyrus
import { readFileSync } from "tyrus:fs";
import { add } from "./math";
function main(): void {
    const x = 1;
    if (x > 0) {
        print("positive");
    }
    const y = 2;
}
```

With `"off"`, the formatter does not add blank lines, and repeated blank lines are collapsed.
