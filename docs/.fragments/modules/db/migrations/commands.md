```sh
# Replace `initial_schema` with a descriptive name for this migration.
tyrus db migrate generate initial_schema
tyrus db migrate status
tyrus db migrate apply
tyrus db introspect

# Replace `default_roles` with a descriptive name for this seed.
tyrus db seed create default_roles
tyrus db seed status
tyrus db seed apply
```
