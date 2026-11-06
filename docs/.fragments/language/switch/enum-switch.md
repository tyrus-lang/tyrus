```tyrus
enum PaymentStatus {
    Pending,
    Paid,
    Refused,
}

const status: PaymentStatus = PaymentStatus.Paid;

switch (status) {
    case PaymentStatus.Pending: {
        print("pending");
        break;
    }
    case PaymentStatus.Paid: {
        print("paid");
        break;
    }
    case PaymentStatus.Refused: {
        print("refused");
        break;
    }
}
```
