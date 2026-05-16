# CGI-only complex smoke (?salt=0).
GET {{base_url}}/cgi-bin/marty-complex/primes/400000?salt=0
HTTP 200
[Asserts]
jsonpath "$.salt" == 0
jsonpath "$.result" == 33860

GET {{base_url}}/cgi-bin/marty-complex/fibonacci/42?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 803742888000

GET {{base_url}}/cgi-bin/marty-complex/matrix/128?salt=0
HTTP 200
[Asserts]
jsonpath "$.route" == "matrix"

GET {{base_url}}/cgi-bin/go-complex/primes/10000?salt=0
HTTP 200
[Asserts]
jsonpath "$.route" == "primes"

GET {{base_url}}/cgi-bin/python-complex/fibonacci/42?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 803742888000
