# EU shared hosters for Marty

Which shared hosting plans can run a Marty binary as CGI. Prices incl. 19 % German VAT, **checked Oct 2026**; prices change, re-check the tariff pages. Cost comparison with cloud functions: [`COST.md`](COST.md). Product defaults: [`DIRECTION.md`](DIRECTION.md).

**Probe before claiming.** "CGI scripts" in marketing ≠ "arbitrary ELF" on every plan. Run [`tools/hosting-probe`](../tools/hosting-probe/) once on a plan before saying it works. **Verified (Oct 2026):** all-inkl, **Hetzner Webhosting S**, and **netcup EiWoMiSau**. Outside sync/axum/php: [`RESULTS-SNAPSHOTS.md` Run N](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-n--outside-compare-hetzner-s-vs-all-inkl-2026-10-28) (Hetzner vs all-inkl), [Run O](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-o--outside-netcup-eiwomisau-2026-10-29) (netcup).

## Table

| Hoster / plan | Price / month | Term | SSH | CGI with own binary | Notes | [`cargo marty`](../tools/cargo-marty/) preset |
|---|---|---|---|---|---|---|
| **all-inkl** Privat / PrivatPlus / Premium / Business | 4.95 / 7.95 / 9.95 / 24.95 € | none, first month free | from Premium | **verified** (Oct 2026: event + mod_cgid + suexec, Rust musl works, Go dies) | servers only in Germany; MySQL via [`examples/07_mysql`](../examples/07_mysql/) (config file, no `SetEnv`) | `allinkl` |
| **Hetzner Webhosting S** | 1.90 € (1.60 € net) | none | from L (9.90 €, 8.32 € net) | **verified** (Oct 2026: Debian 13, stdout **socket**/mod_cgid-class, site user, KeepAlive + HTTP/2, Rust musl + PHP work) | cheapest verified plan; needs a domain in the package for HTTP (no `*.your-vhost.de` preview); Go CGI not re-probed | `generic` (set `docroot`) |
| **netcup EiWoMiSau** | ~6.45 € | 6 months min / 12 mo billing | yes | **verified** (Oct 2026: Debian 12, nginx front + Apache CGI, stdout **pipe**/mod_cgi-class, site user `hosting*`, KeepAlive + HTTP/2, Rust musl + PHP work) | enable **CGI-Unterstützung** in WCP; upload `httpdocs/cgi-bin/`; no Passenger Python/Node/Ruby for Marty; **calm load OK** (320/min × 10 min all 200, P50 ~73 ms); **C=100 grill** ~72 req/s then **client IP ban** (vs Hetzner ~233 req/s, stayed up) — [Run O](../benchmarks/complex/RESULTS-SNAPSHOTS.md#run-o--outside-netcup-eiwomisau-2026-10-29); cheaper 1000 still **probe** | `netcup` |
| netcup Webhosting 1000 | 2.69 € (2.17 € before the 2026 increase) | 12 months | yes, all plans | not documented; **probe** (EiWoMiSau verified; do not assume same stack) | 2000: 4.03 €, 4000: 8.10 €, 8000: 13.17 €; Python/Node/Ruby only from 4000 | `netcup` after probe |
| lima-city Mini | 3.50 € (12 mo) / 4.50 € (monthly) | 12 or 1 month | yes, no root | not documented; **probe** | | `generic` after probe |
| **manitu** Webhosting M | 5.99 € (promo 1 € first 6 months) | 1 month | yes | **"CGIs for your own applications" explicitly documented**; probe | L 11.99 €, XL 15.99 €, XXL 19.99 €; best unverified candidate | `generic` after probe |
| Uberspace | pay what you want, reference 6 / 9 / 12 € | none, prepaid, first month free | yes | not needed | **different model:** "web backends" run your own long-lived HTTP process (port 1024 to 65535, `uberspace web backend set / --http --port N`), so run Axum directly, no CGI | n/a (no CGI) |
| webgo | from 6.95 € | n/a | not stated | not stated | no advantage | |
| Infomaniak (CH) | n/a | n/a | yes | **CGI module switched off on shared hosting** (security); Perl/Python CGI unsupported | **negative example:** marketing vs reality drift | n/a |
| IONOS, DomainFactory / Host Europe, dogado, Strato | various | | | CGI historically / `AddHandler cgi-script` (IONOS); **probe** | keep on the probe list | `generic` after probe |
| DreamHost, NearlyFreeSpeech.NET (US) | | | | long CGI culture; **probe** | outside the EU | `generic` after probe |

## Reading the table

- **Preset** column: what to put in `marty.deploy.toml` for [`cargo marty deploy`](../tools/cargo-marty/). netcup: `httpdocs/cgi-bin`, secrets in `private/` next to `httpdocs`. all-inkl (verified Nov 2026 with `playground/`): in the KAS point the domain to `/<domain>/htdocs/`, create an FTP user restricted to `/<domain>/`, deploy over FTPS; secrets in `_config/` next to `htdocs`, which the CGI reads through the compiled-in `../../_config/marty.conf`. For Hetzner use `generic` and set `docroot` to the domain's directory as seen from the login; a dedicated preset follows once the paths are written down from a real account.

- **Cheaper than all-inkl?** Yes: **Hetzner S (1.90 €, verified)** and netcup 1000 (2.69 €, still probe). Verified netcup is **EiWoMiSau** (~6.45 €). Hetzner S remains the cheapest verified Marty target.
- **Latency / capacity (same client, Oct 2026):** outside sync P50 **Hetzner ~33 ms · all-inkl ~50–54 ms · netcup ~72–73 ms**. Steady **320/min** works on Hetzner and netcup (P50 ~31 vs ~73 ms). Hard **C=100** grill: Hetzner queues (~233 req/s, stays up); netcup ~72 req/s then upstream **IP block** (~8.4k successful requests before ban). Details: [`RESULTS-SNAPSHOTS.md` Run N / Run O](../benchmarks/complex/RESULTS-SNAPSHOTS.md).
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
