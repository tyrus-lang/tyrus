```tyrus
interface SignupInput {
    @minLength(3)
    username: string;

    @minLength(8, "password must have at least 8 characters")
    password: string;

    @email("use a valid company e-mail")
    email: string;
}
```
