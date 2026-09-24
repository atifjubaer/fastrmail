#!/bin/sh
# FastrMail Container Health Check Script
# Used by Docker, Dokploy, and Coolify to monitor service health.

wget -qO- http://127.0.0.1:8080/api/health > /dev/null 2>&1 || exit 1
exit 0
