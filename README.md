# qui-vive-rs

Run `cargo run` to build and run qui-vive locally (http://127.0.0.1:8080).

## Command-line Interface

```
USAGE:
    qui-vive [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Sets the level of verbosity

OPTIONS:
        --cache-type <type>                   The cache type (redis, memory)
        --external-url <URL>                  The external URL (https://qui-vive.link)
        --listener-url <URL>                  The listener URL (http://127.0.0.1:8080)
        --redis-hostname <hostname[:port]>    The redis hostname
        --redis-password <password>           The redis password
```

## REST API

`$ export QV=http://127.0.0.1:8080`

Key-value storage:
`$ curl -X POST $QV/key -d "sample data"`

URL shortener:
`$ curl -X POST $QV/url -d "https://contoso.io/very-long-url"`

Invitation (key-value + URL shortener):
`$ curl -X POST $QV/inv -H "QuiVive-IdParam: id" -H "QuiVive-DstUrl: https://contoso.io/invite"`
