# Build CGI binaries for Linux, then serve with Apache (mod_php + mod_cgi).
FROM rust:1-bookworm AS rust
WORKDIR /repo
COPY . .
RUN cargo build -p marty-webhook-bench --release

FROM golang:1.22-bookworm AS go
WORKDIR /src
COPY benchmarks/simple/go-webhook/ ./
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /out/go-webhook .

FROM php:8.3-apache-bookworm
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && a2enmod cgi \
    && rm -rf /var/lib/apt/lists/*

COPY benchmarks/simple/docker/apache-bench.conf /etc/apache2/sites-available/000-bench.conf
RUN a2dissite 000-default.conf \
    && a2ensite 000-bench.conf

COPY benchmarks/simple/php/ /var/www/html/php/
COPY --from=rust /repo/target/release/marty-webhook-bench /usr/lib/cgi-bin/marty-webhook
COPY --from=go /out/go-webhook /usr/lib/cgi-bin/go-webhook
COPY benchmarks/simple/python-webhook/webhook.py /usr/lib/cgi-bin/python-webhook

RUN chmod 755 /usr/lib/cgi-bin/marty-webhook /usr/lib/cgi-bin/go-webhook /usr/lib/cgi-bin/python-webhook

EXPOSE 80
