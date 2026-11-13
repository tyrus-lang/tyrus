```dotenv
@type(string)
@in(["development", "production", "test", "staging"])
TYRUS_ENV=development

@type(u32)
@port("PORT must be a registered port between 1024 and 49151")
PORT=3000

@type(string)
SECRET=key=with=multiple=equals
```
