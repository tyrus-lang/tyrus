```json
{ "brace_style": "same-line" }
```

```tyrus
if (cond) {
    foo();
} else {
    bar();
}
```

With `"same-line"`, the opening brace stays on the header line.

```json
{ "brace_style": "next-line" }
```

```tyrus
if (cond)
{
    foo();
}
else
{
    bar();
}
```

With `"next-line"`, the opening brace drops to its own line, and `else` starts a new line too.

```json
{ "brace_style": "off" }
```

With `"off"`, each code block and continuation keyword keeps its original same-line or next-line layout.
