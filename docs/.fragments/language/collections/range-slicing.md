```tyrus
const letters: string[] = ["a", "b", "c", "d", "e"];
print(letters[1..4]);
// ["b", "c", "d"] same as letters.slice(1, 4)
print(letters[1..=3]);
// ["b", "c", "d"] inclusive of index 3
print(letters[2..]);
// ["c", "d", "e"]
print(letters[..3]);
// ["a", "b", "c"]
print(letters[..]);
// ["a", "b", "c", "d", "e"]
```
