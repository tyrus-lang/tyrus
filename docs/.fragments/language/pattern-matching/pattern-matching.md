```tyrus
function label(score: i32): string {
    match (score) {
        90..=100 => "A";
        _ => { return "F"; }
    }
}
```
