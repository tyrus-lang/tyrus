```tyrus
import { readFileSync } from "tyrus:fs";     // native module
import { add, minus as sub } from "./math";  // named, with an alias
import sum from "./math";                    // default
import sum, { minus } from "./math";         // default plus named
import * as math from "./math";              // namespace
import { default as plus } from "./math";    // explicit default
```

```json
{
  "imports": {
    "#src/*": "./src/*"
  }
}
```

```tyrus
import { settings } from "#src/config/settings";
import { formatDate } from "#src/utils/dates";
```
