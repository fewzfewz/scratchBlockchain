#!/bin/bash
# Backup Script for Blockchain Data

set -e

BACKUP_DIR="./backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="blockchain_backup_${TIMESTAMP}"

echo "🔒 Blockchain Backup Script"
echo "==========================="

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Stop nodes (optional - comment out for hot backup)
# echo "⏸️  Stopping nodes..."
# docker-compose stop node1 node2 node3

# Backup node data
echo "📦 Backing up node data..."
docker run --rm \
  -v blockchain_node1-data:/data \
  -v "$(pwd)/$BACKUP_DIR:/backup" \
  alpine tar czf "/backup/${BACKUP_NAME}_node1.tar.gz" -C /data .

docker run --rm \
  -v blockchain_node2-data:/data \
  -v "$(pwd)/$BACKUP_DIR:/backup" \
  alpine tar czf "/backup/${BACKUP_NAME}_node2.tar.gz" -C /data .

docker run --rm \
  -v blockchain_node3-data:/data \
  -v "$(pwd)/$BACKUP_DIR:/backup" \
  alpine tar czf "/backup/${BACKUP_NAME}_node3.tar.gz" -C /data .

# Restart nodes if stopped
# echo "▶️  Restarting nodes..."
# docker-compose start node1 node2 node3

echo "✅ Backup complete: $BACKUP_DIR/$BACKUP_NAME"
echo "📊 Backup size:"
du -sh "$BACKUP_DIR/${BACKUP_NAME}"*

# Clean old backups (keep last 7 days)
echo "🧹 Cleaning old backups..."
find "$BACKUP_DIR" -name "blockchain_backup_*.tar.gz" -mtime +7 -delete

echo "✨ Backup process finished"
