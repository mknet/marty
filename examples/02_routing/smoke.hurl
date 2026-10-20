# Smoke: marty-02-routing (sync marty::Router) behind _http-cgi-server.
GET {{base_url}}/cgi-bin/marty-02-routing
HTTP 200
[Asserts]
body contains "root"

GET {{base_url}}/cgi-bin/marty-02-routing/hello/Doc
HTTP 200
[Asserts]
body contains "hello, Doc"

# Query string must not leak into PATH_INFO / the `{name}` segment.
GET {{base_url}}/cgi-bin/marty-02-routing/hello/Doc?ignored=1
HTTP 200
[Asserts]
body contains "hello, Doc"

# One path segment: `<a href="/">Biff</a>` must be percent-encoded (slashes/quotes break segments).
GET {{base_url}}/cgi-bin/marty-02-routing/hello/%3Ca%20href%3D%22%2F%22%3EBiff%3C%2Fa%3E
HTTP 200
[Asserts]
body contains "hello, &lt;a href=&quot;/&quot;&gt;Biff&lt;/a&gt;"

GET {{base_url}}/cgi-bin/marty-02-routing/no-such-route
HTTP 404
[Asserts]
body contains "Not found"
