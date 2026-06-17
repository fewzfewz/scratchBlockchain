variable "azure_subscription_id" {
  description = "Azure subscription ID"
  type        = string
}

variable "azure_location" {
  description = "Azure region"
  type        = string
  default     = "eastus"
}

variable "name_prefix" {
  description = "Resource name prefix"
  type        = string
  default     = "modular-testnet"
}

variable "ssh_public_key_path" {
  description = "Path to SSH public key file (~/.ssh/id_rsa.pub)"
  type        = string
}

variable "admin_cidr_blocks" {
  description = "CIDR blocks allowed for SSH access"
  type        = list(string)
  default     = ["0.0.0.0/0"]
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

variable "validator_vm_size" {
  description = "Validator VM size"
  type        = string
  default     = "Standard_D4s_v5"
}

variable "rpc_vm_size" {
  description = "RPC node VM size"
  type        = string
  default     = "Standard_D8s_v5"
}

variable "validator_volume_size" {
  description = "Validator OS disk size (GB)"
  type        = number
  default     = 100
}

variable "rpc_volume_size" {
  description = "RPC node OS disk size (GB)"
  type        = number
  default     = 200
}

variable "domain_name" {
  description = "Domain name for Azure DNS (empty = skip DNS)"
  type        = string
  default     = ""
}
