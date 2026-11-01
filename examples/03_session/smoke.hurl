# Smoke: marty-03-session (sync signed cookie session; counter survives the second request).
GET {{base_url}}/cgi-bin/marty-03-session
HTTP 200
[Asserts]
body contains "03_session"
body contains "counter = 1"

# Second request: Hurl sends the cookie back; the signed v1 cookie must verify.
GET {{base_url}}/cgi-bin/marty-03-session
HTTP 200
[Asserts]
body contains "counter = 2"
header "Set-Cookie" matches /^marty_03=v1\.[0-9]+\./
