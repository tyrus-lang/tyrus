```text
error[E0708]: a `char` holds exactly one character, but `ab` has 2
at /src/main.tyr:1:22
const letter: char = "ab";
                     ^^^^
help: annotate the value with `string` to hold text of any length, or keep a single character
```
