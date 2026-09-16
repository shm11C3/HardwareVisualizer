variable "cloudflare_api_token" {
  description = <<-DESC
    A Cloudflare API token used only to run this Terraform config, with at
    least "API Tokens Read/Write" and "Workers R2 Storage" permissions on
    the target account. Set via TF_VAR_cloudflare_api_token, not a .tfvars
    file, so it never touches disk or version control.
  DESC
  type        = string
  sensitive   = true
}

variable "account_id" {
  description = "Cloudflare account ID that owns the R2 bucket."
  type        = string
}

variable "bucket_name" {
  description = "Name of the R2 bucket sccache reads and writes to."
  type        = string
  default     = "hardwarevisualizer-sccache"
}

variable "bucket_location" {
  description = <<-DESC
    R2 bucket location hint. Only honored the first time a bucket with this
    name is created; changing it later does not move an existing bucket.
    One of: apac, eeur, enam, weur, wnam, oc.
  DESC
  type    = string
  default = "wnam"
}

variable "object_expiry_days" {
  description = <<-DESC
    Delete cache objects untouched for this many days. Mirrors the eviction
    GitHub's Actions cache did implicitly; R2 has no built-in LRU eviction,
    so without this the bucket grows without bound.
  DESC
  type    = number
  default = 30
}
