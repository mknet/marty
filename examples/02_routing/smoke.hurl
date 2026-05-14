# Smoke: marty-02-routing behind _http-cgi-server (two routes).
GET {{base_url}}/cgi-bin/marty-02-routing
HTTP 200
[Asserts]
body contains "root"

GET {{base_url}}/cgi-bin/marty-02-routing/hello/Doc
HTTP 200
[Asserts]
body contains "hello, Doc"

GET {{base_url}}/cgi-bin/marty-02-routing/no-such-route
HTTP 404
[Asserts]
body contains "Not found"
