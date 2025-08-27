# Docker Setup for AKD- **Static linking** - All dependencies are statically linked for better security isolation

## Quick Start

1. **Build all images**:
   ```bash
   ./docker/build.sh --all
   ```

2. **Customize configuration** (edit `docker/config.docker.toml`):
   - Update namespace configurations with your actual log directories
   - Set appropriate starting epochs. You _can_ start from anywhere, but you will be unable to provide signatures for earlier epochs and you will only establish a chain of trust to the starting epoch.
   - Configure storage backends

3. **Start services with persistent storage**:
   ```bash
   cd docker
   docker-compose up -d
   ```

4. **View logs**:
   ```bash
   docker-compose logs -f
   ```

## Build Optimizationtch

This directory contains Docker configurations for building and running AKD Watch components as distroless, rootless containers.

## Images

Three optimized Docker images are available (all use cargo-chef for dependency caching):

1. **AIO (All-in-One)**: Runs both auditor and web server in a single container, as a single process
2. **Auditor**: Runs only the auditor service
3. **Web**: Runs only the web server

## Security Features

All images are built with security in mind:

- **Alpine build base**: Uses `rust:1.88.0-alpine3.20` for smaller, more secure build environment
- **Distroless runtime**: Uses `gcr.io/distroless/cc-debian12:nonroot` for minimal attack surface
- **Rootless**: Runs as non-root user (uid/gid 65532)
- **Read-only filesystem**: Container filesystem is mounted read-only
- **No new privileges**: Security option prevents privilege escalation
- **Minimal dependencies**: Only includes necessary runtime libraries
- **Static linking**: All dependencies are statically linked for better security isolation

## Build Optimization

All Dockerfiles use **cargo-chef** for intelligent dependency caching:
- **First build**: ~8-12 minutes (installs cargo-chef + compiles dependencies + app)
- **Subsequent builds**: ~1-3 minutes (reuses cached dependencies, only compiles app changes)
- **Works in CI/CD**: GitHub Actions cache configuration preserves layers between runs
- **Alpine benefits**: Smaller base image (~50MB vs ~1.2GB), fewer vulnerabilities

## Building Images

### Using the build script (recommended)

```bash
# Build all images
./docker/build.sh --all

# Build specific images
./docker/build.sh aio web

# Build with custom tag
./docker/build.sh --tag v1.0.0 --all

# Build and push to registry
./docker/build.sh --tag v1.0.0 --push --all
```

### Using Docker directly

```bash
# Build AIO image
docker build -f docker/Dockerfile.aio -t akd-watch/aio:latest .

# Build auditor image
docker build -f docker/Dockerfile.auditor -t akd-watch/auditor:latest .

# Build web image
docker build -f docker/Dockerfile.web -t akd-watch/web:latest .
docker build -f docker/Dockerfile.web -t akd-watch:web .
```

## Running Containers

### Using Docker Compose (recommended)

```bash
# Run AIO service
cd docker
docker-compose up akd-watch-aio

# Run standalone services
docker-compose --profile standalone up
```

### Using Docker directly

```bash
# Run AIO container
docker run -p 8080:8080 \
  -v $(pwd)/config.toml:/etc/akd-watch/config.toml:ro \
  --read-only \
  --tmpfs /tmp:noexec,nosuid,size=100m \
  --security-opt no-new-privileges:true \
  akd-watch/aio:latest

# Run auditor container
docker run \
  -v $(pwd)/config.toml:/etc/akd-watch/config.toml:ro \
  --read-only \
  --tmpfs /tmp:noexec,nosuid,size=100m \
  --security-opt no-new-privileges:true \
  akd-watch/auditor:latest

# Run web container
docker run -p 8080:8080 \
  -v $(pwd)/config.toml:/etc/akd-watch/config.toml:ro \
  --read-only \
  --tmpfs /tmp:noexec,nosuid,size=100m \
  --security-opt no-new-privileges:true \
  akd-watch/web:latest
```

## Configuration


1. **Environment variables** (`AKD_WATCH_*`) - Always applied with highest priority
2. **Config file source** (only one is used):
   - Custom config file (if `AKD_WATCH_CONFIG_PATH` is set) **OR**
   - Default working directory config files (`config.toml`, `config.yaml`, `config.json`)
3. **Built-in defaults** - Fallback values if no config found

## Ports

- **AIO**: Exposes port 8080 for the web service
- **Auditor**: No exposed ports (internal service)
- **Web**: Exposes port 8080

## Persistent Storage

### Shared Storage Requirements

For split deployments, the auditor and web services must share the same storage for:
- **Namespace state** - Tracks audit progress for each namespace
- **Signature files** - Stores cryptographic signatures for verification. Note: web only needs read access to the verify signatures
- **Signing/Verifying keys** - Cryptographic keys for signature operations

## Volumes

The containers use a read-only filesystem with a temporary filesystem mounted at `/tmp` for any runtime temporary files.

## Development

### Build optimizations

All Dockerfiles use optimized multi-stage builds with cargo-chef:

1. **Chef stage**: Install cargo-chef tool
2. **Planner stage**: Analyze dependencies and create recipe.json  
3. **Builder stage**: Build dependencies (cached layer)
4. **App-builder stage**: Build application code
5. **Runtime stage**: Copy binary to distroless image

**Simple versions available**: If you need simpler builds without cargo-chef, use:
- `docker/Dockerfile.aio.simple`
- `docker/Dockerfile.auditor.simple` 
- `docker/Dockerfile.web.simple`

### Performance improvements

- **Dependency caching**: Dependencies are cached until Cargo.toml/Cargo.lock changes
- **Layer optimization**: Source code changes don't invalidate dependency cache
- **CI/CD friendly**: GitHub Actions cache preserves layers between workflow runs

### Security scanning

Consider running security scans on the built images:

```bash
# Using trivy (install from https://github.com/aquasecurity/trivy)
trivy image akd-watch/aio:latest
trivy image akd-watch/auditor:latest
trivy image akd-watch/web:latest
```

## Troubleshooting

### Common issues

1. **Permission denied**: Ensure the configuration file is readable by uid 65532
2. **Configuration not found**: Verify the volume mount path
3. **Build failures**: Check that all dependencies are available and Docker has sufficient resources
4. **Alpine linking errors**: The images include `openssl-libs-static` to resolve OpenSSL linking issues with musl libc

### Debugging

To debug issues, you can run a container with a shell (note: distroless images don't include a shell, so use the debug variant):

```bash
# Use the debug version of distroless for troubleshooting
# Modify the Dockerfile temporarily to use gcr.io/distroless/cc-debian12:debug-nonroot
docker run -it --entrypoint sh akd-watch/aio:debug
```

## Production Considerations

1. **Resource limits**: Set appropriate CPU and memory limits
2. **Health checks**: Implement health check endpoints and configure Docker health checks
3. **Logging**: Configure structured logging and log aggregation
4. **Secrets management**: Use Docker secrets or external secret management for sensitive configuration
5. **Registry**: Push images to a private registry for production use
6. **Monitoring**: Set up monitoring and alerting for container health and performance
