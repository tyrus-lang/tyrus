```tyrus
import { CryptoError, jwt } from "tyrus:crypto";

interface SessionClaims {
    role: string;
}

const key: string = "0123456789abcdef0123456789abcdef";
const claims: SessionClaims = { role: "reader" };
const signed: Result<string, CryptoError> = jwt.sign(claims, key, { algorithm: "HS384", expiresIn: "1h" });
match (signed) {
    Ok(token) => {
        const verified: Result<SessionClaims, CryptoError> = jwt.verify<SessionClaims>(token, key, { algorithms: ["HS384"] });
        match (verified) {
            Ok(value) => {
                print(value.role); // reader
            }
            Err(error) => { printError(error.code); } // ERR_CRYPTO_*
        }
    }
    Err(error) => { printError(error.code); } // ERR_CRYPTO_*
}
```
