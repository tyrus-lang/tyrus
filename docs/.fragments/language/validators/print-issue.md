```tyrus
const printIssue = (issue: ValidationIssue): void => {
    print(issue.validator); // "email"
    print(issue.message); // "invalid e-mail"
};

const printValidation = (error: ValidationError): void => {
    for (const issue of error.issues) {
        printIssue(issue);
    }
};
```
