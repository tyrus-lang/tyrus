```tyrus
const uniqueTags: Set<string> = Set(["rust", "native", "rust"]);
uniqueTags.add("fast");                      // Set { "rust", "native", "fast" }
const hasNative: boolean = uniqueTags.has("native"); // true
uniqueTags.delete("fast");                   // Set { "rust", "native" }
const count: usize = uniqueTags.size;        // 2

for (const tag of uniqueTags) {
    print(tag); // "rust", "native"
}
```