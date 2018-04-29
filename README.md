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
        --id-charset <charset>                The generated id character set
        --id-length <length>                  The generated id length
        --listener-url <URL>                  The listener URL (http://127.0.0.1:8080)
        --redis-hostname <hostname[:port]>    The redis hostname
        --redis-password <password>           The redis password
```

## Sample Usage

By default, qui-vive stores data in an in-memory cache and listens on localhost HTTP port 8080.

For simplicity, export a 'QV' environment variable to point to your qui-vive listener:
`$ export QV=http://127.0.0.1:8080`

First, check that qui-vive is running and healthy:
`$ curl -v http://127.0.0.1:8080/health`

If you do not get an HTTP 200 OK response, then qui-vive is not working properly.

### Key-Value Storage

Create store a new value with a generated id with a POST request on /key with the value in the HTTP request body. The URL that can be used to retrieve the value is returned in the HTTP response body.
```
$ curl -X POST http://127.0.0.1:8080/key -d "this is my sample data"
http://127.0.0.1:8080/key/xXq3FSJK5
```

Use the returned URL to fetch the value and confirm that it is what we expect:
```
$ curl http://127.0.0.1:8080/key/xXq3FSJK5
this is my sample data
```

### URL shortener

Create a short URL that will redirect to a longer one with a POST request on /url with the destination in the HTTP request body. The short URL that can be used to redirect to the long URL is returned in the HTTP response body.

```
curl -X POST http://127.0.0.1:8080/url -d "https://github.com/wayk/qui-vive-rs/"
http://127.0.0.1:8080/TwfdpHQJC
```

If you use the short URL in a browser, it should properly redirect to the long URL. Otherwise, confirm that it works using curl:

```
$ curl -v http://127.0.0.1:8080/TwfdpHQJC
*   Trying 127.0.0.1...
* Connected to 127.0.0.1 (127.0.0.1) port 8080 (#0)
> GET /TwfdpHQJC HTTP/1.1
> Host: 127.0.0.1:8080
> User-Agent: curl/7.47.0
> Accept: */*
> 
< HTTP/1.1 301 Moved Permanently
< Location: https://github.com/wayk/qui-vive-rs/
< Content-Length: 0
< Date: Sun, 29 Apr 2018 02:38:07 GMT
< 
* Connection #0 to host 127.0.0.1 left intact
```

### Invitation Link

The invitation link feature is a combination of the key-value storage and URL shortener.

```
$ curl -X POST http://127.0.0.1:8080/inv \
-H "QuiVive-IdParam: id" \
-H "QuiVive-DstUrl: https://contoso.com/meeting"
```
