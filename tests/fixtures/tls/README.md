# TLS test material

A certificate authority, a server certificate it signed, and that certificate's private key. Used only by `tests/http_tls.rs`, which starts a server on the loopback interface and connects to it with a client that trusts `ca.cert.pem` and nothing else.

- `ca.cert.pem`, the test authority. Only the client uses it, to have something to trust.
- `localhost.cert.pem`, what the server presents. Issued to `localhost` and `127.0.0.1`.
- `localhost.key.pem`, that certificate's key.

The authority's own private key is deliberately **not** kept: it was used to sign the leaf and then discarded, so nothing in the repository can issue further certificates under this authority.

The server key is committed on purpose and is worthless: it is issued by an authority no machine trusts, it names only the loopback interface, and every copy of this repository has it. Nothing in the project, the toolchain or the documentation points a user at these files, and `tyrus new` does not copy them into a generated project.

Regenerate with:

```
openssl req -x509 -newkey rsa:2048 -keyout ca.key.pem -out ca.cert.pem -days 36500 -nodes -subj "/CN=Tyrus Test CA" -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -newkey rsa:2048 -keyout localhost.key.pem -out localhost.csr -nodes -subj "/CN=localhost"
printf "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n" > leaf.ext
openssl x509 -req -in localhost.csr -CA ca.cert.pem -CAkey ca.key.pem -CAcreateserial -out localhost.cert.pem -days 36500 -extfile leaf.ext
rm -f localhost.csr leaf.ext ca.key.pem ca.cert.srl
```
