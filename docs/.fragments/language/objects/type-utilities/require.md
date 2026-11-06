```tyrus
interface Client {
    id: i32;
    name: string;
    email?: string;
    phone?: string;
}

type ClientWithEmail = Require<Client, "email">;
const client: ClientWithEmail = { id: 1, name: "Ada", email: "ada@example.com" };
```
