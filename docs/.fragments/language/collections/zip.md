```tyrus
const names: string[] = ["ana", "bia"];
const scores: i32[] = [30, 25];
try {
    const pairs: [string, i32][] = names.zip(scores); // [["ana", 30], ["bia", 25]]
    print(pairs);
} catch (error: ZipError) {
    print(error.code);
}
```
