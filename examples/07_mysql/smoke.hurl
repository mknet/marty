# Smoke: marty-07-mysql (MySQL/MariaDB). Requires marty_mysql.conf or MARTY_MYSQL_* env.
GET {{base_url}}/cgi-bin/marty-07-mysql
HTTP 200
[Asserts]
body contains "07_mysql"
body contains "Bob"
