```tyrus
const scores: i32[] = [10, 20, 30];
const point: [i32, i32] = [10, 20];
const pair: Tuple<string, i32> = ["one", 1];
```

### Array repeat: `[value; count]`

Create an array where every element is the same value. The count must be a positive constant integer expression from 1 through 65,536:

```tyrus
const zeros: i32[] = [0; 5]; // [0, 0, 0, 0, 0]
const flags: boolean[] = [false; 4]; // [false, false, false, false]
const xs: i32[] = [1 + 2; 3]; // [3, 3, 3]
const greetings: string[] = ["hi"; 4]; // ["hi", "hi", "hi", "hi"]
const matrix: i32[][] = [[0; 3]; 3]; // [[0, 0, 0], [0, 0, 0], [0, 0, 0]]
```

### Array.withCapacity: pre-allocated arrays

Create an empty array with pre-allocated buffer space to avoid repeated reallocations during `push`. The element type comes from the variable annotation:

```tyrus
let xs: i32[] = Array.withCapacity(100);
// xs.length == 0, xs.capacity == 100

xs.push(1);
// xs.length == 1, xs.capacity == 100 (no reallocation)
```
