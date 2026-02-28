# OpenClaw Router -- Deployment Guide

> Deploy the full OpenClaw Router stack with Docker Compose.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Environment Variables](#environment-variables)
- [Domain Setup](#domain-setup)
- [SSL/TLS with Let's Encrypt](#ssltls-with-lets-encrypt)
- [Production Checklist](#production-checklist)
- [Service Architecture](#service-architecture)
- [Monitoring and Logging](#monitoring-and-logging)
- [Backup Strategy](#backup-strategy)
- [Scaling](#scaling)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| Docker | 24.0+ | Latest stable |
| Docker Compose | v2.20+ | Latest stable |
| RAM | 2 GB | 4 GB+ |
| Disk | 10 GB | 20 GB+ |
| CPU | 2 cores | 4 cores |
| OS | Any Linux, macOS, or Windows with WSL2 | Ubuntu 22.04 LTS |

**Required accounts / keys** (at least one LLM provider):

- OpenAI API key (`OPENAI_API_KEY`)
- Google Gemini API key (`GEMINI_API_KEY`)
- OpenRouter API key (`OPENROUTER_API_KEY`)
- xAI API key (`XAI_API_KEY`)
- DeepSeek API key (`DEEPSEEK_API_KEY`)
- Or a local Ollama instance (no key required)

**For dashboard authentication**:

- Google Cloud OAuth credentials (`GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`)

**For production domain**:

- A domain name with DNS management access
- A server with a public IP address

---

## Quick Start

### 1. Clone the repository

```bash
git clone https://github.com/sunilvalmiki/openclaw-router.git
cd openclaw-router
```

### 2. Configure environment variables

```bash
cp docker/.env.example docker/.env
```

Edit `docker/.env` with your API keys:

```bash
# Database (change password for production)
POSTGRES_USER=openclaw
POSTGRES_PASSWORD=your-secure-password-here
POSTGRES_DB=openclaw_router

# Dashboard Auth
NEXTAUTH_SECRET=$(openssl rand -base64 32)
NEXTAUTH_URL=https://your-domain.com
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret

# Provider API Keys (set at least one)
OPENAI_API_KEY=sk-...
OPENROUTER_API_KEY=sk-or-...
GEMINI_API_KEY=AIza...
XAI_API_KEY=xai-...
DEEPSEEK_API_KEY=sk-...
```

### 3. Start the stack

```bash
cd docker
docker compose up -d
```

This starts five services:

| Service | Port | Description |
|---------|------|-------------|
| `gateway` | 8080 | Rust LLM routing gateway |
| `dashboard` | 3000 | Next.js admin dashboard |
| `postgres` | 5432 | PostgreSQL 16 database |
| `redis` | 6379 | Redis 7 cache |
| `nginx` | 80, 443 | Reverse proxy with TLS |

### 4. Verify the deployment

```bash
# Check all services are running
docker compose ps

# Test gateway health
curl http://localhost:8080/health

# Test gateway readiness (PostgreSQL connectivity)
curl http://localhost:8080/ready

# Test a chat completion
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### 5. Access the dashboard

Open `http://localhost:3000` in your browser (or `https://your-domain.com` if DNS is configured).

---

## Environment Variables

### Gateway Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DATABASE_URL` | **Yes** | -- | PostgreSQL connection string. Set automatically by Docker Compose. |
| `HOST` | No | `0.0.0.0` | Bind address for the HTTP server. |
| `PORT` | No | `8080` | Port number for the HTTP server. |
| `REDIS_URL` | No | `redis://127.0.0.1:6379` | Redis connection string. |
| `DEFAULT_COST_PROFILE` | No | `balanced` | Default cost profile: `eco`, `balanced`, `premium`, `free`. |
| `CACHE_TTL_SECONDS` | No | `3600` | Response cache TTL in seconds. |
| `MAX_CACHE_ENTRIES` | No | `10000` | Maximum number of cached responses. |
| `PRICING_SYNC_INTERVAL_SECONDS` | No | `3600` | Interval between pricing data refreshes. |
| `DASHBOARD_URL` | No | `http://localhost:3000` | Dashboard URL (used for CORS). |
| `RUST_LOG` | No | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error`. |

### Provider API Keys

| Variable | Required | Description |
|----------|----------|-------------|
| `OPENAI_API_KEY` | No | OpenAI API key. Enables GPT-4o, GPT-4o Mini, GPT-3.5 Turbo. |
| `OPENAI_BASE_URL` | No | Custom OpenAI-compatible base URL (e.g., Azure OpenAI). |
| `GEMINI_API_KEY` | No | Google Gemini API key. Enables Gemini 2.5 Pro/Flash. |
| `OPENROUTER_API_KEY` | No | OpenRouter API key. Enables 300+ models. |
| `XAI_API_KEY` | No | xAI API key. Enables Grok 3/3 Mini. |
| `DEEPSEEK_API_KEY` | No | DeepSeek API key. Enables DeepSeek Chat/Reasoner. |
| `OLLAMA_ENDPOINT` | No | Ollama API endpoint (default: `http://localhost:11434`). Always enabled. |
| `AWS_ACCESS_KEY_ID` | No | AWS IAM access key for Bedrock. |
| `AWS_SECRET_ACCESS_KEY` | No | AWS IAM secret key for Bedrock. |
| `AWS_REGION` | No | AWS region for Bedrock (default: `us-east-1`). |
| `CUSTOM_LLM_BASE_URL` | No | Base URL for a custom OpenAI-compatible provider. |
| `CUSTOM_LLM_NAME` | No | Display name for the custom provider (default: `custom`). |
| `CUSTOM_LLM_API_KEY` | No | API key for the custom provider. |

### Dashboard Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NEXTAUTH_URL` | **Yes** | -- | Public URL of the dashboard (e.g., `https://sunilkumarvalmiki.xyz`). |
| `NEXTAUTH_SECRET` | **Yes** | -- | Random secret for session encryption. Generate with `openssl rand -base64 32`. |
| `GOOGLE_CLIENT_ID` | No | -- | Google OAuth client ID for sign-in. |
| `GOOGLE_CLIENT_SECRET` | No | -- | Google OAuth client secret. |
| `GATEWAY_URL` | No | `http://gateway:8080` | Internal URL of the gateway (Docker network). |

### Database Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_USER` | No | `openclaw` | PostgreSQL username. |
| `POSTGRES_PASSWORD` | No | `openclaw_secret` | PostgreSQL password. **Change in production.** |
| `POSTGRES_DB` | No | `openclaw_router` | PostgreSQL database name. |

---

## Domain Setup

### 1. Configure DNS

Add an A record pointing your domain to your server's public IP address:

| Type | Name | Value | TTL |
|------|------|-------|-----|
| A | `@` | `your.server.ip` | 300 |
| A | `www` | `your.server.ip` | 300 |

For `sunilkumarvalmiki.xyz`:

```
sunilkumarvalmiki.xyz     A    -> server-ip
www.sunilkumarvalmiki.xyz A    -> server-ip
```

### 2. Verify DNS propagation

```bash
dig sunilkumarvalmiki.xyz +short
# Should return your server IP
```

### 3. Update Nginx configuration

The default `docker/nginx.conf` is pre-configured for `sunilkumarvalmiki.xyz`. If using a different domain, update the `server_name` directives:

```nginx
server_name yourdomain.com www.yourdomain.com;
```

### Nginx Routing

The Nginx reverse proxy routes traffic as follows:

| Path | Destination | Description |
|------|-------------|-------------|
| `/v1/*` | Gateway (port 8080) | LLM API endpoints with rate limiting |
| `/health` | Gateway (port 8080) | Health check |
| `/ready` | Gateway (port 8080) | Readiness check |
| `/metrics` | Gateway (port 8080) | Prometheus metrics |
| `/api/admin/*` | Gateway (port 8080) | Admin API |
| `/*` | Dashboard (port 3000) | Next.js dashboard |

---

## SSL/TLS with Let's Encrypt

### Initial certificate setup

```bash
# Install certbot on the host (not in Docker)
sudo apt install certbot

# Stop nginx temporarily
cd docker
docker compose stop nginx

# Obtain certificate
sudo certbot certonly --standalone \
  -d sunilkumarvalmiki.xyz \
  -d www.sunilkumarvalmiki.xyz \
  --agree-tos \
  --email your-email@example.com

# Start nginx
docker compose up -d nginx
```

### Auto-renewal

Set up a cron job for automatic certificate renewal:

```bash
sudo crontab -e
```

Add the following line:

```
0 3 * * * certbot renew --quiet --pre-hook "cd /path/to/openclaw-router/docker && docker compose stop nginx" --post-hook "cd /path/to/openclaw-router/docker && docker compose start nginx"
```

This checks for renewal daily at 3:00 AM and restarts Nginx if the certificate is renewed.

---

## Production Checklist

### Security

- [ ] Change `POSTGRES_PASSWORD` from the default.
- [ ] Generate a strong `NEXTAUTH_SECRET` (`openssl rand -base64 32`).
- [ ] Never commit `.env` files to version control.
- [ ] Set up SSL/TLS certificates (see above).
- [ ] Review and restrict API key access.
- [ ] Consider IP-based firewall rules for database and Redis ports.
- [ ] Ensure Redis and PostgreSQL ports (5432, 6379) are not exposed to the public internet.

### Performance

- [ ] Set appropriate `CACHE_TTL_SECONDS` for your workload.
- [ ] Tune `MAX_CACHE_ENTRIES` based on available memory.
- [ ] Monitor Redis memory usage (256MB limit by default).
- [ ] Set appropriate PostgreSQL connection pool size (default: 20).

### Reliability

- [ ] Verify health checks are passing: `curl localhost:8080/health` and `curl localhost:8080/ready`.
- [ ] Ensure `restart: unless-stopped` is set for all services (default in compose).
- [ ] Set up monitoring and alerting (see below).
- [ ] Set up database backups (see below).

### Observability

- [ ] Set `RUST_LOG=info` for production (avoid `debug` or `trace` in production).
- [ ] Configure log aggregation (e.g., Loki, CloudWatch, Datadog).
- [ ] Set up uptime monitoring for `/health` endpoint.

---

## Service Architecture

### Docker Compose Services

```
                     +---------------+
                     |    Nginx      |
                     |  :80 / :443   |
                     +---+-------+---+
                         |       |
              +----------+       +----------+
              |                              |
    +---------v---------+      +-------------v-----------+
    |   Gateway (Rust)  |      |   Dashboard (Next.js)   |
    |       :8080       |      |         :3000            |
    +---------+---------+      +-------------------------+
              |
    +---------+----------+
    |                    |
+---v---------+    +-----v------+
| PostgreSQL  |    |   Redis    |
|    :5432    |    |   :6379    |
+-------------+    +------------+
```

### Startup Order

Docker Compose health checks ensure correct startup ordering:

1. **PostgreSQL** starts first and becomes healthy when `pg_isready` succeeds.
2. **Redis** starts first and becomes healthy when `redis-cli ping` succeeds.
3. **Gateway** waits for PostgreSQL and Redis to be healthy, then runs database migrations on startup.
4. **Dashboard** waits for the gateway to start.
5. **Nginx** waits for both the gateway and dashboard.

### Network

All services communicate over a private Docker bridge network (`openclaw-net`). Only Nginx exposes ports 80 and 443 to the host. In development, the gateway (8080), dashboard (3000), PostgreSQL (5432), and Redis (6379) are also exposed for local access.

---

## Monitoring and Logging

### Structured Logging

The gateway outputs structured JSON logs via `tracing-subscriber`:

```json
{
  "timestamp": "2026-03-01T10:30:00.000Z",
  "level": "INFO",
  "target": "openclaw_gateway::routes::completions",
  "message": "Request scored",
  "tier": "simple",
  "complexity": 12.5,
  "profile": "auto"
}
```

### Viewing Logs

```bash
# All services
docker compose logs -f

# Gateway only
docker compose logs -f gateway

# Last 100 lines
docker compose logs --tail 100 gateway

# Filter for errors
docker compose logs gateway 2>&1 | grep '"level":"ERROR"'
```

### Log Levels

Set via the `RUST_LOG` environment variable:

| Level | Use Case |
|-------|----------|
| `error` | Production (minimal output) |
| `warn` | Production with warnings |
| `info` | **Recommended for production** |
| `debug` | Development/debugging |
| `trace` | Verbose debugging (high volume) |

Granular control:

```bash
RUST_LOG=openclaw_gateway=info,actix_web=warn,sqlx=warn
```

### Health Monitoring

Set up external uptime monitoring (e.g., UptimeRobot, Pingdom, or a simple cron job):

```bash
# Simple health check script
#!/bin/bash
HEALTH=$(curl -s -o /dev/null -w "%{http_code}" https://sunilkumarvalmiki.xyz/health)
READY=$(curl -s -o /dev/null -w "%{http_code}" https://sunilkumarvalmiki.xyz/ready)

if [ "$HEALTH" != "200" ] || [ "$READY" != "200" ]; then
  echo "ALERT: OpenClaw Router is unhealthy (health=$HEALTH, ready=$READY)"
  # Send alert via email, Slack, etc.
fi
```

### Redis Monitoring

```bash
# Connect to Redis CLI
docker compose exec redis redis-cli

# Check memory usage
INFO memory

# Check cache stats
GET cache:stats:hits
GET cache:stats:misses

# Count cached entries
DBSIZE
```

### PostgreSQL Monitoring

```bash
# Connect to psql
docker compose exec postgres psql -U openclaw -d openclaw_router

# Check active connections
SELECT count(*) FROM pg_stat_activity;

# Check database size
SELECT pg_size_pretty(pg_database_size('openclaw_router'));
```

---

## Backup Strategy

### PostgreSQL Backups

#### Automated daily backup

Create a backup script at `/opt/openclaw/backup.sh`:

```bash
#!/bin/bash
BACKUP_DIR="/opt/openclaw/backups"
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p "$BACKUP_DIR"

# Dump the database
docker compose -f /path/to/docker/docker-compose.yml exec -T postgres \
  pg_dump -U openclaw -d openclaw_router \
  | gzip > "$BACKUP_DIR/openclaw_$DATE.sql.gz"

# Keep only last 30 days
find "$BACKUP_DIR" -name "*.sql.gz" -mtime +30 -delete

echo "Backup completed: openclaw_$DATE.sql.gz"
```

Schedule with cron:

```bash
sudo crontab -e
# Daily at 2:00 AM
0 2 * * * /opt/openclaw/backup.sh >> /var/log/openclaw-backup.log 2>&1
```

#### Manual backup

```bash
docker compose exec -T postgres pg_dump -U openclaw -d openclaw_router > backup.sql
```

#### Restore from backup

```bash
# Stop the gateway first
docker compose stop gateway

# Restore
gunzip -c openclaw_20260301_020000.sql.gz | \
  docker compose exec -T postgres psql -U openclaw -d openclaw_router

# Start the gateway
docker compose start gateway
```

### Redis Data

Redis data is persisted to the `redis_data` Docker volume. For critical deployments, enable Redis RDB snapshots or AOF persistence:

```bash
# In docker-compose.yml, modify the Redis command:
command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru --save 900 1 --save 300 10
```

---

## Scaling

### Vertical Scaling

Increase resources for the gateway:

```yaml
# In docker-compose.yml
gateway:
  deploy:
    resources:
      limits:
        cpus: '4'
        memory: 2G
```

The gateway automatically uses all available CPU cores (`num_cpus::get()` workers) and supports up to 25,000 concurrent connections.

### Horizontal Scaling (Future)

For multi-instance deployments:

1. Move rate limiting to Redis (currently in-memory per instance).
2. Add a load balancer in front of multiple gateway instances.
3. Use PostgreSQL connection pooling (e.g., PgBouncer).
4. Share the Redis cache across instances (already external).

---

## Troubleshooting

### Common Issues

#### Gateway fails to start: "Failed to connect to PostgreSQL"

PostgreSQL is not ready yet. Check health:

```bash
docker compose ps postgres
docker compose logs postgres
```

Ensure the health check is passing before the gateway starts.

#### "No healthy providers available"

No LLM provider API keys are configured. Set at least one:

```bash
# In docker/.env
OPENAI_API_KEY=sk-your-key-here
```

Then restart:

```bash
docker compose restart gateway
```

#### Redis connection errors in logs

Redis may not be running or reachable. Check:

```bash
docker compose ps redis
docker compose exec redis redis-cli ping
# Should return: PONG
```

#### SSL certificate not found

Certificates have not been generated yet. Follow the [SSL/TLS with Let's Encrypt](#ssltls-with-lets-encrypt) section.

For local development without SSL, use HTTP directly on port 8080:

```bash
curl http://localhost:8080/health
```

#### Dashboard shows "Unauthorized"

- Verify `NEXTAUTH_SECRET` is set and consistent across restarts.
- Verify `NEXTAUTH_URL` matches the actual dashboard URL.
- For Google OAuth, ensure the redirect URI in Google Cloud Console matches your domain.

#### High latency on first request

The first request after startup may be slower due to:
- Database connection pool initialization.
- Provider health checks.
- Cold JIT compilation in the Rust runtime (minimal).

Subsequent requests benefit from connection pooling and caching.

### Useful Commands

```bash
# Restart a single service
docker compose restart gateway

# Rebuild and restart (after code changes)
docker compose up -d --build gateway

# View resource usage
docker compose stats

# Enter a container shell
docker compose exec gateway /bin/bash
docker compose exec postgres psql -U openclaw -d openclaw_router

# Reset everything (WARNING: destroys all data)
docker compose down -v
docker compose up -d
```
