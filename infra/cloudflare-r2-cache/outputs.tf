output "bucket_name" {
  description = "R2 bucket name. Set as the CLOUDFLARE_R2_BUCKET repository variable (not a secret)."
  value       = cloudflare_r2_bucket.sccache.name
}

output "endpoint" {
  description = <<-DESC
    S3-compatible endpoint. Set as the CLOUDFLARE_R2_ENDPOINT repository
    variable (not a secret). Cloudflare requires a jurisdiction-specific
    hostname once bucket_jurisdiction is not "default"
    (https://developers.cloudflare.com/r2/api/tokens/); computed here so
    callers never reconstruct it and never have to keep a second copy of
    this branch in sync.
  DESC
  value = (
    var.bucket_jurisdiction == "default"
    ? "https://${var.account_id}.r2.cloudflarestorage.com"
    : "https://${var.account_id}.${var.bucket_jurisdiction}.r2.cloudflarestorage.com"
  )
}

output "access_key_id" {
  description = "R2 Access Key ID. Store as the AWS_ACCESS_KEY_ID repository secret."
  value       = cloudflare_account_token.sccache_ci.id
}

output "secret_access_key" {
  description = "R2 Secret Access Key. Store as the AWS_SECRET_ACCESS_KEY repository secret."
  value       = sha256(cloudflare_account_token.sccache_ci.value)
  sensitive   = true
}
