```tyrus
import { Color, accessibleColorContrast } from "tyrus:os";

const black: Color = { r: 0, g: 0, b: 0 };
const white: Color = { r: 255, g: 255, b: 255 };
print(accessibleColorContrast(black, white)); // true, 21:1
```
