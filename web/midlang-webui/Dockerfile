# syntax=docker/dockerfile:1

#   docker build -t midlang-webui web/midlang-webui
#
# The API endpoint is inlined by Vite at build time, so it is a build argument
# rather than a runtime one:
#
#   docker build --build-arg VITE_MIDLANG_REMOTE_URL=https://api.example.com \
#     -t midlang-webui web/midlang-webui

FROM node:24-bookworm-slim AS builder

# Corepack ships with Node and provisions the pnpm version used by CI, so the
# frozen lockfile stays authoritative.
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
RUN corepack enable \
    && corepack prepare pnpm@11 --activate

WORKDIR /app

# Install from the lockfile before copying sources so the dependency layer is
# reused until the dependency set changes.
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY . .

ARG VITE_MIDLANG_REMOTE_URL=http://localhost:4321
ENV VITE_MIDLANG_REMOTE_URL=$VITE_MIDLANG_REMOTE_URL

RUN pnpm build

FROM nginx:1.29-alpine AS runtime

COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80
