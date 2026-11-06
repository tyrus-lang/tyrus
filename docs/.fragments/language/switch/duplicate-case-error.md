```text
error[E0003]: duplicate match case
  --> src/main.tyr:4:10
   |
 2 |     case "admin": {
   |          ^^^^^^^ first use here
 4 |     case "admin": { // [!code error]
   |          ^^^^^^^ duplicate case
   |
help: remove the duplicate case or use a different value
```