```tyrus
interface Form {
    name: string;
    email?: string;
    phone?: string;
}

type ContactForm = AtLeastOne<Form, "email" | "phone">;
const formA: ContactForm = { name: "Ada", email: "ada@example.com" };
const formB: ContactForm = { name: "Ada", phone: "555-1234" };
```
