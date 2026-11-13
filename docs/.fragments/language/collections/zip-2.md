```tyrus
const names: string[] = ["ana", "bia"];
const scores: i32[] = [30];
try {
    const pairs: [string, i32][] = names.zip(scores);
    print(pairs.length);
} catch (error: ZipError) {
    print(error.code); // ERR_ARRAY_ZIP_MISMATCH
}
```
