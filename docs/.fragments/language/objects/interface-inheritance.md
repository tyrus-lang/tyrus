```tyrus
interface Age {
    age: u32;
}

interface Person extends Age {
    name: string;
}

// `Person` requires both the inherited `age` and its own `name`.
const person: Person = { age: 30, name: "Ada" };

// An interface that extends another and adds nothing has just the inherited
// fields.
interface JustAge extends Age {}
const onlyAge: JustAge = { age: 25 };
```
