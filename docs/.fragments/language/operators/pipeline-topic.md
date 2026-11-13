```tyrus
const log = (label: string, value: i64): void => {
    print(`${label}: ${value}`);
};

const result: void = 42 |> log("answer", %);
```
