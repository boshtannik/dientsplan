#!/usr/bin/env bash
set -euo pipefail

DB_FILE="./duties.db"
BACKUP_DIR="./backups"

mkdir -p "$BACKUP_DIR"

TIMESTAMP="$(date +%Y-%m-%d_%H-%M-%S)"
BACKUP_FILE="$BACKUP_DIR/$(basename "$DB_FILE" .db)-backup-$TIMESTAMP.db"

cp "$DB_FILE" "$BACKUP_FILE"

echo "Backup created: $BACKUP_FILE"
