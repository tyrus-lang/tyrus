```tyrus
const names: string[] = ["Ada", "Linus"];
const first: string = names[0]; // "Ada"
const count: usize = names.length; // 2

let statuses: Array<i32> = [200, 201];
statuses[1] = 204; // [200, 204]
statuses.push(202); // [200, 204, 202]

const successful: boolean = statuses.includes(200); // true
const page: i32[] = statuses.slice(0, 2); // [200, 204]
const immutableReverse: i32[] = statuses.toReversed(); // [202, 204, 200]
```
