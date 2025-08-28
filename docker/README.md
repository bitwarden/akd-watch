# Docker Setup for AKD

## Quick Start

### Build all images
```bash
./docker/build.sh --all
```

### Customize configuration
`docker/config.docker.toml` is a good starting point. You should:
- Update namespace configurations with your actual log directories
- Set appropriate starting epochs. You _can_ start from anywhere, but you will be unable to provide signatures for earlier epochs and you will only establish a chain of trust to the starting epoch.
- Configure storage backends.
- Configure a data directory for persistent storage (e.g. `./data`). Both auditor and web services need to use the same directory, though web only needs read access.

### Start services
```bash
cd docker
docker-compose up -d
```

## Images

Three Docker images are available:

1. **AIO (All-in-One)**: Runs both auditor and web server in a single container, as a single process
2. **Auditor**: Runs only the auditor service
3. **Web**: Runs only the web server

A docker compose file is also provided to run the auditor and web services at the same time in separate containers.

## Security Features

All images are built with security in mind:

- **Alpine build base**: Uses `rust:1.88.0-alpine3.20` for smaller, more secure build environment
- **Distroless runtime**: Uses `gcr.io/distroless/cc-debian12:nonroot` for minimal attack surface
- **Rootless**: Runs as non-root user (uid/gid 65532)
- **No new privileges**: Security option prevents privilege escalation

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
```

## Running Containers

### Using Docker Compose (recommended)

```bash
cd docker
docker-compose up -d
```

### Using Docker directly

```bash
# Run AIO container
docker run -p 3000:3000 \
  -e AKD_WATCH_CONFIG_PATH=/etc/akd-watch/config.toml \
  -e AKD_WATCH__DATA_DIRECTORY=/var/lib/akd-watch \
  -v $(pwd)/config.toml:/etc/akd-watch/config.toml:ro \
  -v $(pwd)/data:/var/lib/akd-watch:rw \
  --tmpfs /tmp:noexec,nosuid,size=100m \
  --security-opt no-new-privileges:true \
  akd-watch/aio:latest
```
or
```bash
# Run auditor container
docker run \
  -e AKD_WATCH_CONFIG_PATH=/etc/akd-watch/config.toml \
  -e AKD_WATCH__DATA_DIRECTORY=/var/lib/akd-watch \
  -v $(pwd)/config.toml:/etc/akd-watch/config.toml:ro \
  -v $(pwd)/data:/var/lib/akd-watch:rw \
  --tmpfs /tmp:noexec,nosuid,size=100m \
  --security-opt no-new-privileges:true \
  akd-watch/auditor:latest

# Run web container
docker run -p 8080:8080 \
  -e AKD_WATCH_CONFIG_PATH=/etc/akd-watch/config.toml \
  -e AKD_WATCH__DATA_DIRECTORY=/var/lib/akd-watch \
  -v $(pwd)/config.toml:/etc/akd-watch/config.toml:ro \
  -v $(pwd)/data:/var/lib/akd-watch:ro \
  --read-only \
  --tmpfs /tmp:noexec,nosuid,size=100m \
  --security-opt no-new-privileges:true \
  akd-watch/web:latest
```
