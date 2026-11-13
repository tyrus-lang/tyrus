```tyrus
import { styleText } from "tyrus:utils";

printError(styleText("green", "created"));
printError(styleText(["bold", "cyan"], "tyrus"));
printError(styleText.bold.red("failed"));
```
