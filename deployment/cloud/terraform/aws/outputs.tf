output "bootstrap_public_ip" {
  description = "Bootstrap node public IP"
  value       = aws_instance.bootstrap.public_ip
}

output "validator_public_ips" {
  description = "Validator node public IPs"
  value       = aws_instance.validator[*].public_ip
}

output "rpc_public_ips" {
  description = "RPC node public IPs"
  value       = aws_instance.rpc[*].public_ip
}

output "alb_dns_name" {
  description = "Load balancer DNS name"
  value       = aws_lb.rpc.dns_name
}

output "rpc_endpoint" {
  description = "Public RPC endpoint"
  value       = var.domain_name != "" ? "https://rpc.${var.domain_name}" : "http://${aws_lb.rpc.dns_name}"
}

output "bootstrap_peer_id" {
  description = "Bootstrap node multiaddr (set after Ansible deploy)"
  value       = "/dns4/${var.domain_name != "" ? "bootstrap.${var.domain_name}" : aws_instance.bootstrap.public_ip}/tcp/26656"
}
