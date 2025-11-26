# Cloud Deployment Guide

Migrate your local testnet to cloud infrastructure (AWS/GCP/Azure).

## Overview

The local Docker Compose setup is designed to be cloud-ready. This guide shows how to deploy to production cloud infrastructure.

## Architecture

### Local (Current)
```
Docker Compose on single machine
├── 3 Validators
├── 2 RPC nodes
├── Monitoring stack
└── Faucet
```

### Cloud (Target)
```
Multi-region deployment
├── 3 Validator VMs (US, EU, Asia)
├── 2 RPC VMs (load balanced)
├── Monitoring cluster
├── Faucet service
└── CDN + DDoS protection
```

## Option 1: AWS Deployment

### Prerequisites
- AWS Account
- AWS CLI configured
- Terraform installed (optional)

### Step 1: Create VPC and Security Groups

```bash
# Create VPC
aws ec2 create-vpc --cidr-block 10.0.0.0/16

# Create security group
aws ec2 create-security-group \
  --group-name modular-testnet \
  --description "Modular Blockchain Testnet" \
  --vpc-id vpc-xxxxx

# Allow P2P (26656)
aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxx \
  --protocol tcp \
  --port 26656 \
  --cidr 0.0.0.0/0

# Allow RPC (26657)
aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxx \
  --protocol tcp \
  --port 26657 \
  --cidr 0.0.0.0/0

# Allow API (8545)
aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxx \
  --protocol tcp \
  --port 8545 \
  --cidr 0.0.0.0/0

# Allow Metrics (9090)
aws ec2 authorize-security-group-ingress \
  --group-id sg-xxxxx \
  --protocol tcp \
  --port 9090 \
  --cidr 10.0.0.0/16  # Internal only
```

### Step 2: Launch EC2 Instances

```bash
# Validator instances (t3.large, 4 vCPU, 8GB RAM)
for i in 1 2 3; do
  aws ec2 run-instances \
    --image-id ami-0c55b159cbfafe1f0 \
    --instance-type t3.large \
    --key-name your-key \
    --security-group-ids sg-xxxxx \
    --subnet-id subnet-xxxxx \
    --block-device-mappings '[{"DeviceName":"/dev/sda1","Ebs":{"VolumeSize":100}}]' \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=validator$i}]"
done

# RPC instances (t3.xlarge, 8 vCPU, 16GB RAM)
for i in 1 2; do
  aws ec2 run-instances \
    --image-id ami-0c55b159cbfafe1f0 \
    --instance-type t3.xlarge \
    --key-name your-key \
    --security-group-ids sg-xxxxx \
    --subnet-id subnet-xxxxx \
    --block-device-mappings '[{"DeviceName":"/dev/sda1","Ebs":{"VolumeSize":200}}]' \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=rpc$i}]"
done
```

### Step 3: Deploy Using Ansible

Create `ansible/inventory.yml`:
```yaml
all:
  children:
    validators:
      hosts:
        validator1:
          ansible_host: 1.2.3.4
        validator2:
          ansible_host: 1.2.3.5
        validator3:
          ansible_host: 1.2.3.6
    rpc:
      hosts:
        rpc1:
          ansible_host: 1.2.3.7
        rpc2:
          ansible_host: 1.2.3.8
```

Create `ansible/deploy.yml`:
```yaml
---
- name: Deploy Modular Blockchain
  hosts: all
  become: yes
  tasks:
    - name: Install Docker
      apt:
        name: docker.io
        state: present
        update_cache: yes

    - name: Copy node binary
      copy:
        src: ../target/release/modular-node
        dest: /usr/local/bin/
        mode: '0755'

    - name: Copy config
      template:
        src: config.toml.j2
        dest: /etc/modular/config.toml

    - name: Copy genesis
      copy:
        src: ../deployment/local/configs/genesis.json
        dest: /etc/modular/genesis.json

    - name: Create systemd service
      copy:
        content: |
          [Unit]
          Description=Modular Blockchain Node
          After=network.target

          [Service]
          Type=simple
          User=modular
          ExecStart=/usr/local/bin/modular-node start --config /etc/modular/config.toml
          Restart=on-failure
          RestartSec=10
          LimitNOFILE=65535

          [Install]
          WantedBy=multi-user.target
        dest: /etc/systemd/system/modular-node.service

    - name: Start service
      systemd:
        name: modular-node
        state: started
        enabled: yes
        daemon_reload: yes
```

Run deployment:
```bash
ansible-playbook -i ansible/inventory.yml ansible/deploy.yml
```

### Step 4: Set Up Load Balancer

```bash
# Create Application Load Balancer
aws elbv2 create-load-balancer \
  --name modular-rpc-lb \
  --subnets subnet-xxxxx subnet-yyyyy \
  --security-groups sg-xxxxx

# Create target group
aws elbv2 create-target-group \
  --name modular-rpc-targets \
  --protocol HTTP \
  --port 8545 \
  --vpc-id vpc-xxxxx

# Register RPC nodes
aws elbv2 register-targets \
  --target-group-arn arn:aws:... \
  --targets Id=i-xxxxx Id=i-yyyyy
```

### Step 5: Configure DNS

```bash
# Create Route53 hosted zone
aws route53 create-hosted-zone \
  --name testnet.modular.io \
  --caller-reference $(date +%s)

# Add A record for RPC
aws route53 change-resource-record-sets \
  --hosted-zone-id Z123456 \
  --change-batch '{
    "Changes": [{
      "Action": "CREATE",
      "ResourceRecordSet": {
        "Name": "rpc.testnet.modular.io",
        "Type": "A",
        "AliasTarget": {
          "HostedZoneId": "Z123456",
          "DNSName": "modular-rpc-lb-123.us-east-1.elb.amazonaws.com",
          "EvaluateTargetHealth": true
        }
      }
    }]
  }'
```

## Option 2: GCP Deployment

### Using Terraform

Create `terraform/main.tf`:
```hcl
provider "google" {
  project = "modular-blockchain"
  region  = "us-central1"
}

# Validator instances
resource "google_compute_instance" "validator" {
  count        = 3
  name         = "validator-${count.index + 1}"
  machine_type = "n1-standard-4"
  zone         = "us-central1-a"

  boot_disk {
    initialize_params {
      image = "ubuntu-2004-lts"
      size  = 100
    }
  }

  network_interface {
    network = "default"
    access_config {}
  }

  metadata_startup_script = file("startup.sh")

  tags = ["validator", "blockchain"]
}

# RPC instances
resource "google_compute_instance" "rpc" {
  count        = 2
  name         = "rpc-${count.index + 1}"
  machine_type = "n1-standard-8"
  zone         = "us-central1-a"

  boot_disk {
    initialize_params {
      image = "ubuntu-2004-lts"
      size  = 200
    }
  }

  network_interface {
    network = "default"
    access_config {}
  }

  metadata_startup_script = file("startup.sh")

  tags = ["rpc", "blockchain"]
}

# Load balancer
resource "google_compute_global_forwarding_rule" "rpc" {
  name       = "rpc-lb"
  target     = google_compute_target_http_proxy.rpc.id
  port_range = "80"
}
```

Deploy:
```bash
cd terraform
terraform init
terraform plan
terraform apply
```

## Option 3: Docker Swarm (Easiest)

### Step 1: Initialize Swarm

```bash
# On manager node
docker swarm init --advertise-addr <MANAGER-IP>

# On worker nodes
docker swarm join --token <TOKEN> <MANAGER-IP>:2377
```

### Step 2: Deploy Stack

```bash
# Copy docker-compose.yml to manager
scp deployment/local/docker-compose.yml manager:/root/

# Deploy
docker stack deploy -c docker-compose.yml modular
```

### Step 3: Scale Services

```bash
# Scale RPC nodes
docker service scale modular_rpc1=3

# Update service
docker service update --image modular-node:v2 modular_validator1
```

## Monitoring Setup

### Deploy Prometheus + Grafana

```bash
# Create monitoring stack
docker stack deploy -c monitoring/docker-compose.yml monitoring

# Access Grafana
https://monitor.testnet.modular.io
```

### Configure Alerts

Edit `monitoring/prometheus/alerts.yml` and add Slack webhook:

```yaml
receivers:
  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#alerts'
        title: 'Testnet Alert'
```

## SSL/TLS Setup

### Using Certbot

```bash
# Install Certbot
sudo apt-get install certbot

# Get certificate
sudo certbot certonly --standalone \
  -d rpc.testnet.modular.io \
  -d faucet.testnet.modular.io \
  -d monitor.testnet.modular.io

# Auto-renewal
sudo crontab -e
# Add: 0 0 * * * certbot renew --quiet
```

### Using Cloudflare

1. Add domain to Cloudflare
2. Update nameservers
3. Enable SSL (Full)
4. Enable DDoS protection
5. Set up page rules for caching

## Cost Estimates

### AWS (Monthly)
- 3 × t3.large validators: $150
- 2 × t3.xlarge RPC: $300
- Load balancer: $20
- Data transfer: $50
- **Total**: ~$520/month

### GCP (Monthly)
- 3 × n1-standard-4: $180
- 2 × n1-standard-8: $360
- Load balancer: $18
- Data transfer: $50
- **Total**: ~$608/month

### DigitalOcean (Cheapest)
- 3 × 4GB droplets: $72
- 2 × 8GB droplets: $96
- Load balancer: $12
- **Total**: ~$180/month

## Migration Checklist

- [ ] Choose cloud provider
- [ ] Create cloud account
- [ ] Set up VPC/networking
- [ ] Launch instances
- [ ] Deploy nodes
- [ ] Configure load balancer
- [ ] Set up DNS
- [ ] Configure SSL
- [ ] Deploy monitoring
- [ ] Set up alerts
- [ ] Test endpoints
- [ ] Update documentation
- [ ] Announce migration

## Rollback Plan

If cloud deployment fails:

1. Keep local testnet running
2. Update DNS back to local IP
3. Debug cloud issues
4. Retry deployment

## Support

- AWS: https://aws.amazon.com/support
- GCP: https://cloud.google.com/support
- Discord: #cloud-deployment
