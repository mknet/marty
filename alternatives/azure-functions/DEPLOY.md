# Azure Functions deploy notes (hard-won)

Lessons from standing up [`01_basic`](01_basic/) as a **Rust custom handler** on Linux Consumption (Oct 2026). The justfile encodes these; this page is the “why”.

Marty twin: [`examples/01_basic`](../../examples/01_basic/). AWS cousin: [`../aws-lambda/01_basic`](../aws-lambda/01_basic/).

## What worked in the end

1. Login to the **personal** tenant (`*.onmicrosoft.com`), not a work org.
2. Region **`northeurope`** (not `westeurope` on Free Trial).
3. Register **`Microsoft.Storage`** + **`Microsoft.Web`** before creating storage / Function App.
4. Build **musl x86_64** `handler`, zip `host.json` + `handler` + `basic/`.
5. Set **`FUNCTIONS_WORKER_RUNTIME=Custom`** (capital **C**) and **`linuxFxVersion=DOTNET|8.0`** (custom handlers piggy-back the .NET host stack — [MS custom handlers](https://learn.microsoft.com/en-us/azure/azure-functions/functions-custom-handlers)).
6. Zip-deploy, then **restart** and **sync triggers** if needed.
7. Call `GET https://<app>.azurewebsites.net/api/basic` → `88 miles per hour!`

## Hurdles (in order we hit them)

### 1. Wrong Azure AD tenant / no subscription

`az login` attached to a **work directory** (e.g. EWE); personal mailbox showed **“No subscriptions found”**.

- Use `just login tenant=<GUID|name.onmicrosoft.com>`.
- Confirm with `just whoami` / `az account list`.
- Stale “Kostenlose Testversion” entries can appear in `az account list` whose **tenant no longer exists** (`AADSTS90002`) — ignore or clear; don’t `az account set` to them.

### 2. Resource providers not registered

New / Free Trial subscriptions often have `Microsoft.Storage` / `Microsoft.Web` as **NotRegistered**. Creating a resource group can succeed while **storage create** fails with confusing errors (`SubscriptionNotFound` or similar).

- `just deploy` registers those namespaces and waits until `Registered`.
- App Insights / OperationalInsights may auto-register on first Function App create.

### 3. Free Trial region lock (`locationineligible`)

```
RequestDisallowedByAzure: The selected region is currently not accepting new customers
```

Seen for **`westeurope`**. **`northeurope`** accepted the same Free Trial subscription.

- Default location in the justfile is **`northeurope`**.
- Override: `just deploy location=swedencentral` (or another allowed region).
- Portal hint: [aka.ms/locationineligible](https://aka.ms/locationineligible).

### 4. Resource group already in another location

```
InvalidResourceGroupLocation: Resource group already exists in location 'westeurope'
```

RG location is immutable. After switching default region, an old empty RG blocks recreate.

- `just deploy` deletes the RG if its location ≠ target (then waits).
- Or: `just destroy` and redeploy.

### 5. Broken zip on macOS (`mktemp -t …zip`)

```
zip error: Zip file structure invalid (/var/folders/…/marty-az-XXXXXX.zip.… )
```

`mktemp -t prefix.zip` creates an **empty** file; piping `zip` into that path produced an invalid archive. Azure never got a good package.

- Use a **temp directory** + `deploy.zip` path (see justfile).
- Always `unzip -t` before `config-zip`.

### 6. Zip upload OK, trigger sync fails (`ServiceUnavailable`)

```
Could not detect runtime. To configure linuxFxVersion…
Operation returned an invalid status 'Bad Request'
… Encountered an error (ServiceUnavailable) from host runtime.
```

`az functionapp deployment source config-zip` **uploaded** the blob (`WEBSITE_RUN_FROM_PACKAGE` set) but **syncfunctiontriggers** failed because:

- `linuxFxVersion` was **empty** (`--runtime custom` alone is not enough on Linux Consumption).
- `FUNCTIONS_WORKER_RUNTIME=custom` (lowercase) is weaker than MS docs’ **`Custom`**.

Fix that worked:

```bash
az functionapp config appsettings set -g … -n … \
  --settings FUNCTIONS_WORKER_RUNTIME=Custom FUNCTIONS_EXTENSION_VERSION=~4
az functionapp config set -g … -n … --linux-fx-version 'DOTNET|8.0'
az functionapp restart -g … -n …
# then sync / curl /api/basic
```

`just deploy` applies the same settings after create/zip.

On a **brand-new** Function App, `config-zip` may still print `Bad Request` / `ServiceUnavailable` on **syncfunctiontriggers** even with Fx set: the zip is usually already in `WEBSITE_RUN_FROM_PACKAGE`, and a restart + later sync / HTTP call succeeds. Treat that message as noisy unless `curl …/api/basic` fails.

### 7. Platform noise (not blockers for this sample)

- **Linux Consumption EOL** warning (Sep 2028) → Flex Consumption recommended later; this sample stays on classic Consumption for a simple custom-handler zip.
- Custom handler on Linux Consumption: zip deploy via publish-profile/Kudu has historically **stripped the executable bit**; RBAC/`config-zip` → `WEBSITE_RUN_FROM_PACKAGE` blob is the safer path (what `az` uses here).
- Handler must be **Linux x86_64** (we use musl static) even when building on Apple Silicon.

### 8. Azure CLI: `--output text` is invalid

AWS-style `--output text` fails on `az` (`Allowed values: json, …, tsv, …`). Use **`-o tsv`** for query scalars.

## Quick verify

```bash
just whoami
just deploy
curl -sS "https://$(az functionapp show -g marty-alt-azure -n marty-alt-az-01-basic --query defaultHostName -o tsv)/api/basic"
# expect: 88 miles per hour!
just destroy
```
