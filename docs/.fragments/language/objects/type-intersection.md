```tyrus
interface Age {
    age: u32;
}

interface Contact {
    email: string;
}

// `&` combines object types into one named type.
type Person = Age & Contact;
const person: Person = { age: 30, email: "ada@example.com" };

// An operand can be written inline, anywhere in the chain.
type Employee = { title: string; } & Age & Contact;
const employee: Employee = { title: "Engineer", age: 41, email: "grace@example.com" };
```
