#!/bin/sh
# Start php-fpm in the background, then Apache in the foreground (hosting-like bench container).
set -eu

: "${BENCH_TIMING:=0}"
export BENCH_TIMING

mkdir -p /run/php
php-fpm8.3 --daemonize --fpm-config /etc/php/8.3/fpm/php-fpm.conf

# Ubuntu's apache2ctl environment (APACHE_LOG_DIR, APACHE_RUN_USER, ...). envvars expects
# APACHE_CONFDIR to be set (or unset under `set +u`), so relax -u around the source.
export APACHE_CONFDIR=/etc/apache2
set +u
. /etc/apache2/envvars
set -u
# apache2ctl normally creates these; mod_cgid needs the socks dir for its ScriptSock.
mkdir -p "$APACHE_RUN_DIR/socks" "$APACHE_LOCK_DIR" "$APACHE_LOG_DIR"
exec apache2 -DFOREGROUND
