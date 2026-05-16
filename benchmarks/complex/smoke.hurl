# Complex benchmark smoke (?salt=0 → deterministic workload for assertions).
GET {{base_url}}/cgi-bin/marty-complex/primes/400000?salt=0
HTTP 200
[Asserts]
jsonpath "$.route" == "primes"
jsonpath "$.salt" == 0
jsonpath "$.result" == 33860

GET {{base_url}}/cgi-bin/marty-complex/fibonacci/42?salt=0
HTTP 200
[Asserts]
jsonpath "$.route" == "fibonacci"
jsonpath "$.salt" == 0
jsonpath "$.result" == 803742888000

GET {{base_url}}/cgi-bin/marty-complex/matrix/128?salt=0
HTTP 200
[Asserts]
jsonpath "$.route" == "matrix"
jsonpath "$.salt" == 0
jsonpath "$.result.size" == 128

GET {{base_url}}/cgi-bin/go-complex/primes/400000?salt=0
HTTP 200
[Asserts]
jsonpath "$.salt" == 0
jsonpath "$.result" == 33860

GET {{base_url}}/cgi-bin/go-complex/fibonacci/42?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 803742888000

GET {{base_url}}/cgi-bin/python-complex/primes/400000?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 33860

GET {{base_url}}/cgi-bin/python-complex/fibonacci/42?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 803742888000

GET {{base_url}}/php/complex.php/primes/400000?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 33860

GET {{base_url}}/php/complex.php/fibonacci/42?salt=0
HTTP 200
[Asserts]
jsonpath "$.result" == 803742888000

GET {{base_url}}/php/complex.php/matrix/128?salt=0
HTTP 200
[Asserts]
jsonpath "$.route" == "matrix"
