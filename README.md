# qui-vive-rs

Run `cargo run` to build and run qui-vive locally (http://127.0.0.1:8080).

## REST API

`$ export QV=http://127.0.0.1:8080`

Key-value storage:
`$ curl -X POST $QV/key -d "sample data"`

URL shortener:
`$ curl -X POST $QV/url -d "https://contoso.io/very-long-url"`

Invitation (key-value + URL shortener):
`$ curl -X POST $QV/inv -H "QuiVive-IdParam: id" -H "QuiVive-DstUrl: https://contoso.io/invite"`
