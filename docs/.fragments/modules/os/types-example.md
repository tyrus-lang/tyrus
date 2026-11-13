```tyrus
import { arch, platform } from "tyrus:os";
import { OsArch, OsPlatform } from "tyrus:os";

const currentPlatform: OsPlatform = platform();
const currentArch: OsArch = arch();

print(currentPlatform === OsPlatform.Linux);
print(currentArch === OsArch.X86_64);
```
