# Build CGI binaries for Linux, then serve with Apache (mod_php + mod_cgi).
FROM rust:1-bookworm AS rust
WORKDIR /repo
COPY . .
RUN cargo build -p marty-complex-bench --release

FROM golang:1.22-bookworm AS go
WORKDIR /src
COPY benchmarks/complex/go-complex/ ./
RUN CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /out/go-complex .

FROM php:8.3-apache-bookworm
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && a2enmod cgi \
    && mkdir -p /var/log/bench-timing \
    && chown www-data:www-data /var/log/bench-timing \
    && rm -rf /var/lib/apt/lists/*

COPY benchmarks/complex/docker/apache-bench.conf /etc/apache2/sites-available/000-bench.conf
RUN a2dissite 000-default.conf \
    && a2ensite 000-bench.conf

COPY benchmarks/complex/php/ /var/www/html/php/
COPY --from=rust /repo/target/release/marty-complex-bench /usr/lib/cgi-bin/marty-complex
COPY --from=go /out/go-complex /usr/lib/cgi-bin/go-complex
COPY benchmarks/complex/python-complex/complex.py /usr/lib/cgi-bin/python-complex

RUN chmod 755 /usr/lib/cgi-bin/marty-complex /usr/lib/cgi-bin/go-complex /usr/lib/cgi-bin/python-complex

EXPOSE 80
