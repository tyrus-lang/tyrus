```json
{ "blank_lines_around_blocks": true }
```

```tyrus
function main(): void {
    const x = 1;

    if (x > 0) {
        print("positive");
    }

    for (let i = 0; i < x; i++) {
        print(i);
    }
}
```

With `true`, blank lines surround block statements and function declarations.

```json
{ "blank_lines_around_blocks": false }
```

```tyrus
function main(): void {
    const x = 1;
    if (x > 0) {
        print("positive");
    }
    for (let i = 0; i < x; i++) {
        print(i);
    }
}
```

With `false`, blocks sit directly adjacent to surrounding statements.
