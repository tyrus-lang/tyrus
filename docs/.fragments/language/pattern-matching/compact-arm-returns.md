```tyrus
const main = (): void => {
    const codes: i32[] = [1, 2, 3];
    for (const code of codes) {
        match (code) {
            1 => print("one"); // returns from `main`
            _ => { print("other"); } // continues after the `match`
        }
        print("next");
    }
    print("done");
};
```
