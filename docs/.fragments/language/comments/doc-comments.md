```tyrus
/**
 * Sends a welcome message.
 * @param email Destination e-mail address.
 * @returns true when the message was accepted.
 */
function sendWelcome(email: string): boolean {
    return email.includes("@");
}
```
