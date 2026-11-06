```tyrus
const main = () => {
    const getLetterGrade = (score: u8): string => {
        match (score) {
            90..=100 => "A";
            80..=89 => "B";
            70..=79 => "C";
            60..=69 => "D";
            0..=59 => "F";
            _ => "Invalid Score";
        }
    };

    print(getLetterGrade(95)); // A
};
```
