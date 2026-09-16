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

variable "bucket_jurisdiction" {
  description = <<-DESC
    R2 bucket jurisdiction, distinct from `bucket_location` (a geographic
    hint): this is the legal/regulatory jurisdiction objects are guaranteed
    to stay within, and it is also the segment the CI token's bucket-scoped
    resource key is built from
    (com.cloudflare.edge.r2.bucket.<account_id>_<jurisdiction>_<bucket>).
    One of: default, eu, fedramp, us. "default" is correct unless you have
    a specific data-residency requirement.
  DESC
  type    = string
  default = "default"
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
