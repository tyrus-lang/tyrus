# Project Wycheproof test vectors

`aes_gcm_test.json` is the complete AES-GCM corpus from [C2SP/wycheproof](https://github.com/C2SP/wycheproof) revision `fc24cd5b787d8e496bff31b0468af693a652b0f2`. The tyrus runtime executes every vector matching its fixed AES-256-GCM profile: 256-bit keys, 96-bit nonces, and 128-bit authentication tags. The corpus is distributed under the Apache License 2.0 included in this directory.
