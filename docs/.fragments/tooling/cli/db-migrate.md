```sh
# Replace `add_users` with a descriptive name for this migration.
tyrus db migrate create add_users
# Edit db/migrations/0001_add_users.sql before applying it.
tyrus db migrate status
tyrus db migrate apply
tyrus db introspect
tyrus db seed status
# Create db/seeds/default_roles.tyr manually, then edit and run it.
tyrus db apply default_roles.tyr
```
