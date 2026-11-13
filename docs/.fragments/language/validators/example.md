```tyrus
interface UserInput {
    @minLength(3)
    @maxLength(32)
    name: string;

    @email()
    primaryEmail: string;

    @min(18, "The minimum age requirement is 18")
    age: u8;
}

const divide = (a: f64, @notEqual(0) b: f64): f64 => a / b;
```
