```tyrus
import { EOL, arch, hostname, language, locale, platform, release, systemName } from "tyrus:os";
import { OsArch, OsPlatform } from "tyrus:os";

const currentPlatform: OsPlatform = platform();
const currentArch: OsArch = arch();

print(`${systemName()} ${release()} (${hostname()})${EOL}locale ${locale()} (${language()})`);
// "Linux 6.8.0-generic (my-machine)"
// "locale en-US (en)"

if (currentPlatform === OsPlatform.Windows) {
    print("running on Windows");
}
```
