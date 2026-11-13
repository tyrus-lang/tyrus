```sh
tyrus db explain-sql "SELECT COUNT(*) AS total FROM db_examples WHERE id >= ?"
```

```text
SQL: SELECT COUNT (*) AS total FROM db_examples WHERE id >= ?
Parameters:
  ?1: i64 (value hidden)
Result fields:
  total: i64
```
