```tyrus
const day: string = "Monday";

match (day) {
    ("Saturday" | "Sunday") => {
        print("Weekend");
    }
    ("Monday" | "Tuesday" | "Wednesday" | "Thursday" | "Friday") => {
        print("Weekday");
    }
    _ => {
        print("Invalid day");
    }
}
// "Weekday"
```
