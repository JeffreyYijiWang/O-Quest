# syntax=docker/dockerfile:1.7

FROM oven/bun:1 AS base

WORKDIR /app

USER root

COPY package.json bun.lock* ./

RUN --mount=type=secret,id=local_ca,required=false \
	if [ -f /run/secrets/local_ca ]; then \
		apt-get update \
		&& apt-get install -y ca-certificates \
		&& rm -rf /var/lib/apt/lists/* \
		&& cp /run/secrets/local_ca /usr/local/share/ca-certificates/local-ca.crt \
		&& update-ca-certificates \
		&& NODE_EXTRA_CA_CERTS=/usr/local/share/ca-certificates/local-ca.crt \
			bun install --frozen-lockfile --cafile=/usr/local/share/ca-certificates/local-ca.crt \
		&& rm -f /usr/local/share/ca-certificates/local-ca.crt /etc/ssl/certs/local-ca.pem \
		&& update-ca-certificates; \
	else \
		bun install --frozen-lockfile; \
	fi
# Copy source code
COPY . .

# Build the application
RUN bun run build:temp

FROM nginx:alpine AS production

# Copy built assets from build stage
COPY --from=base /app/dist /usr/share/nginx/html

# Copy custom nginx config
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Start nginx
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
