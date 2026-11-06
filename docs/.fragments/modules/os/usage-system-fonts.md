```tyrus
import { FontInfo, systemFonts } from "tyrus:os";

const fonts: FontInfo[] = systemFonts();
print(`${fonts.length} faces installed`);
if (fonts.length > 0) {
    print(`${fonts[0].name} (${fonts[0].style})`);
}
```
