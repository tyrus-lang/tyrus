```tyrus
interface Contact {
    @email()
    email?: string;
}

const acceptAge = (@min(18) age?: i32): void => {
    print(age ?? -1); // -1
};

const main = (): void => {
    const empty: Contact = {};
    print(empty.email ?? "no e-mail"); // "no e-mail"
    try {
        acceptAge();
    } catch (error: ValidationError) {
        print(error.issues[0].validator); // "min"
    }
};
```
