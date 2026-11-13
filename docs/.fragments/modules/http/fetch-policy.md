```tyrus
const app: HttpRouter = createRouter();
app.fetchPolicy({ allow: ["internal.example.com"], denyPrivate: true });
```
