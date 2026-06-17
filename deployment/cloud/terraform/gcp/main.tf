terraform {
  required_version = ">= 1.5"
  required_providers {
    google = { source = "hashicorp/google", version = "~> 5.0" }
  }
}

provider "google" {
  project = var.gcp_project_id
  region  = var.gcp_region
}

# VPC
resource "google_compute_network" "testnet" {
  name                    = "${var.name_prefix}-vpc"
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "public" {
  name          = "${var.name_prefix}-subnet"
  network       = google_compute_network.testnet.id
  region        = var.gcp_region
  ip_cidr_range = "10.0.0.0/24"
}

# Firewall rules
resource "google_compute_firewall" "validator" {
  name    = "${var.name_prefix}-validator-fw"
  network = google_compute_network.testnet.name

  allow { protocol = "tcp", ports = ["26656", "26657", "8545", "9090"] }
  allow { protocol = "tcp", ports = ["22"] }
  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["validator"]
}

resource "google_compute_firewall" "rpc" {
  name    = "${var.name_prefix}-rpc-fw"
  network = google_compute_network.testnet.name

  allow { protocol = "tcp", ports = ["8545"] }
  allow { protocol = "tcp", ports = ["22"] }
  source_ranges = ["0.0.0.0/0"]
  target_tags   = ["rpc"]
}

resource "google_compute_firewall" "lb_health" {
  name    = "${var.name_prefix}-lb-hc-fw"
  network = google_compute_network.testnet.name

  allow { protocol = "tcp", ports = ["8545"] }
  source_ranges = ["130.211.0.0/22", "35.191.0.0/16"]
  target_tags   = ["rpc"]
}

# Bootstrap node
resource "google_compute_instance" "bootstrap" {
  name         = "${var.name_prefix}-bootstrap"
  machine_type = var.validator_machine_type
  zone         = var.gcp_zone

  tags = ["validator"]

  boot_disk {
    initialize_params {
      image = "ubuntu-os-cloud/ubuntu-2204-lts"
      size  = var.validator_volume_size
      type  = "pd-ssd"
    }
  }

  network_interface {
    subnetwork = google_compute_subnetwork.public.self_link
    access_config { nat_ip = null }
  }

  service_account {
    scopes = ["cloud-platform"]
  }

  metadata = {
    ssh-keys = "ubuntu:${file(var.ssh_public_key_path)}"
  }
}

# Validator instances
resource "google_compute_instance" "validator" {
  count        = var.validator_count
  name         = "${var.name_prefix}-validator-${count.index + 1}"
  machine_type = var.validator_machine_type
  zone         = var.gcp_zone

  tags = ["validator"]

  boot_disk {
    initialize_params {
      image = "ubuntu-os-cloud/ubuntu-2204-lts"
      size  = var.validator_volume_size
      type  = "pd-ssd"
    }
  }

  network_interface {
    subnetwork = google_compute_subnetwork.public.self_link
    access_config { nat_ip = null }
  }

  service_account {
    scopes = ["cloud-platform"]
  }

  metadata = {
    ssh-keys = "ubuntu:${file(var.ssh_public_key_path)}"
  }
}

# RPC instances
resource "google_compute_instance" "rpc" {
  count        = var.rpc_count
  name         = "${var.name_prefix}-rpc-${count.index + 1}"
  machine_type = var.rpc_machine_type
  zone         = var.gcp_zone

  tags = ["rpc"]

  boot_disk {
    initialize_params {
      image = "ubuntu-os-cloud/ubuntu-2204-lts"
      size  = var.rpc_volume_size
      type  = "pd-ssd"
    }
  }

  network_interface {
    subnetwork = google_compute_subnetwork.public.self_link
    access_config { nat_ip = null }
  }

  service_account {
    scopes = ["cloud-platform"]
  }

  metadata = {
    ssh-keys = "ubuntu:${file(var.ssh_public_key_path)}"
  }
}

# Global load balancer (HTTP/HTTPS)
resource "google_compute_instance_group" "rpc" {
  count     = var.rpc_count
  name      = "${var.name_prefix}-rpc-ig-${count.index}"
  zone      = var.gcp_zone
  instances = [google_compute_instance.rpc[count.index].self_link]

  named_port {
    name = "http"
    port = 8545
  }
}

resource "google_compute_health_check" "rpc" {
  name = "${var.name_prefix}-hc"

  http_health_check {
    port         = 8545
    request_path = "/health"
  }
}

resource "google_compute_backend_service" "rpc" {
  name       = "${var.name_prefix}-backend"
  port_name  = "http"
  protocol   = "HTTP"
  timeout_sec = 10

  backend {
    group = google_compute_instance_group.rpc[0].self_link
  }
  backend {
    group = google_compute_instance_group.rpc[1].self_link
  }

  health_checks = [google_compute_health_check.rpc.id]
}

resource "google_compute_url_map" "rpc" {
  name            = "${var.name_prefix}-url-map"
  default_service = google_compute_backend_service.rpc.id
}

resource "google_compute_target_https_proxy" "rpc" {
  name    = "${var.name_prefix}-https-proxy"
  url_map = google_compute_url_map.rpc.id
  ssl_certificates = [google_compute_ssl_certificate.rpc.id]
}

resource "google_compute_ssl_certificate" "rpc" {
  name        = "${var.name_prefix}-ssl-cert"
  private_key = var.ssl_private_key
  certificate = var.ssl_certificate
}

resource "google_compute_global_forwarding_rule" "https" {
  name       = "${var.name_prefix}-https-fwd"
  target     = google_compute_target_https_proxy.rpc.id
  port_range = "443"
  ip_address = google_compute_global_address.rpc.id
}

resource "google_compute_global_address" "rpc" {
  name = "${var.name_prefix}-lb-ip"
}

# DNS
resource "google_dns_managed_zone" "main" {
  count       = var.domain_name != "" ? 1 : 0
  name        = "${var.name_prefix}-zone"
  dns_name    = "${var.domain_name}."
  description = "Testnet DNS zone"
}

resource "google_dns_record_set" "rpc" {
  count        = var.domain_name != "" ? 1 : 0
  name         = "rpc.${google_dns_managed_zone.main[0].dns_name}"
  type         = "A"
  ttl          = 60
  managed_zone = google_dns_managed_zone.main[0].name
  rrdatas      = [google_compute_global_address.rpc.address]
}

resource "google_dns_record_set" "bootstrap" {
  count        = var.domain_name != "" ? 1 : 0
  name         = "bootstrap.${google_dns_managed_zone.main[0].dns_name}"
  type         = "A"
  ttl          = 60
  managed_zone = google_dns_managed_zone.main[0].name
  rrdatas      = [google_compute_instance.bootstrap.network_interface[0].access_config[0].nat_ip]
}
