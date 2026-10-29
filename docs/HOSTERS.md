# EU shared hosters for Marty

Which shared hosting plans can run a Marty binary as CGI. Prices incl. 19 % German VAT, **checked Oct 2026**; prices change, re-check the tariff pages. Cost comparison with cloud functions: [`COST.md`](COST.md). Product defaults: [`DIRECTION.md`](DIRECTION.md).

**Probe before claiming.** "CGI scripts" in marketing ≠ "arbitrary ELF" on every plan. Run [`tools/hosting-probe`](../tools/hosting-probe/) once on a plan before saying it works. **Verified (Oct 2026):** all-inkl and **Hetzner Webhosting S**. Head-to-head outside sync/axum/php: [`RESULTS-SNAPSHOTS.md` Run N](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-n--outside-compare-hetzner-s-vs-all-inkl-2026-10-28).

## Table

| Hoster / plan | Price / month | Term | SSH | CGI with own binary | Notes |
|---|---|---|---|---|---|
| **all-inkl** Privat / PrivatPlus / Premium / Business | 4.95 / 7.95 / 9.95 / 24.95 € | none, first month free | from Premium | **verified** (Oct 2026: event + mod_cgid + suexec, Rust musl works, Go dies) | servers only in Germany; MySQL via [`examples/07_mysql`](../examples/07_mysql/) (config file, no `SetEnv`) |
| **Hetzner Webhosting S** | 1.90 € (1.60 € net) | none | from L (9.90 €, 8.32 € net) | **verified** (Oct 2026: Debian 13, stdout **socket**/mod_cgid-class, site user, KeepAlive + HTTP/2, Rust musl + PHP work) | cheapest verified plan; needs a domain in the package for HTTP (no `*.your-vhost.de` preview); Go CGI not re-probed |
| netcup Webhosting 1000 | 2.69 € (2.17 € before the 2026 increase) | 12 months | yes, all plans | not documented; **probe** | 2000: 4.03 €, 4000: 8.10 €, 8000: 13.17 €; Python/Node/Ruby only from 4000 |
| lima-city Mini | 3.50 € (12 mo) / 4.50 € (monthly) | 12 or 1 month | yes, no root | not documented; **probe** | |
| **manitu** Webhosting M | 5.99 € (promo 1 € first 6 months) | 1 month | yes | **"CGIs for your own applications" explicitly documented**; probe | L 11.99 €, XL 15.99 €, XXL 19.99 €; best unverified candidate |
| Uberspace | pay what you want, reference 6 / 9 / 12 € | none, prepaid, first month free | yes | not needed | **different model:** "web backends" run your own long-lived HTTP process (port 1024 to 65535, `uberspace web backend set / --http --port N`), so run Axum directly, no CGI |
| webgo | from 6.95 € | n/a | not stated | not stated | no advantage |
| Infomaniak (CH) | n/a | n/a | yes | **CGI module switched off on shared hosting** (security); Perl/Python CGI unsupported | **negative example:** marketing vs reality drift |
| IONOS, DomainFactory / Host Europe, dogado, Strato | various | | | CGI historically / `AddHandler cgi-script` (IONOS); **probe** | keep on the probe list |
| DreamHost, NearlyFreeSpeech.NET (US) | | | | long CGI culture; **probe** | outside the EU |

## Reading the table

- **Cheaper than all-inkl?** Yes: **Hetzner S (1.90 €, verified)** and netcup 1000 (2.69 €, still probe). Hetzner S is the cheapest verified Marty target so far.
- **Latency vs clouds:** in the Oct 2026 complex outside compare, Hetzner S warm P50 tied Cloudflare Workers at the top; all-inkl landed near AWS. [`alternatives/RESULTS-SNAPSHOTS.md`](../alternatives/RESULTS-SNAPSHOTS.md).
- **Strongest unverified candidate:** manitu M, because it documents CGI for your own programs.
- **Uberspace** is a different and attractive model: a long-lived process instead of CGI. Marty is optional there.
- **Sovereignty:** German hosters with servers in Germany (all-inkl states this for all plans; netcup, Hetzner, manitu are German) avoid the US CLOUD Act question. See [`COST.md`](COST.md#digital-sovereignty).

## Sources

- all-inkl: <https://all-inkl.com/webhosting/uebersicht/>
- Hetzner: <https://www.hetzner.com/webhosting>
- netcup: <https://www.netcup.com/en/hosting/hosting-comparison>; price increase: <https://www.hosttest.de/vergleich/netcup-webhosting.html>
- lima-city: <https://www.lima-city.de/webhosting>
- manitu: <https://www.manitu.de/webhosting/>
- Uberspace: <https://uberspace.de/en/product/>; web backends: <https://manual.uberspace.de/web-backends/>
- webgo: <https://www.webgo.de/webhosting/>
- Infomaniak CGI: <https://infomaniak.com/de/support/faq/1307/problem-mit-perl-oder-python-skript-losen>
