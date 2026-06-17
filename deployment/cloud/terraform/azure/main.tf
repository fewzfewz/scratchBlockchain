terraform {
  required_version = ">= 1.5"
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
  }
}

provider "azurerm" {
  features {}
  subscription_id = var.azure_subscription_id
}

# Resource group
resource "azurerm_resource_group" "testnet" {
  name     = "${var.name_prefix}-rg"
  location = var.azure_location
}

# Virtual network
resource "azurerm_virtual_network" "testnet" {
  name                = "${var.name_prefix}-vnet"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name
  address_space       = ["10.0.0.0/16"]
}

resource "azurerm_subnet" "public" {
  name                 = "${var.name_prefix}-subnet"
  resource_group_name  = azurerm_resource_group.testnet.name
  virtual_network_name = azurerm_virtual_network.testnet.name
  address_prefixes     = ["10.0.1.0/24"]
}

# Public IP for each instance
resource "azurerm_public_ip" "bootstrap" {
  name                = "${var.name_prefix}-bootstrap-pip"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name
  allocation_method   = "Static"
  sku                 = "Standard"
}

resource "azurerm_public_ip" "validator" {
  count               = var.validator_count
  name                = "${var.name_prefix}-validator-${count.index + 1}-pip"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name
  allocation_method   = "Static"
  sku                 = "Standard"
}

resource "azurerm_public_ip" "rpc" {
  count               = var.rpc_count
  name                = "${var.name_prefix}-rpc-${count.index + 1}-pip"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name
  allocation_method   = "Static"
  sku                 = "Standard"
}

resource "azurerm_public_ip" "lb" {
  name                = "${var.name_prefix}-lb-pip"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name
  allocation_method   = "Static"
  sku                 = "Standard"
}

# Network security group
resource "azurerm_network_security_group" "validator" {
  name                = "${var.name_prefix}-validator-nsg"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name

  security_rule {
    name                       = "P2P"
    priority                   = 100
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "26656"
    source_address_prefix      = "*"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "RPC"
    priority                   = 110
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "26657"
    source_address_prefix      = "VirtualNetwork"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "API"
    priority                   = 120
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "8545"
    source_address_prefix      = "VirtualNetwork"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "Metrics"
    priority                   = 130
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "9090"
    source_address_prefix      = "VirtualNetwork"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "SSH"
    priority                   = 140
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "22"
    source_address_prefixes    = var.admin_cidr_blocks
    destination_address_prefix = "*"
  }
}

resource "azurerm_network_security_group" "rpc" {
  name                = "${var.name_prefix}-rpc-nsg"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name

  security_rule {
    name                       = "API"
    priority                   = 100
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "8545"
    source_address_prefix      = "VirtualNetwork"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "P2P"
    priority                   = 110
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "26656"
    source_address_prefix      = "VirtualNetwork"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "SSH"
    priority                   = 120
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "22"
    source_address_prefixes    = var.admin_cidr_blocks
    destination_address_prefix = "*"
  }
}

resource "azurerm_network_security_group" "lb" {
  name                = "${var.name_prefix}-lb-nsg"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name

  security_rule {
    name                       = "HTTPS"
    priority                   = 100
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "443"
    source_address_prefix      = "*"
    destination_address_prefix = "*"
  }
  security_rule {
    name                       = "HTTP"
    priority                   = 110
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "80"
    source_address_prefix      = "*"
    destination_address_prefix = "*"
  }
}

# Bootstrap node
resource "azurerm_network_interface" "bootstrap" {
  name                = "${var.name_prefix}-bootstrap-nic"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name

  ip_configuration {
    name                          = "primary"
    subnet_id                     = azurerm_subnet.public.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id          = azurerm_public_ip.bootstrap.id
  }
}

resource "azurerm_network_interface_security_group_association" "bootstrap" {
  network_interface_id      = azurerm_network_interface.bootstrap.id
  network_security_group_id = azurerm_network_security_group.validator.id
}

resource "azurerm_linux_virtual_machine" "bootstrap" {
  name                            = "${var.name_prefix}-bootstrap"
  location                        = azurerm_resource_group.testnet.location
  resource_group_name             = azurerm_resource_group.testnet.name
  size                            = var.validator_vm_size
  admin_username                  = "ubuntu"
  disable_password_authentication = true
  network_interface_ids = [
    azurerm_network_interface.bootstrap.id,
  ]

  admin_ssh_key {
    username   = "ubuntu"
    public_key = file(var.ssh_public_key_path)
  }

  os_disk {
    name                 = "${var.name_prefix}-bootstrap-osdisk"
    caching              = "ReadWrite"
    storage_account_type = "Premium_LRS"
    disk_size_gb         = var.validator_volume_size
  }

  source_image_reference {
    publisher = "Canonical"
    offer     = "ubuntu-server"
    sku       = "22_04-lts-gen2"
    version   = "latest"
  }
}

# Validator nodes
resource "azurerm_network_interface" "validator" {
  count               = var.validator_count
  name                = "${var.name_prefix}-validator-${count.index + 1}-nic"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name

  ip_configuration {
    name                          = "primary"
    subnet_id                     = azurerm_subnet.public.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id          = azurerm_public_ip.validator[count.index].id
  }
}

resource "azurerm_network_interface_security_group_association" "validator" {
  count                     = var.validator_count
  network_interface_id      = azurerm_network_interface.validator[count.index].id
  network_security_group_id = azurerm_network_security_group.validator.id
}

resource "azurerm_linux_virtual_machine" "validator" {
  count                           = var.validator_count
  name                            = "${var.name_prefix}-validator-${count.index + 1}"
  location                        = azurerm_resource_group.testnet.location
  resource_group_name             = azurerm_resource_group.testnet.name
  size                            = var.validator_vm_size
  admin_username                  = "ubuntu"
  disable_password_authentication = true
  network_interface_ids = [
    azurerm_network_interface.validator[count.index].id,
  ]

  admin_ssh_key {
    username   = "ubuntu"
    public_key = file(var.ssh_public_key_path)
  }

  os_disk {
    name                 = "${var.name_prefix}-validator-${count.index + 1}-osdisk"
    caching              = "ReadWrite"
    storage_account_type = "Premium_LRS"
    disk_size_gb         = var.validator_volume_size
  }

  source_image_reference {
    publisher = "Canonical"
    offer     = "ubuntu-server"
    sku       = "22_04-lts-gen2"
    version   = "latest"
  }
}

# RPC nodes
resource "azurerm_network_interface" "rpc" {
  count               = var.rpc_count
  name                = "${var.name_prefix}-rpc-${count.index + 1}-nic"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name

  ip_configuration {
    name                          = "primary"
    subnet_id                     = azurerm_subnet.public.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id          = azurerm_public_ip.rpc[count.index].id
  }
}

resource "azurerm_network_interface_security_group_association" "rpc" {
  count                     = var.rpc_count
  network_interface_id      = azurerm_network_interface.rpc[count.index].id
  network_security_group_id = azurerm_network_security_group.rpc.id
}

resource "azurerm_linux_virtual_machine" "rpc" {
  count                           = var.rpc_count
  name                            = "${var.name_prefix}-rpc-${count.index + 1}"
  location                        = azurerm_resource_group.testnet.location
  resource_group_name             = azurerm_resource_group.testnet.name
  size                            = var.rpc_vm_size
  admin_username                  = "ubuntu"
  disable_password_authentication = true
  network_interface_ids = [
    azurerm_network_interface.rpc[count.index].id,
  ]

  admin_ssh_key {
    username   = "ubuntu"
    public_key = file(var.ssh_public_key_path)
  }

  os_disk {
    name                 = "${var.name_prefix}-rpc-${count.index + 1}-osdisk"
    caching              = "ReadWrite"
    storage_account_type = "Premium_LRS"
    disk_size_gb         = var.rpc_volume_size
  }

  source_image_reference {
    publisher = "Canonical"
    offer     = "ubuntu-server"
    sku       = "22_04-lts-gen2"
    version   = "latest"
  }
}

# Load balancer
resource "azurerm_lb" "rpc" {
  name                = "${var.name_prefix}-lb"
  location            = azurerm_resource_group.testnet.location
  resource_group_name = azurerm_resource_group.testnet.name
  sku                 = "Standard"

  frontend_ip_configuration {
    name                 = "PublicIPAddress"
    public_ip_address_id = azurerm_public_ip.lb.id
  }
}

resource "azurerm_lb_backend_address_pool" "rpc" {
  name            = "${var.name_prefix}-backend-pool"
  loadbalancer_id = azurerm_lb.rpc.id
}

resource "azurerm_lb_backend_address_pool_address" "rpc" {
  count                   = var.rpc_count
  name                    = "${var.name_prefix}-rpc-${count.index + 1}-address"
  backend_address_pool_id = azurerm_lb_backend_address_pool.rpc.id
  virtual_network_id      = azurerm_virtual_network.testnet.id
  ip_address              = azurerm_network_interface.rpc[count.index].private_ip_address
}

resource "azurerm_lb_probe" "rpc" {
  name                = "${var.name_prefix}-health-probe"
  loadbalancer_id     = azurerm_lb.rpc.id
  protocol            = "Http"
  port                = 8545
  request_path        = "/health"
  interval_in_seconds = 10
  number_of_probes    = 3
}

resource "azurerm_lb_rule" "https" {
  name                           = "${var.name_prefix}-https-rule"
  loadbalancer_id                = azurerm_lb.rpc.id
  protocol                       = "Tcp"
  frontend_port                  = 443
  backend_port                   = 8545
  frontend_ip_configuration_name = "PublicIPAddress"
  backend_address_pool_ids       = [azurerm_lb_backend_address_pool.rpc.id]
  probe_id                       = azurerm_lb_probe.rpc.id
}

resource "azurerm_lb_rule" "http" {
  name                           = "${var.name_prefix}-http-rule"
  loadbalancer_id                = azurerm_lb.rpc.id
  protocol                       = "Tcp"
  frontend_port                  = 80
  backend_port                   = 8545
  frontend_ip_configuration_name = "PublicIPAddress"
  backend_address_pool_ids       = [azurerm_lb_backend_address_pool.rpc.id]
  probe_id                       = azurerm_lb_probe.rpc.id
}

# Azure DNS zone
resource "azurerm_dns_zone" "main" {
  count               = var.domain_name != "" ? 1 : 0
  name                = var.domain_name
  resource_group_name = azurerm_resource_group.testnet.name
}

resource "azurerm_dns_a_record" "rpc" {
  count               = var.domain_name != "" ? 1 : 0
  name                = "rpc"
  zone_name           = azurerm_dns_zone.main[0].name
  resource_group_name = azurerm_resource_group.testnet.name
  ttl                 = 60
  records             = [azurerm_public_ip.lb.ip_address]
}

resource "azurerm_dns_a_record" "bootstrap" {
  count               = var.domain_name != "" ? 1 : 0
  name                = "bootstrap"
  zone_name           = azurerm_dns_zone.main[0].name
  resource_group_name = azurerm_resource_group.testnet.name
  ttl                 = 60
  records             = [azurerm_public_ip.bootstrap.ip_address]
}

resource "azurerm_dns_a_record" "faucet" {
  count               = var.domain_name != "" ? 1 : 0
  name                = "faucet"
  zone_name           = azurerm_dns_zone.main[0].name
  resource_group_name = azurerm_resource_group.testnet.name
  ttl                 = 60
  records             = [azurerm_public_ip.lb.ip_address]
}
