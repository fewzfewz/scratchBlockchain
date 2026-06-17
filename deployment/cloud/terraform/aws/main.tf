terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = { source = "hashicorp/aws", version = "~> 5.0" }
  }
}

provider "aws" {
  region = var.aws_region
}

# VPC
resource "aws_vpc" "testnet" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true
  tags = { Name = "${var.name_prefix}-vpc" }
}

# Public subnets
resource "aws_subnet" "public" {
  count                   = length(var.availability_zones)
  vpc_id                  = aws_vpc.testnet.id
  cidr_block              = cidrsubnet("10.0.0.0/16", 8, count.index)
  availability_zone       = var.availability_zones[count.index]
  map_public_ip_on_launch = true
  tags = { Name = "${var.name_prefix}-subnet-${count.index}" }
}

# Internet gateway
resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.testnet.id
  tags   = { Name = "${var.name_prefix}-igw" }
}

# Route table
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.testnet.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }
  tags = { Name = "${var.name_prefix}-rt" }
}

resource "aws_route_table_association" "public" {
  count          = length(var.availability_zones)
  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

# Security group for validator nodes
resource "aws_security_group" "validator" {
  name        = "${var.name_prefix}-validator-sg"
  description = "Validator node security group"
  vpc_id      = aws_vpc.testnet.id

  ingress {
    description = "P2P from anywhere"
    from_port   = 26656
    to_port     = 26656
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  ingress {
    description = "RPC from load balancer"
    from_port   = 26657
    to_port     = 26657
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.testnet.cidr_block]
  }
  ingress {
    description = "API from load balancer"
    from_port   = 8545
    to_port     = 8545
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.testnet.cidr_block]
  }
  ingress {
    description = "Metrics from Prometheus"
    from_port   = 9090
    to_port     = 9090
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.testnet.cidr_block]
  }
  ingress {
    description = "SSH"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = var.admin_cidr_blocks
  }
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  tags = { Name = "${var.name_prefix}-validator-sg" }
}

# Security group for RPC nodes (public API)
resource "aws_security_group" "rpc" {
  name        = "${var.name_prefix}-rpc-sg"
  description = "RPC node security group"
  vpc_id      = aws_vpc.testnet.id

  ingress {
    description = "API from ALB"
    from_port   = 8545
    to_port     = 8545
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.testnet.cidr_block]
  }
  ingress {
    description = "P2P from validators"
    from_port   = 26656
    to_port     = 26656
    protocol    = "tcp"
    cidr_blocks = [aws_vpc.testnet.cidr_block]
  }
  ingress {
    description = "SSH"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = var.admin_cidr_blocks
  }
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  tags = { Name = "${var.name_prefix}-rpc-sg" }
}

# ALB security group
resource "aws_security_group" "alb" {
  name        = "${var.name_prefix}-alb-sg"
  description = "ALB security group"
  vpc_id      = aws_vpc.testnet.id

  ingress {
    description = "HTTPS from internet"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  ingress {
    description = "HTTP redirect"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
  tags = { Name = "${var.name_prefix}-alb-sg" }
}

# Bootstrap node (first validator, public P2P)
resource "aws_instance" "bootstrap" {
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.validator_instance_type
  subnet_id              = aws_subnet.public[0].id
  vpc_security_group_ids = [aws_security_group.validator.id]
  key_name               = var.ssh_key_name
  associate_public_ip_address = true

  root_block_device {
    volume_type = "gp3"
    volume_size = var.validator_volume_size
  }

  tags = { Name = "${var.name_prefix}-bootstrap" }
}

# Validator nodes
resource "aws_instance" "validator" {
  count                  = var.validator_count
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.validator_instance_type
  subnet_id              = aws_subnet.public[count.index % length(var.availability_zones)].id
  vpc_security_group_ids = [aws_security_group.validator.id]
  key_name               = var.ssh_key_name
  associate_public_ip_address = true

  root_block_device {
    volume_type = "gp3"
    volume_size = var.validator_volume_size
  }

  tags = { Name = "${var.name_prefix}-validator-${count.index + 1}" }
}

# RPC nodes
resource "aws_instance" "rpc" {
  count                  = var.rpc_count
  ami                    = data.aws_ami.ubuntu.id
  instance_type          = var.rpc_instance_type
  subnet_id              = aws_subnet.public[count.index % length(var.availability_zones)].id
  vpc_security_group_ids = [aws_security_group.rpc.id]
  key_name               = var.ssh_key_name
  associate_public_ip_address = true

  root_block_device {
    volume_type = "gp3"
    volume_size = var.rpc_volume_size
  }

  tags = { Name = "${var.name_prefix}-rpc-${count.index + 1}" }
}

# Application Load Balancer
resource "aws_lb" "rpc" {
  name               = "${var.name_prefix}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = aws_subnet.public[*].id
  tags               = { Name = "${var.name_prefix}-alb" }
}

resource "aws_lb_target_group" "rpc" {
  name        = "${var.name_prefix}-tg"
  port        = 8545
  protocol    = "HTTP"
  vpc_id      = aws_vpc.testnet.id
  target_type = "instance"

  health_check {
    path                = "/health"
    interval            = 10
    timeout             = 5
    healthy_threshold   = 2
    unhealthy_threshold = 3
  }
  tags = { Name = "${var.name_prefix}-tg" }
}

resource "aws_lb_target_group_attachment" "rpc" {
  count            = var.rpc_count
  target_group_arn = aws_lb_target_group.rpc.arn
  target_id        = aws_instance.rpc[count.index].id
  port             = 8545
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.rpc.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.acm_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.rpc.arn
  }
}

resource "aws_lb_listener" "http_redirect" {
  load_balancer_arn = aws_lb.rpc.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type = "redirect"
    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

# Route53 DNS
data "aws_route53_zone" "main" {
  count = var.domain_name != "" ? 1 : 0
  name  = var.domain_name
}

resource "aws_route53_record" "rpc" {
  count   = var.domain_name != "" ? 1 : 0
  zone_id = data.aws_route53_zone.main[0].zone_id
  name    = "rpc.${var.domain_name}"
  type    = "A"
  alias {
    name                   = aws_lb.rpc.dns_name
    zone_id                = aws_lb.rpc.zone_id
    evaluate_target_health = true
  }
}

resource "aws_route53_record" "bootstrap" {
  count   = var.domain_name != "" ? 1 : 0
  zone_id = data.aws_route53_zone.main[0].zone_id
  name    = "bootstrap.${var.domain_name}"
  type    = "A"
  ttl     = 60
  records = [aws_instance.bootstrap.public_ip]
}

data "aws_ami" "ubuntu" {
  most_recent = true
  owners      = ["099720109477"]
  filter {
    name   = "name"
    values = ["ubuntu/images/hvm-ssd-gp3/ubuntu-22.04-amd64-server-*"]
  }
}
