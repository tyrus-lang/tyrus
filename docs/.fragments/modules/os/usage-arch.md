```tyrus
import { arch } from "tyrus:os";
import { OsArch } from "tyrus:os";

const currentArch: OsArch = arch();

if (currentArch === OsArch.Aarch64) {
    print("running on an ARM64 target");
}
```
