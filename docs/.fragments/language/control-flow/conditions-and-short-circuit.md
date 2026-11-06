```tyrus
const user: User? = find();

// Ternary and `&&` narrow the optional to its present type in the truthy branch.
const label: string = user ? user.name : "anonymous";
const isAda: boolean = user && user.name === "Ada";

// A logical expression statement runs the right side for its side effect.
const enabled: boolean = ready();
enabled && notify();
invalid() || abort();
```
