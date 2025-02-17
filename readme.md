# ProxyPlz

```
Access to fetch at 'https://api.example.com/data' from origin 'http://localhost:3000' 
has been blocked by CORS policy: No 'Access-Control-Allow-Origin' header is present 
on the requested resource.
```
## F....

ProxyPlz is a blazing-fast HTTP/S proxy server that gives you instant CORS relief for frontend development and API testing. Built with Rust's safety and Warp's async magic, it's your lightweight ally for:

    CORS-free development 🛡️ - Automatic Access-Control-* headers

    Universal API access 🌍 - Call any endpoint from any origin

    Method mastery 🥋 - Full HTTP method support (GET/POST/PUT/etc)

    Security-first 🔒 - Timeouts & redirect controls out-of-the-box

## Run

```bash
proxyplz 127.0.0.1:8080
```

## Request

```bash
curl "http://localhost:8080?url=https://api.awesome-service.com/data"
```