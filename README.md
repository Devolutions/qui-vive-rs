# qui-vive-rs

A key-value store and url shortener that is always on alert.

Use `cargo run` to build and run qui-vive locally with default options.

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
        --default-expiration <expiration>     The default key expiration, in seconds
        --external-url <URL>                  The external URL (https://qui-vive.link)
        --id-charset <charset>                The generated id character set
        --id-length <length>                  The generated id length
        --listener-url <URL>                  The listener URL (http://127.0.0.1:8080)
        --redis-hostname <hostname[:port]>    The redis hostname
        --redis-password <password>           The redis password
```

## Sample Usage

By default, qui-vive stores data in an in-memory cache and listens on localhost HTTP port 8080.

First, check that qui-vive is running and healthy:
```
$ curl -w "%{http_code}" http://127.0.0.1:8080/health
200
```

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

The value can also be fetched directly from a browser with the same URL.

Alternatively, the key id can be specified at creation time, instead of using the generated id:

```
$ curl -X POST http://127.0.0.1:8080/key/f16a75c1-9804-4e10-85c0-b6a159837c0d \
> -d 'this is my sample data'
http://127.0.0.1:8080/f16a75c1-9804-4e10-85c0-b6a159837c0d
```

You can then fetch the value again using the custom key id:

```
$ curl http://127.0.0.1:8080/f16a75c1-9804-4e10-85c0-b6a159837c0d
this is my sample data
```

### URL shortener

Create a short URL that will redirect to a longer one with a POST request on /url with the destination in the HTTP request body. The short URL that can be used to redirect to the long URL is returned in the HTTP response body.

```
curl -X POST http://127.0.0.1:8080/url \
> -d "https://github.com/devolutions/qui-vive-rs/"
http://127.0.0.1:8080/TwfdpHQJC
```

If you use the short URL in a browser, it should properly redirect to the long URL. Otherwise, confirm that it works using curl:

```
$ curl -w "%{redirect_url}" http://127.0.0.1:8080/stgQBECEz
https://github.com/devolutions/qui-vive-rs/
```

### Invitation Link

The invitation link feature is a combination of the key-value storage and URL shortener. The idea is to generate a short link to a page that will then load data from the key-value storage using an id provider in a query parameter.

Create an invitation that redirects to https://contoso.com/meeting?id=<qui-vive-id>:
```
$ curl -X POST http://127.0.0.1:8080/inv \
> -H "QuiVive-IdParam: id" \
> -H "QuiVive-DstUrl: https://contoso.com/meeting" \
> -d '{"meeting":"master plan","organizer":"ceo@contoso.com"}'
http://127.0.0.1:8080/KT2HKxVRi
```

Fetch destination URL:
```
$ curl -w "%{redirect_url}" http://127.0.0.1:8080/KT2HKxVRi
https://contoso.com/meeting?id=KT2HKxVRi
```

Fetch associated data:
```
$ curl http://127.0.0.1:8080/key/KT2HKxVRi
{"meeting":"master plan","organizer":"ceo@contoso.com"}
```

The destination page should use the id=<qui-vive-id> query parameter in the URL to fetch the associated data and present the invitation information to the user.

### Key Expiration

All keys will expire by default after 24 hours (86400 seconds), unless the expiration is explicitly set to something else. A value of 0 means no expiration. To specify the desired expiration (in seconds) when creating a new key, use the "QuiVive-Expiration" HTTP header.

```
$ curl -X POST http://127.0.0.1:8080/key \
> -H "QuiVive-Expiration: 86400" \
> -d 'this is my sample data'
http://127.0.0.1:8080/KT2HKxVRi
```
