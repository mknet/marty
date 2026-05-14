# Smoke: marty-03-session (tower-sessions, single-request counter).
GET {{base_url}}/cgi-bin/marty-03-session
HTTP 200
[Asserts]
body contains "03_session"
body contains "counter = 1"
