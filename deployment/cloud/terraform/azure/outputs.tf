output "bootstrap_public_ip" {
  description = "Bootstrap node public IP address"
  value       = azurerm_public_ip.bootstrap.ip_address
}

output "validator_public_ips" {
  description = "Validator node public IP addresses"
  value       = azurerm_public_ip.validator[*].ip_address
}

output "rpc_public_ips" {
  description = "RPC node public IP addresses"
  value       = azurerm_public_ip.rpc[*].ip_address
}

output "lb_public_ip" {
  description = "Load balancer public IP address"
  value       = azurerm_public_ip.lb.ip_address
}

output "rpc_endpoint" {
  description = "Public RPC endpoint"
  value       = var.domain_name != "" ? "https://rpc.${var.domain_name}" : "http://${azurerm_public_ip.lb.ip_address}"
}

output "bootstrap_peer_id" {
  description = "Bootstrap node multiaddr (update after Ansible deploy)"
  value       = "/dns4/${var.domain_name != "" ? "bootstrap.${var.domain_name}" : azurerm_public_ip.bootstrap.ip_address}/tcp/26656"
}

output "resource_group_name" {
  description = "Azure resource group name"
  value       = azurerm_resource_group.testnet.name
}

output "lb_private_ip" {
  description = "Load balancer private IP address"
  value       = azurerm_lb.rpc.private_ip_address
}
