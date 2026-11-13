```tyrus
import { writeFileSync, type FsError } from "tyrus:fs";
import { input, confirm } from "tyrus:cli";
import { styleText } from "tyrus:utils";

const main = (): void => {
    const name: string = input({ message: "Project name?" });
    const proceed: boolean = confirm({ message: `Create "${name}"?` });

    if (proceed) {
        match (writeFileSync(`${name}/main.tyr`, `print("hello from ${name}");`)) {
            Ok() => print(styleText("green", `Created ${name}`));
            Err(error) => printError(error.message);
        }
    }
};
```
