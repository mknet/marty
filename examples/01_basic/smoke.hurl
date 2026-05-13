# Smoke test: this example’s CGI binary behind _http-cgi-server.
# From this directory: `just` (or: hurl --variable base_url=http://127.0.0.1:8080 --retry 50 smoke.hurl)
GET {{base_url}}/cgi-bin/example-01-basic
HTTP 200
[Asserts]
body contains "88 miles per hour"
