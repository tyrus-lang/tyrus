```tyrus
interface Account {
    id: i32;
    email: string;
    active: boolean;
}

type AccountPatch = Partial<Account>;
const patch: AccountPatch = { email: "a@b.com" };
```
