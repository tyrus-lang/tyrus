```tyrus
enum EventKind {
    Click,
    KeyPress,
    Submit,
}

type Event =
    | { kind: EventKind.Click; x: i32; y: i32 }
    | { kind: EventKind.KeyPress; key: string }
    | { kind: EventKind.Submit; formId: string };

const handleEvent = (event: Event): void => {
    match (event) {
        ({ kind: EventKind.Click, x, y }) => {
            print(`Click at (${x}, ${y})`); // "Click at (10, 20)"
        }
        ({ kind: EventKind.KeyPress, key }) => {
            print(`Key: ${key}`); // "Key: enter"
        }
        ({ kind: EventKind.Submit, formId }) => {
            print(`Submit: ${formId}`); // "Submit: login"
        }
    }
};
```
