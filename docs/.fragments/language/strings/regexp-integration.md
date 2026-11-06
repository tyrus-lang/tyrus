```tyrus
const text: string = "error: file not found";
const hasError: boolean = text.includes("error"); // true
const firstSpace: i64 = text.indexOf(" "); // 5
const afterColon: string = text.substring(firstSpace + 1);
// "file not found"

const matched: string[]? = text.match(/error: (.+)/);

if (matched) {
    const message: string = matched[1]; // "file not found"
}

const replaced: string = text.replace("error", "warning");
// "warning: file not found"

const parts: string[] = text.split(" ");
// ["error:", "file", "not", "found"]
```
