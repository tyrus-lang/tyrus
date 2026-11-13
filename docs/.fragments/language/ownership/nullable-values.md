```tyrus
interface Profile {
    nickname?: string;
}

const profile: Profile = {};
const nickname: string? = profile.nickname;
const fallback: string = nickname ?? "anonymous"; // "anonymous"

if (nickname) {
    const present: string = nickname;
    print(present); // "nickname value"
}
```
