output "bucket_name" {
  description = "R2 bucket name. Set as SCCACHE_BUCKET in ci.yml (not a secret)."
  value       = cloudflare_r2_bucket.sccache.name
}

output "endpoint" {
  description = "S3-compatible endpoint. Set as SCCACHE_ENDPOINT in ci.yml (not a secret)."
  value       = "https://${var.account_id}.r2.cloudflarestorage.com"
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
