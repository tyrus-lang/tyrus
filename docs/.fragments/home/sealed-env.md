```json
{
  "security": {
    "envFile": ".env",
    "envSealed": true
  }
}
```

```text
# .env (local secrets)         # .env.example (mirrored & committed)
@type(u16)                     @type(u16)
@port()                        @port()
PORT=8080                      PORT=

@type(string)                  @type(string)
@startsWith("postgres://")     @startsWith("postgres://")
DATABASE_URL=postgres://...    DATABASE_URL=

@type(string)                  @type(string)
@minLength(16)                 @minLength(16)
API_SECRET=super_secret_value  API_SECRET=
```

