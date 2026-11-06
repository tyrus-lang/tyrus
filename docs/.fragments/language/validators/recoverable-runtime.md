```tyrus
interface SignupInput {
    @minLength(3)
    username: string;

    @email()
    email: string;
}

const readUsername = (): string => "a";
const readEmail = (): string => "invalid";

const buildSignup = (): SignupInput throws ValidationError => {
    return {
        username: readUsername(),
        email: readEmail(),
    };
};

const main = (): void => {
    try {
        const input: SignupInput = buildSignup();
        print(input.username); // "a"
    } catch (error: ValidationError) {
        match (error.issues.length === (1 as usize)) {
            (true) => {
                const issue = error.issues[0];
                print(issue.target); // "username"
                print(issue.message); // "must be at least 3 characters"
            }
            _ => {
                for (const issue of error.issues) {
                    print(issue.target); // field name
                    print(issue.message); // validation message
                }
            }
        }
    }
};
```
