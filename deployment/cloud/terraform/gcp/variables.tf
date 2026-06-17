variable "gcp_project_id" {
  description = "GCP project ID"
  type        = string
}

variable "gcp_region" {
  description = "GCP region"
  type        = string
  default     = "us-central1"
}

variable "gcp_zone" {
  description = "GCP zone"
  type        = string
  default     = "us-central1-a"
}

variable "name_prefix" {
  description = "Resource name prefix"
  type        = string
  default     = "modular-testnet"
}

variable "ssh_public_key_path" {
  description = "Path to SSH public key file"
  type        = string
}

variable "validator_count" {
  description = "Number of validator nodes"
  type        = number
  default     = 4
}

variable "rpc_count" {
  description = "Number of public RPC nodes"
  type        = number
  default     = 2
}

variable "validator_machine_type" {
  description = "Validator GCP machine type"
  type        = string
  default     = "n1-standard-4"
}

variable "rpc_machine_type" {
  description = "RPC node GCP machine type"
  type        = string
  default     = "n1-standard-8"
}

variable "validator_volume_size" {
  description = "Validator disk size (GB)"
  type        = number
  default     = 100
}

variable "rpc_volume_size" {
  description = "RPC node disk size (GB)"
  type        = number
  default     = 200
}

variable "domain_name" {
  description = "Domain name for DNS (empty = skip DNS)"
  type        = string
  default     = ""
}

variable "ssl_private_key" {
  description = "SSL private key content (required if domain_name is set)"
  type        = string
  default     = ""
}

variable "ssl_certificate" {
  description = "SSL certificate content (required if domain_name is set)"
  type        = string
  default     = ""
}
