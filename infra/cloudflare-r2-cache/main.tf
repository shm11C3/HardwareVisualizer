provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

# --- Bucket ---------------------------------------------------------------

resource "cloudflare_r2_bucket" "sccache" {
  account_id     = var.account_id
  name           = var.bucket_name
  location       = var.bucket_location
  storage_class  = "Standard"
}

# sccache's own object keys are content-addressed (hash of preprocessed
# source, flags, and compiler); there is no request-side TTL. Without this,
# the bucket grows without bound the way the GitHub Actions cache did before
# #2132's cleanup. R2 has no LRU eviction of its own, so time-based expiry is
# the only backstop. max_age is in seconds (per this resource's schema),
# unlike R2's dashboard, which displays the same underlying field in days.
resource "cloudflare_r2_bucket_lifecycle" "sccache_expiry" {
  account_id  = var.account_id
  bucket_name = cloudflare_r2_bucket.sccache.name

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
# Scope: this token is granted R2 read/write for the whole account (the
# permission groups below are inherently account-scoped; R2 does not expose
# a per-bucket resource ARN through this API), not narrowed to `bucket_name`
# alone. If this account later holds other R2 buckets that CI must not
# touch, split those into a separate Cloudflare account.

data "cloudflare_api_token_permission_groups_list" "all" {}

locals {
  r2_read_permission_group_id = one([
    for g in data.cloudflare_api_token_permission_groups_list.all.result :
    g.id if g.name == "Workers R2 Storage Bucket Item Read"
  ])
  r2_write_permission_group_id = one([
    for g in data.cloudflare_api_token_permission_groups_list.all.result :
    g.id if g.name == "Workers R2 Storage Bucket Item Write"
  ])
}

resource "cloudflare_api_token" "sccache_ci" {
  name = "hardwarevisualizer-ci-sccache-r2"

  policies = [{
    effect = "allow"
    permission_groups = [
      { id = local.r2_read_permission_group_id },
      { id = local.r2_write_permission_group_id },
    ]
    resources = jsonencode({
      "com.cloudflare.api.account.${var.account_id}" = "*"
    })
  }]
}
