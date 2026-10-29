# `04_complex` — AWS Lambda twin of Marty `marty-sync` (complex CPU)

Marty reference: [`benchmarks/complex/marty-sync`](../../../benchmarks/complex/marty-sync/) — primes / fibonacci / matrix JSON via shared [`marty-bench-shared`](../../../benchmarks/complex/shared/).

Same algorithms and JSON as the hosting outside bench (Run N). Fair compare: [`../../compare-outside.sh`](../../compare-outside.sh).

| | Marty CGI (`marty-sync`) | This sample |
|---|---------------------------|-------------|
| Entry | sync `Router` → CGI stdout | `lambda_http` + Function URL |
| Paths | `/cgi-bin/marty-sync/primes/…` | `/primes/{limit}`, `/fibonacci/{n}`, `/matrix/{size}` |
| Memory | host CGI | **128 MB**, timeout 30 s |

## Build / deploy

From `alternatives/aws-lambda/`:

```bash
just login
just deploy-one 04_complex
# printed Function URL → set AWS_COMPLEX_URL in alternatives/targets.env
```

Smoke:

```bash
curl -sS "${AWS_COMPLEX_URL}/primes/400000?salt=0"
```
