```sh
# Compare the edited Models with the snapshot and live database.
tyrus db diff
# Replace add_profile with a descriptive migration name.
tyrus db migrate generate add_profile
# Review and edit the generated SQL file before applying it.
tyrus db migrate apply
# Refresh the snapshot after the database schema changes.
tyrus db introspect
```
