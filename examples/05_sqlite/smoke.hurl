# Smoke: marty-05-sqlite (rusqlite).
GET {{base_url}}/cgi-bin/marty-05-sqlite
HTTP 200
[Asserts]
body contains "05_sqlite"
body contains "Bob"
