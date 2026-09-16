provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

# --- Bucket ---------------------------------------------------------------

resource "cloudflare_r2_bucket" "sccache" {
  account_id    = var.account_id
  name          = var.bucket_name
  location      = var.bucket_location
  jurisdiction  = var.bucket_jurisdiction
  storage_class = "Standard"
}

# sccache's own object keys are content-addressed (hash of preprocessed
# source, flags, and compiler); there is no request-side TTL. Without this,
# the bucket grows without bound the way the GitHub Actions cache did before
# #2132's cleanup. R2 has no LRU eviction of its own, so time-based expiry is
# the only backstop. max_age is in seconds (per this resource's schema),
# unlike R2's dashboard, which displays the same underlying field in days.
resource "cloudflare_r2_bucket_lifecycle" "sccache_expiry" {
  account_id   = var.account_id
  bucket_name  = cloudflare_r2_bucket.sccache.name
  jurisdiction = var.bucket_jurisdiction

  rules = [{
    id      = "expire-untouched-objects"
    enabled = true
    conditions = {
      prefix = ""
    }
    delete_objects_transition = {
      condition = {
        type    = "Age"
        max_age = var.object_expiry_days * 86400
      }
    }
  }]
}

# --- CI credentials ---------------------------------------------------------
#
# R2's S3-compatible Access Key ID / Secret Access Key are derived from a
# Cloudflare API token, not issued directly:
#   Access Key ID     = the token's id
#   Secret Access Key = SHA-256(the token's value)
# (https://developers.cloudflare.com/r2/api/tokens/)
#
# Uses the account-scoped token resource/data source (cloudflare_account_token
# / cloudflare_account_api_token_permission_groups_list), not the user-scoped
# ones (cloudflare_api_token / cloudflare_api_token_permission_groups_list) an
# earlier version of this file used. The user-scoped permission-groups lookup
# hits GET /client/v4/user/tokens/permission_groups, which requires
# user-level authentication (a Global API Key, or a token carrying the
# separate "User API Tokens" permission) and returns 403 for an ordinary
# account-scoped token — exactly the credential this Terraform config asks
# for in its README. The account-scoped equivalent only needs "Account API
# Tokens Read/Write" on this one account, matching that token.
#
# Scope: the created token's `resources` map is a bucket-scoped resource key
# (com.cloudflare.edge.r2.bucket.<account_id>_<jurisdiction>_<bucket_name>,
# documented at the URL above), not an account-wide wildcard. This limits
# the credentials injected into every duckdb-archive CI job to this one
# bucket: a leaked secret or a compromised workflow cannot read or overwrite
# any other R2 bucket this Cloudflare account might hold.

data "cloudflare_account_api_token_permission_groups_list" "all" {
  account_id = var.account_id
}

locals {
  r2_read_permission_group_id = one([
    for g in data.cloudflare_account_api_token_permission_groups_list.all.result :
    g.id if g.name == "Workers R2 Storage Bucket Item Read"
  ])
  r2_write_permission_group_id = one([
    for g in data.cloudflare_account_api_token_permission_groups_list.all.result :
    g.id if g.name == "Workers R2 Storage Bucket Item Write"
  ])
  r2_bucket_resource_key = "com.cloudflare.edge.r2.bucket.${var.account_id}_${var.bucket_jurisdiction}_${cloudflare_r2_bucket.sccache.name}"
}

resource "cloudflare_account_token" "sccache_ci" {
  account_id = var.account_id
  name       = "hardwarevisualizer-ci-sccache-r2"

  policies = [{
    effect = "allow"
    permission_groups = [
      { id = local.r2_read_permission_group_id },
      { id = local.r2_write_permission_group_id },
    ]
    resources = jsonencode({
      (local.r2_bucket_resource_key) = "*"
    })
  }]
}
