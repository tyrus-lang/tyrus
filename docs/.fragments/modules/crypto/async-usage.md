```tyrus
import { CryptoError, argon2, jwt, rand } from "tyrus:crypto";

interface SessionClaims {
    role: string;
}

// Password hashing
match (await argon2.hash("my-password")) {
    Ok(hash) => {
        match (await argon2.verify(hash, "my-password")) {
            Ok(valid) => { print(valid); } // true
            Err(error) => { printError(error.code); } // ERR_CRYPTO_ARGON2_*
        }
        match (argon2.needsRehash(hash)) {
            Ok(migrate) => { print(migrate); } // false
            Err(error) => { printError(error.code); } // ERR_CRYPTO_ARGON2_INVALID_HASH
        }
    }
    Err(error) => { printError(error.code); } // ERR_CRYPTO_ARGON2_*
}

// JWT HMAC requires at least 32 bytes of secret material.
try {
    const key: string = "0123456789abcdef0123456789abcdef";
    const inputClaims: SessionClaims = { role: "reader" };
    const signed: Result<string, CryptoError> = jwt.sign(inputClaims, key, { algorithm: "HS384", expiresIn: "1h", subject: "123" });
    match (signed) {
        Ok(token) => {
            const verified: Result<SessionClaims, CryptoError> = jwt.verify<SessionClaims>(token, key, { algorithms: ["HS384"], subject: "123" });
            match (verified) {
                Ok(claims) => {
                    print(claims.role); // reader
                }
                Err(error) => { printError(error.code); } // ERR_CRYPTO_*
            }
        }
        Err(error) => { printError(error.code); } // ERR_CRYPTO_*
    }
} catch (error: Error) {
    printError(error.message);
}

// Secure random
const id: string = rand.uuidv7();
try {
    const bytes: u8[] = rand.randomBytes(32);
    print(bytes.length);
} catch (error: Error) {
    printError(error.message);
}
```
