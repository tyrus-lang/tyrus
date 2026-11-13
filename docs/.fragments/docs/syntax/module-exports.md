```tyrus
// math.tyr
export const add = (left: i32, right: i32): i32 => left + right;

const subtract = (left: i32, right: i32): i32 => left - right;
export { subtract as minus };

export default add;
```
