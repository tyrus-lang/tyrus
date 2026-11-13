```tyrus
const main = (): void => {
    const scores: unknown = [90, 85]; // [!code error]
    print(scores);
};
```

```text
error[E0794]: cannot store a value of type `integer literal[]` in `unknown`
at /src/main.tyr:2:11
    const scores: unknown = [90, 85];
          ^^^^^^
help: `unknown` holds only `string`, `boolean`, `char`, numeric and `decimal` values, and `integer literal[]` is none of them
help: give `scores` a type that describes the value
```
