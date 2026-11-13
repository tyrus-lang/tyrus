```tyrus
type Validator = { isValid: true } | { isValid: false; message: string };

validator function hasPrefix(value: string, constraint: string): Validator {
    if (!value.startsWith(constraint)) {
        return { isValid: false, message: "Value must start with the configured prefix" };
    }
    return { isValid: true };
}

validator const hasNoSpaces = (value: string): Validator => {
    if (value.includes(" ")) {
        return { isValid: false, message: "Value cannot contain spaces" };
    }
    return { isValid: true };
};

interface UserInput {
    @hasPrefix("user_")
    @hasNoSpaces()
    username: string;
}

function acceptUsername(@hasNoSpaces() value: string): string {
    return value;
}

const main = (): void => {
    try {
        const user: UserInput = { username: "guest user" };
        print(user.username);
    } catch (error: ValidationError) {
        print(error.issues.length);
        print(error.issues[0].target);
        print(error.issues[0].validator);
        print(error.issues[0].message);
        print(error.issues[1].validator);
        print(error.issues[1].message);
    }
};
```
