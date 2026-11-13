```tyrus
interface Age {
    age: u32;
}

interface Contact {
    email: string;
}

function greet(person: Age & Contact): void { // [!code error]
    print(person.email);
}
```
