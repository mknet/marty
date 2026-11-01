# EU shared hosters for Marty

Which shared hosting plans can run a Marty binary as CGI. Prices incl. 19 % German VAT, **checked Oct 2026**; prices change, re-check the tariff pages. Cost comparison with cloud functions: [`COST.md`](COST.md). Product defaults: [`DIRECTION.md`](DIRECTION.md). **Italian hosters** are in their own section, [Italy](#italy), **checked Nov 2026**; their net prices are converted the same way (net × 1.19, German VAT) so the columns compare.

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
- **Sovereignty:** German hosters with servers in Germany (all-inkl states this for all plans; netcup, Hetzner, manitu are German) avoid the US CLOUD Act question. Aruba, Serverplan and Tophost name data centres in Italy, so they are EU-hosted too, without a CLOUD Act link. See [`COST.md`](COST.md#digital-sovereignty).

## Italy

Italian shared hosters with CGI in their docs or plan pages, **checked Nov 2026**. **None is verified** with [`tools/hosting-probe`](../tools/hosting-probe/) yet, so every row stays **probe**. Prices: Italian net price × 1.19 (German VAT, as in the main table), per month; the **renewal price** is the headline, first-year promos are only noted.

| Hoster / plan | Price / month | Term | SSH | CGI with own binary | Notes | [`cargo marty`](../tools/cargo-marty/) preset |
|---|---|---|---|---|---|---|
| **Aruba** Hosting Linux Basic / Easy / Advanced | ~3.47 / 5.95 / 7.93 € (41.64 / 71.39 / 95.19 €/yr; first year 9.90 / 9.90 / 19.90 € net) | 12 months | only Advanced | **documented:** official guide says Perl/Python/Ruby/shell **and executables (`.exe`, `.cgi`)** go in `cgi-bin/`, CHMOD 755; older KB: upload C CGIs "already compiled as a binary"; CGI test in the panel; **probe** | data centres in Italy; `.htaccess` with mod_rewrite/deflate/expires; PHP runs as CGI; renewal 34.99 / 59.99 / 79.99 € net/yr; best Italian candidate | `generic` after probe |
| **Keliweb** KeliUser / KeliPRO / KeliCMS / KeliTOP | KeliPRO ~5.45 € on renewal (4.58 € net); promo ~3.26 / 2.73 / 4.11 / 12.39 € (2.74 / 2.29 / 3.45 / 10.41 € net) | 12 months billing | "Accesso SSH" listed, per plan unclear | **documented:** comparison table lists **CGI**, Perl per FAQ; **probe** | cPanel (KeliUser: DirectAdmin); renewal prices of the other plans not stated | `generic` after probe |
| **Netsons** Shared Business / Pro Premium | Business ~11.89 € (9.99 € net); promo ~7.13 € (5.99 € net) | n/a | yes (SSH, SFTP, Git) | **documented:** plan pages list "Supporto CGI"; **probe** | cPanel, no root; ACN-qualified cloud; site returns 403 to bots, data from search results | `generic` after probe |
| Serverplan Starterkit / Startup / Enterprise | ~2.58 / 7.54 / 14.08 € (26 / 76 / 142 € net/yr) | 12 months | from Startup | not stated (Perl/Python listed); **probe** | cPanel; own data centre in Italy (Cassino) | `generic` after probe |
| Register.it Web Hosting | from ~6.32 € (5.31 € net) | n/a | not stated | not stated (Linux hosting "for PHP, Python, Perl, Ruby"); **probe** | own panel or cPanel/Plesk | `generic` after probe |
| SupportHost Condiviso 1 / 2 / 3 / 4 | 38 / 65 / 99 / 164 €/yr as listed (VAT treatment unclear; if net: ~3.77 / 6.45 / 9.82 / 16.26 €) | 12 months | yes, all plans | not stated, but cPanel; **probe** | 14-day trial, enough for a probe run | `generic` after probe |
| Tophost | from ~0.59 € (5.99 € net/yr) | 12 months | not stated | no CGI evidence; "FastCGI" only for PHP | cheap, but **no Marty candidate** | n/a |

### Italy at a glance

- **Aruba is the strongest unverified Italian candidate:** it explicitly documents your own executables in `cgi-bin/`, has a CGI test in the panel, and runs its data centres in Italy. SSH only from Advanced, so Basic/Easy deploy over FTP, which fits [`cargo marty deploy`](../tools/cargo-marty/) over FTP/FTPS.
- **Next: Keliweb and Netsons**, which list CGI on their cPanel plans.
- cPanel hosters usually ship a `cgi-bin/`; whether a static musl ELF actually runs there only the probe shows.
- **Nothing in Italy is verified yet.** Watch renewal prices: Aruba's 9.90 € is first year only.

## Sources

- all-inkl: <https://all-inkl.com/webhosting/uebersicht/>
- Hetzner: <https://www.hetzner.com/webhosting>
- netcup: <https://www.netcup.com/en/hosting/hosting-comparison>; price increase: <https://www.hosttest.de/vergleich/netcup-webhosting.html>
- lima-city: <https://www.lima-city.de/webhosting>
- manitu: <https://www.manitu.de/webhosting/>
- Uberspace: <https://uberspace.de/en/product/>; web backends: <https://manual.uberspace.de/web-backends/>
- webgo: <https://www.webgo.de/webhosting/>
- Infomaniak CGI: <https://infomaniak.com/de/support/faq/1307/problem-mit-perl-oder-python-skript-losen>
- Aruba plans: <https://hosting.aruba.it/en/web-hosting/>; CGI / `cgi-bin`: <https://guide.aruba.it/faq/hosting-e-domini/linguaggi-e-spazio-web-linux>; CGI test: <https://guide.aruba.it/hosting-e-domini/hosting/hosting-linux/pannello-controllo-linux/gestione-test-cgi-linux>
- Keliweb: <https://www.keliweb.it/hosting>
- Netsons: <https://www.netsons.com/hosting-cpanel>
- Serverplan: <https://www.serverplan.com/hosting/>
- Register.it: <https://www.register.it/hosting/>
- SupportHost: <https://supporthost.com/it/hosting-condiviso/>
- Tophost: <https://www.tophost.it/>
