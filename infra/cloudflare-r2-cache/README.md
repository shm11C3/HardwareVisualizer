# sccache R2 cache store

Provisions the Cloudflare R2 bucket and scoped API credentials that
`.github/actions/cache-duckdb/action.yml` uses as sccache's storage backend,
replacing the GitHub Actions cache (`actions/cache`) wrapper that PR #2132
originally shipped. Background and the reasoning for moving off GitHub's
cache entirely is in
[`docs/development/local-build-cache.md`](../../docs/development/local-build-cache.md#ci-caches-the-duckdb-c-build-with-sccache-backed-by-cloudflare-r2).

State is local (`terraform.tfstate`, gitignored) since this is applied by
hand, not from CI, and the state file contains the derived secret access
key. Keep it somewhere durable (e.g. sync the whole `infra/cloudflare-r2-cache`
directory to a private location, or move to a remote backend such as R2
itself via the `s3` backend type) rather than only on one machine.

## Prerequisites

- [Terraform](https://developer.hashicorp.com/terraform/install) >= 1.7
- A Cloudflare account with an R2 subscription enabled (R2 has its own
  opt-in on the dashboard; the free tier is enough for this bucket)
- A Cloudflare API token scoped to the target account with `Account API
  Tokens Read`, `Account API Tokens Write`, and R2 admin (`Workers R2
  Storage`) permissions, used only to run this Terraform config (distinct
  from the account-scoped token Terraform creates for CI). The account-
  scoped variants matter: the account-level permission-groups lookup this
  config uses returns 403 for a token that only has the user-scoped `API
  Tokens Read`/`API Tokens Write` permissions.

## Apply

```bash
cd infra/cloudflare-r2-cache
cp terraform.tfvars.example terraform.tfvars
# edit terraform.tfvars: set account_id

export TF_VAR_cloudflare_api_token="<your-personal-cloudflare-api-token>"

terraform init
terraform plan
terraform apply
```

## Wire up GitHub Actions

After `apply` succeeds, set the two credential secrets (Settings, Secrets
and variables, Actions, Secrets) from the outputs. Nothing here should be
pasted into chat, a PR, or a commit; copy directly from your terminal to
the GitHub UI or `gh secret set`:

```bash
gh secret set AWS_ACCESS_KEY_ID --repo shm11C3/HardwareVisualizer \
  --body "$(terraform output -raw access_key_id)"

gh secret set AWS_SECRET_ACCESS_KEY --repo shm11C3/HardwareVisualizer \
  --body "$(terraform output -raw secret_access_key)"
```

Also set the account ID as a repository **variable** (Settings, Secrets and
variables, Actions, Variables), not a secret; it identifies the R2 endpoint
but is not sensitive on its own:

```bash
gh variable set CLOUDFLARE_ACCOUNT_ID --repo shm11C3/HardwareVisualizer \
  --body "<your-cloudflare-account-id>"
```

`bucket_name` is not a secret; it's already hardcoded into
`.github/actions/cache-duckdb/action.yml`. If you change `bucket_name` in
`terraform.tfvars`, update that file to match.

## Rotating the CI token

Re-running `apply` after tainting the token resource issues a new token
and rotates both derived credentials in one step:

```bash
terraform taint cloudflare_account_token.sccache_ci
terraform apply
# then re-run the gh secret set commands above with the new outputs
```

The token has no `expires_on`, so rotation is manual/on-demand rather than
forced. If you want it to expire automatically as a defense-in-depth
measure, add `expires_on` to the `cloudflare_account_token` resource in
`main.tf` and re-apply before it lapses — an expired unrotated token will
fail CI's R2 auth the same way an unset secret does (sccache falls back to
building without a cache rather than failing the build).

## Changing retention

`object_expiry_days` (default 30) controls how long an untouched cache
object survives before R2 deletes it. Lower it if the bucket grows faster
than expected; there's no cost alert wired up here, so check usage
occasionally from the Cloudflare dashboard's R2 overview.
