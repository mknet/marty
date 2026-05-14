# Smoke: marty-04-cookies (tower-cookies).
GET {{base_url}}/cgi-bin/marty-04-cookies
HTTP 200
[Asserts]
header "Set-Cookie" contains "marty_04"
body contains "04_cookies"
