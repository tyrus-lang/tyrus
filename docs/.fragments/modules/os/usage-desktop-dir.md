```tyrus
import { desktopDir } from "tyrus:os";

const desktop: string? = desktopDir();
if (desktop) {
    print(`desktop: ${desktop}`);
}
```
