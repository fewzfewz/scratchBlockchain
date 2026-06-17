variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "availability_zones" {
  description = "Availability zones"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b", "us-east-1c"]
}

variable "name_prefix" {
  description = "Resource name prefix"
  type        = string
  default     = "modular-testnet"
}

variable "ssh_key_name" {
  description = "EC2 SSH key pair name"
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

variable "validator_instance_type" {
  description = "Validator EC2 instance type"
  type        = string
  default     = "t3.large"
}

variable "rpc_instance_type" {
  description = "RPC node EC2 instance type"
  type        = string
  default     = "t3.xlarge"
}

variable "validator_volume_size" {
  description = "Validator root volume size (GB)"
  type        = number
  default     = 100
}

variable "rpc_volume_size" {
  description = "RPC node root volume size (GB)"
  type        = number
  default     = 200
}

variable "domain_name" {
  description = "Domain name for Route53 records (empty = skip DNS)"
  type        = string
  default     = ""
}

variable "acm_certificate_arn" {
  description = "ACM certificate ARN for HTTPS (required if domain_name is set)"
  type        = string
  default     = ""
}
