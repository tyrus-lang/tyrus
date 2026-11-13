```text
warning[prefer-const]: `retries` is never reassigned
  --> src/main.tyr:1:1
   |
 1 | let retries: u32 = 3;
   | ^^^
   |
help: use `const` for `retries`
```
