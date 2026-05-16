# Correctness smoke for all webhook benchmark handlers.
POST {{base_url}}/cgi-bin/marty-webhook
Content-Type: application/json
X-Webhook-Secret: bench-secret
```json
{"id":"evt_smoke","type":"payment.completed"}
```
HTTP 200
[Asserts]
jsonpath "$.received" == true
jsonpath "$.id" == "evt_smoke"

POST {{base_url}}/cgi-bin/go-webhook
Content-Type: application/json
X-Webhook-Secret: bench-secret
```json
{"id":"evt_smoke","type":"payment.completed"}
```
HTTP 200
[Asserts]
jsonpath "$.received" == true
jsonpath "$.id" == "evt_smoke"

POST {{base_url}}/cgi-bin/python-webhook
Content-Type: application/json
X-Webhook-Secret: bench-secret
```json
{"id":"evt_smoke","type":"payment.completed"}
```
HTTP 200
[Asserts]
jsonpath "$.received" == true
jsonpath "$.id" == "evt_smoke"

POST {{base_url}}/php/webhook.php
Content-Type: application/json
X-Webhook-Secret: bench-secret
```json
{"id":"evt_smoke","type":"payment.completed"}
```
HTTP 200
[Asserts]
jsonpath "$.received" == true
jsonpath "$.id" == "evt_smoke"
