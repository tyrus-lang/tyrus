```tyrus
const main = (): void => {
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
    print(getLetterGrade(80)); // B
    print(getLetterGrade(79)); // C
    print(getLetterGrade(61)); // D
    print(getLetterGrade(59)); // F
    print(getLetterGrade(150)); // Invalid Score
};
```
