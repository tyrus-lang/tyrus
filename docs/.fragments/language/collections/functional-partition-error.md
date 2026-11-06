```tyrus
interface Ticket {
    completed: boolean;
}

const tickets: Ticket[] = [{ completed: true }];
const [completed, pending] = tickets.partition("archived"); // [!code error]
```

```text
error[E0609]: `partition` key `archived` is not a field of `Ticket`
at /src/main.tyr:6:30
const [completed, pending] = tickets.partition("archived");
                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
